// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tracing::warn;

use crate::stream::{GuardedStream, TelemetryStreamGuard};
use crate::telemetry::{TelemetryEvent, TelemetrySender};
use crate::unix_ms;

// -- Shared application state ----------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub telemetry: TelemetrySender,
    pub http_client: Client,
    pub upstream_base_url: String,
    pub upstream_provider: String,
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

    let upstream_url = format!(
        "{}/v1/chat/completions",
        state.upstream_base_url.trim_end_matches('/')
    );
    let mut upstream_req = state.http_client.post(&upstream_url);

    for (name, value) in headers.iter() {
        if name != axum::http::header::HOST && name != axum::http::header::CONNECTION {
            upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
        }
    }

    let bytes_in_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let bytes_in_clone = bytes_in_counter.clone();
    let body_stream = req_body.into_data_stream().map(move |chunk| {
        if let Ok(ref bytes) = chunk {
            bytes_in_clone.fetch_add(bytes.len(), std::sync::atomic::Ordering::Relaxed);
        }
        chunk
    });

    let reqwest_body = reqwest::Body::wrap_stream(body_stream);
    upstream_req = upstream_req.body(reqwest_body);

    let upstream_res = match upstream_req.send().await {
        Ok(res) => res,
        Err(e) => {
            warn!("upstream request failed: {}", e);
            guard.error_class = Some(e.to_string());
            guard.bytes_in = bytes_in_counter.load(std::sync::atomic::Ordering::Relaxed);
            return Err(axum::http::StatusCode::BAD_GATEWAY);
        }
    };

    guard.bytes_in = bytes_in_counter.load(std::sync::atomic::Ordering::Relaxed);
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
