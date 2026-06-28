// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP routes and handlers for oxideLLM gateway.

use std::borrow::Cow;

use axum::body::Body;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{info, warn};

use crate::config::ResolvedUpstream;
use crate::drain::UpstreamHealthState;
use crate::stream::{GuardedStream, TelemetryStreamGuard};
use crate::telemetry::TelemetrySender;
use crate::unix_ms;

// -- Shared application state ----------------------------------------

/// Shared application state across HTTP routes.
#[derive(Clone)]
pub struct AppState {
    /// Sender channel for telemetry events.
    pub telemetry: TelemetrySender,
    /// HTTP client for proxying requests.
    pub http_client: Client,
    /// Configured upstreams.
    pub upstreams: Vec<ResolvedUpstream>,
    /// Thread-safe status of upstream health check workers.
    pub upstream_health: UpstreamHealthState,
    /// Atomic counter tracking total requests processed.
    pub total_requests: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

// -- Response types ---------------------------------------------------

/// Response payload for the /healthz endpoint.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

/// Response payload for the /readyz endpoint.
#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    telemetry_capacity: usize,
    telemetry_drops: u64,
}

/// Response payload for the /analytics endpoint.
#[derive(Serialize)]
pub struct AnalyticsResponse {
    /// Total requests received by the gateway.
    pub total_requests: u64,
    /// Count of telemetry events dropped due to buffer capacity limit.
    pub dropped_events: u64,
    /// Maximum capacity of the telemetry buffer queue.
    pub telemetry_capacity: usize,
}

#[derive(Deserialize)]
struct ModelRouteView<'a> {
    #[serde(borrow)]
    model: Cow<'a, str>,
}

// -- Router construction ----------------------------------------------

/// Builds the Axum router for the gateway with all routes registered.
pub fn build_router(state: AppState, body_limit_bytes: usize) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/analytics", get(analytics))
        .route("/v1/chat/completions", post(chat_completions))
        .layer(RequestBodyLimitLayer::new(body_limit_bytes))
        .with_state(state)
}

// -- Handlers ---------------------------------------------------------

/// Handler for the /healthz endpoint.
async fn healthz() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: "oxidellm",
    })
}

/// Handler for the /readyz endpoint.
async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    Json(ReadyResponse {
        status: "ready",
        telemetry_capacity: state.telemetry.capacity(),
        telemetry_drops: state.telemetry.dropped(),
    })
}

/// Handler for the /analytics endpoint.
pub async fn analytics(State(state): State<AppState>) -> impl IntoResponse {
    Json(AnalyticsResponse {
        total_requests: state
            .total_requests
            .load(std::sync::atomic::Ordering::Relaxed),
        dropped_events: state.telemetry.dropped(),
        telemetry_capacity: state.telemetry.capacity(),
    })
}

/// Handler for the OpenAI-compatible /v1/chat/completions endpoint.
async fn chat_completions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    req_body: axum::body::Body,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let started_at_ms = unix_ms();
    let started_at_mono = tokio::time::Instant::now();

    state
        .total_requests
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let mut guard = TelemetryStreamGuard {
        request_id,
        started_at_ms,
        started_at_mono,
        telemetry: state.telemetry.clone(),
        ttft_ms: None,
        bytes_in: 0,
        bytes_out: 0,
        status: "error".to_string(),
        error_class: None,
        upstream_id: None,
    };

    let body_bytes = match req_body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            guard.error_class = Some(format!("request_body_error: {}", e));
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };
    guard.bytes_in = body_bytes.len();

    let model = if state.upstreams.len() > 1 {
        extract_model_for_routing(&body_bytes)
    } else {
        None
    };

    let upstream_res =
        match send_with_failover(&state, &headers, body_bytes, model.as_deref()).await {
            Ok((res, upstream_id)) => {
                guard.upstream_id = Some(upstream_id);
                res
            }
            Err((status, error_class)) => {
                guard.error_class = Some(error_class);
                return Err(status);
            }
        };

    let status_code = upstream_res.status();
    if !status_code.is_success() {
        guard.status = "error".to_string();
        guard.error_class = Some(format!("upstream_status_{}", status_code.as_u16()));
    } else {
        guard.status = "disconnected".to_string();
    }

    let mut response_builder = axum::response::Response::builder().status(status_code);

    if let Some(ref uid) = guard.upstream_id {
        response_builder = response_builder.header("x-upstream-id", uid);
    }

    for (name, value) in upstream_res.headers().iter() {
        if crate::proxy::should_forward_response_header(name) {
            response_builder = response_builder.header(name.as_str(), value.as_bytes());
        }
    }

    let res_stream = upstream_res
        .bytes_stream()
        .map(|res| res.map_err(axum::Error::new));

    let guarded_stream = GuardedStream::new(res_stream, guard);
    let axum_body = Body::from_stream(guarded_stream);

    response_builder
        .body(axum_body)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

