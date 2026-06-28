// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end integration tests for oxideLLM gateway.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

use oxidellm::config::ResolvedUpstream;
use oxidellm::drain::UpstreamHealthState;
use oxidellm::routes::{AppState, build_router};
use oxidellm::telemetry;

#[tokio::test]
async fn test_full_request_lifecycle() {
    let upstream_hits = Arc::new(AtomicUsize::new(0));

    // Spawn a mock upstream
    let (upstream_url, upstream_handle) =
        spawn_mock_upstream(StatusCode::OK, upstream_hits.clone()).await;

    let (telemetry_tx, _rx) = telemetry::channel(64);
    let upstream_health = UpstreamHealthState::new(1);

    let state = AppState {
        telemetry: telemetry_tx,
        http_client: reqwest::Client::new(),
        upstreams: vec![ResolvedUpstream {
            id: "test-upstream".to_string(),
            provider: "mock".to_string(),
            base_url: upstream_url,
            priority: 0,
            health_path: "/healthz".to_string(),
        }],
        upstream_health,
        total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    let router = build_router(state, 10_485_760);

    // Send chat completion request
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"model":"gpt-4","messages":[{"role":"user","content":"hello"}],"stream":true}"#,
        ))
        .expect("request should build");

    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    assert!(body.as_ref().starts_with(b"data:"));
    assert_eq!(upstream_hits.load(Ordering::Relaxed), 1);

    upstream_handle.abort();
}

#[tokio::test]
async fn test_health_endpoint_returns_status() {
    let (telemetry_tx, _rx) = telemetry::channel(64);
    let upstream_health = UpstreamHealthState::new(0);

    let state = AppState {
        telemetry: telemetry_tx,
        http_client: reqwest::Client::new(),
        upstreams: vec![],
        upstream_health,
        total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    let router = build_router(state, 10_485_760);

    let request = Request::builder()
        .method("GET")
        .uri("/healthz")
        .body(Body::empty())
        .expect("request should build");

    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("should parse JSON");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "oxidellm");
}

#[tokio::test]
async fn test_analytics_endpoint() {
    let (telemetry_tx, _rx) = telemetry::channel(64);
    let upstream_health = UpstreamHealthState::new(0);
    let total_requests = Arc::new(std::sync::atomic::AtomicU64::new(5));

    let state = AppState {
        telemetry: telemetry_tx,
        http_client: reqwest::Client::new(),
        upstreams: vec![],
        upstream_health,
        total_requests,
    };

    let router = build_router(state, 10_485_760);

    let request = Request::builder()
        .method("GET")
        .uri("/analytics")
        .body(Body::empty())
        .expect("request should build");

    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("should parse JSON");
    assert_eq!(json["total_requests"], 5);
    assert_eq!(json["telemetry_capacity"], 64);
}

async fn spawn_mock_upstream(
    status: StatusCode,
    hits: Arc<AtomicUsize>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = axum::Router::new()
        .route("/v1/chat/completions", post(move || {
            let hits = hits.clone();
            async move {
                hits.fetch_add(1, Ordering::Relaxed);
                if status != StatusCode::OK {
                    return (status, "error").into_response();
                }
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .body(Body::from("data: {\"choices\":[{\"delta\":{\"content\":\"hello\"},\"index\":0}]}\n\ndata: [DONE]\n\n"))
                    .expect("response should build")
            }
        }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind");
    let addr: SocketAddr = listener.local_addr().expect("should get addr");

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("should serve");
    });

    (format!("http://{}", addr), handle)
}
