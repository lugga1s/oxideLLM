// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderName;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use reqwest::Client;
use serde::Serialize;
use tracing::warn;

use crate::config::ResolvedUpstream;
use crate::drain::UpstreamHealthState;
use crate::stream::{GuardedStream, TelemetryStreamGuard};
use crate::telemetry::{TelemetryEvent, TelemetrySender};
use crate::unix_ms;

// -- Shared application state ----------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub telemetry: TelemetrySender,
    pub http_client: Client,
    pub upstreams: Vec<ResolvedUpstream>,
    pub upstream_health: UpstreamHealthState,
}

// -- Response types ---------------------------------------------------

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    telemetry_capacity: usize,
    telemetry_drops: u64,
}

// -- Router construction ----------------------------------------------

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
}

// -- Handlers ---------------------------------------------------------

async fn healthz() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: "oxidellm",
    })
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    Json(ReadyResponse {
        status: "ready",
        telemetry_capacity: state.telemetry.capacity(),
        telemetry_drops: state.telemetry.dropped(),
    })
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    req_body: axum::body::Body,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let started_at_ms = unix_ms();
    let started_at_mono = tokio::time::Instant::now();

    state.telemetry.try_record(TelemetryEvent::request_started(
        request_id.clone(),
        started_at_ms,
    ));

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
    };

    let body_bytes = match req_body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            guard.error_class = Some(format!("request_body_error: {}", e));
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };
    guard.bytes_in = body_bytes.len();

    let upstream_res = match send_with_failover(&state, &headers, body_bytes).await {
        Ok(res) => res,
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

    for (name, value) in upstream_res.headers().iter() {
        if name != axum::http::header::TRANSFER_ENCODING && name != axum::http::header::CONNECTION {
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

async fn send_with_failover(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Result<reqwest::Response, (axum::http::StatusCode, String)> {
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

    for (index, upstream) in state.upstreams.iter().enumerate() {
        if !state.upstream_health.is_healthy(index) {
            warn!(
                upstream_id = %upstream.id,
                provider = %upstream.provider,
                "skipping unhealthy upstream"
            );
            continue;
        }

        let has_next_healthy = has_next_healthy_upstream(state, index);
        let upstream_url = chat_completions_url(upstream);
        let mut upstream_req = state.http_client.post(&upstream_url);

        for (name, value) in headers.iter() {
            if should_forward_header(name) {
                upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
            }
        }

        upstream_req = upstream_req.body(reqwest::Body::from(body.clone()));

        match upstream_req.send().await {
            Ok(res) => {
                let status = res.status();
                if should_try_next_upstream(status) && has_next_healthy {
                    warn!(
                        upstream_id = %upstream.id,
                        provider = %upstream.provider,
                        status = status.as_u16(),
                        "upstream returned retryable status, trying next configured upstream"
                    );
                    continue;
                }

                return Ok(res);
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

fn has_next_healthy_upstream(state: &AppState, current_index: usize) -> bool {
    state
        .upstreams
        .iter()
        .enumerate()
        .skip(current_index + 1)
        .any(|(index, _)| state.upstream_health.is_healthy(index))
}

fn chat_completions_url(upstream: &ResolvedUpstream) -> String {
    format!(
        "{}/v1/chat/completions",
        upstream.base_url.trim_end_matches('/')
    )
}

fn should_forward_header(name: &HeaderName) -> bool {
    name != axum::http::header::HOST
        && name != axum::http::header::CONNECTION
        && name != axum::http::header::TRANSFER_ENCODING
}

fn should_try_next_upstream(status: reqwest::StatusCode) -> bool {
    status.as_u16() >= 400
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
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
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"stream":true}"#))
            .expect("request should build");

        let response = build_router(state)
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
        };

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"stream":true}"#))
            .expect("request should build");

        let response = build_router(state)
            .oneshot(request)
            .await
            .expect("gateway response should complete");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(primary_hits.load(Ordering::Relaxed), 0);
        assert_eq!(secondary_hits.load(Ordering::Relaxed), 1);

        primary_handle.abort();
        secondary_handle.abort();
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