/// Sends the request to the best upstream with transparent failover support.
async fn send_with_failover(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    body: bytes::Bytes,
    model: Option<&str>,
) -> Result<(reqwest::Response, String), (axum::http::StatusCode, String)> {
    if state.upstreams.is_empty() {
        warn!("no upstreams configured");
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            "no_upstreams_configured".to_string(),
        ));
    }

    if state.upstream_health.healthy_count() == 0 {
        warn!("all configured upstreams are marked unhealthy");
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            "no_healthy_upstreams".to_string(),
        ));
    }

    let ordered_indices = crate::proxy::ordered_upstream_indices(&state.upstreams, model);

    for (index_in_loop, orig_index) in ordered_indices.iter().copied().enumerate() {
        let upstream = &state.upstreams[orig_index];

        if !state.upstream_health.is_healthy(orig_index) {
            warn!(
                upstream_id = %upstream.id,
                provider = %upstream.provider,
                "skipping unhealthy upstream"
            );
            continue;
        }

        let has_next_healthy = ordered_indices
            .iter()
            .skip(index_in_loop + 1)
            .any(|o_idx| state.upstream_health.is_healthy(*o_idx));

        let upstream_url = crate::proxy::chat_completions_url(upstream);
        let mut upstream_req = state.http_client.post(&upstream_url);

        for (name, value) in headers.iter() {
            if crate::proxy::should_forward_header(name) {
                upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
            }
        }

        upstream_req = upstream_req.body(reqwest::Body::from(body.clone()));

        match upstream_req.send().await {
            Ok(res) => {
                let status = res.status();
                if crate::proxy::should_try_next_upstream(status) && has_next_healthy {
                    warn!(
                        upstream_id = %upstream.id,
                        provider = %upstream.provider,
                        status = status.as_u16(),
                        "upstream returned retryable status, trying next configured upstream"
                    );
                    continue;
                }

                if orig_index > 0 {
                    info!(
                        upstream_id = %upstream.id,
                        provider = %upstream.provider,
                        "fallback upstream successfully handled the request"
                    );
                }

                return Ok((res, upstream.id.clone()));
            }
            Err(e) if has_next_healthy => {
                warn!(
                    upstream_id = %upstream.id,
                    provider = %upstream.provider,
                    error = %e,
                    "upstream request failed, trying next configured upstream"
                );
            }
            Err(e) => {
                warn!(
                    upstream_id = %upstream.id,
                    provider = %upstream.provider,
                    error = %e,
                    "upstream request failed with no fallback left"
                );
                return Err((axum::http::StatusCode::BAD_GATEWAY, e.to_string()));
            }
        }
    }

    Err((
        axum::http::StatusCode::BAD_GATEWAY,
        "upstream_failover_exhausted".to_string(),
    ))
}

fn extract_model_for_routing(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<ModelRouteView<'_>>(body)
        .ok()
        .map(|view| view.model.into_owned())
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    };

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use axum::response::{IntoResponse, Response};
    use axum::routing::post;
    use tower::ServiceExt;

    use super::*;
    use crate::telemetry;

    #[tokio::test]
    async fn chat_completions_fails_over_after_retryable_upstream_status() {
        let primary_hits = Arc::new(AtomicUsize::new(0));
        let secondary_hits = Arc::new(AtomicUsize::new(0));

        let (primary_url, primary_handle) =
            spawn_test_upstream(StatusCode::TOO_MANY_REQUESTS, primary_hits.clone()).await;
        let (secondary_url, secondary_handle) =
            spawn_test_upstream(StatusCode::OK, secondary_hits.clone()).await;

        let (telemetry, _rx) = telemetry::channel(64);
        let upstream_health = UpstreamHealthState::new(2);
        let state = AppState {
            telemetry,
            http_client: reqwest::Client::new(),
            upstreams: vec![
                ResolvedUpstream {
                    id: "primary".to_string(),
                    provider: "mock".to_string(),
                    base_url: primary_url,
                    priority: 0,
                    health_path: "/healthz".to_string(),
                },
                ResolvedUpstream {
                    id: "secondary".to_string(),
                    provider: "mock".to_string(),
                    base_url: secondary_url,
                    priority: 10,
                    health_path: "/healthz".to_string(),
                },
            ],
            upstream_health,
            total_requests: Arc::new(AtomicU64::new(0)),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-4","messages":[{"role":"user","content":"hello"}],"stream":true}"#,
            ))
            .expect("request should build");

        let response = build_router(state, 10_485_760)
            .oneshot(request)
            .await
            .expect("gateway response should complete");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should collect");
        assert_eq!(
            body.as_ref(),
            b"data: {\"choices\":[{\"delta\":{\"content\":\"fallback\"},\"index\":0}]}\n\ndata: [DONE]\n\n"
        );
        assert_eq!(primary_hits.load(Ordering::Relaxed), 1);
        assert_eq!(secondary_hits.load(Ordering::Relaxed), 1);

        primary_handle.abort();
        secondary_handle.abort();
    }

    #[tokio::test]
    async fn chat_completions_skips_unhealthy_upstream() {
        let primary_hits = Arc::new(AtomicUsize::new(0));
        let secondary_hits = Arc::new(AtomicUsize::new(0));

        let (primary_url, primary_handle) =
            spawn_test_upstream(StatusCode::OK, primary_hits.clone()).await;
        let (secondary_url, secondary_handle) =
            spawn_test_upstream(StatusCode::OK, secondary_hits.clone()).await;

        let (telemetry, _rx) = telemetry::channel(64);
        let upstream_health = UpstreamHealthState::new(2);
        upstream_health.set_healthy(0, false);

        let state = AppState {
            telemetry,
            http_client: reqwest::Client::new(),
            upstreams: vec![
                ResolvedUpstream {
                    id: "primary".to_string(),
                    provider: "mock".to_string(),
                    base_url: primary_url,
                    priority: 0,
                    health_path: "/healthz".to_string(),
                },
                ResolvedUpstream {
                    id: "secondary".to_string(),
                    provider: "mock".to_string(),
                    base_url: secondary_url,
                    priority: 10,
                    health_path: "/healthz".to_string(),
                },
            ],
            upstream_health,
            total_requests: Arc::new(AtomicU64::new(0)),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-4","messages":[{"role":"user","content":"hello"}],"stream":true}"#,
            ))
            .expect("request should build");

        let response = build_router(state, 10_485_760)
            .oneshot(request)
            .await
            .expect("gateway response should complete");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(primary_hits.load(Ordering::Relaxed), 0);
        assert_eq!(secondary_hits.load(Ordering::Relaxed), 1);

        primary_handle.abort();
        secondary_handle.abort();
    }

    #[tokio::test]
    async fn chat_completions_does_not_failover_on_client_error() {
        let primary_hits = Arc::new(AtomicUsize::new(0));
        let secondary_hits = Arc::new(AtomicUsize::new(0));

        let (primary_url, primary_handle) =
            spawn_test_upstream(StatusCode::BAD_REQUEST, primary_hits.clone()).await;
        let (secondary_url, secondary_handle) =
            spawn_test_upstream(StatusCode::OK, secondary_hits.clone()).await;

        let (telemetry, _rx) = telemetry::channel(64);
        let upstream_health = UpstreamHealthState::new(2);
        let state = AppState {
            telemetry,
            http_client: reqwest::Client::new(),
            upstreams: vec![
                ResolvedUpstream {
                    id: "primary".to_string(),
                    provider: "mock".to_string(),
                    base_url: primary_url,
                    priority: 0,
                    health_path: "/healthz".to_string(),
                },
                ResolvedUpstream {
                    id: "secondary".to_string(),
                    provider: "mock".to_string(),
                    base_url: secondary_url,
                    priority: 10,
                    health_path: "/healthz".to_string(),
                },
            ],
            upstream_health,
            total_requests: Arc::new(AtomicU64::new(0)),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-4","messages":[{"role":"user","content":"hello"}],"stream":true}"#,
            ))
            .expect("request should build");

        let response = build_router(state, 10_485_760)
            .oneshot(request)
            .await
            .expect("gateway response should complete");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        assert_eq!(primary_hits.load(Ordering::Relaxed), 1);
        assert_eq!(secondary_hits.load(Ordering::Relaxed), 0);

        primary_handle.abort();
        secondary_handle.abort();
    }

    #[tokio::test]
    async fn chat_completions_rejects_oversized_body() {
        let (telemetry, _rx) = telemetry::channel(64);
        let upstream_health = UpstreamHealthState::new(0);
        let state = AppState {
            telemetry,
            http_client: reqwest::Client::new(),
            upstreams: vec![],
            upstream_health,
            total_requests: Arc::new(AtomicU64::new(0)),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(vec![0u8; 2000])) // 2KB body
            .expect("request should build");

        let response = build_router(state, 1000) // 1KB limit
            .oneshot(request)
            .await
            .expect("gateway response should complete");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn extract_model_for_routing_reads_only_route_hint() {
        let model = extract_model_for_routing(
            br#"{"model":"LLaMA3","messages":[{"role":"user","content":"large prompt"}],"stream":true}"#,
        );

        assert_eq!(model.as_deref(), Some("LLaMA3"));
    }

    async fn spawn_test_upstream(
        status: StatusCode,
        hits: Arc<AtomicUsize>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = axum::Router::new()
            .route("/v1/chat/completions", post(test_upstream_handler))
            .with_state(TestUpstreamState { status, hits });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test upstream should bind");
        let addr: SocketAddr = listener
            .local_addr()
            .expect("local addr should be available");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test upstream server should run");
        });

        (format!("http://{}", addr), handle)
    }

    #[derive(Clone)]
    struct TestUpstreamState {
        status: StatusCode,
        hits: Arc<AtomicUsize>,
    }

    async fn test_upstream_handler(State(state): State<TestUpstreamState>) -> Response {
        state.hits.fetch_add(1, Ordering::Relaxed);

        if state.status != StatusCode::OK {
            return (state.status, "retry later").into_response();
        }

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(Body::from(
                "data: {\"choices\":[{\"delta\":{\"content\":\"fallback\"},\"index\":0}]}\n\ndata: [DONE]\n\n",
            ))
            .expect("SSE response should build")
    }
}
