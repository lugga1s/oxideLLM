// SPDX-License-Identifier: AGPL-3.0-or-later

mod telemetry;

use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use clap::Parser;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{info, warn};

use crate::telemetry::{TelemetryEvent, TelemetryQueue};

struct TelemetryStreamGuard {
    request_id: String,
    started_at_ms: u64,
    telemetry: Arc<TelemetryQueue>,
}

impl Drop for TelemetryStreamGuard {
    fn drop(&mut self) {
        let completed = TelemetryEvent::request_completed(
            self.request_id.clone(),
            self.started_at_ms,
            unix_ms(),
        );
        let _ = self.telemetry.try_record(completed);
    }
}

struct GuardedStream<S> {
    inner: S,
    _guard: TelemetryStreamGuard,
}

impl<S> GuardedStream<S> {
    fn new(inner: S, guard: TelemetryStreamGuard) -> Self {
        Self {
            inner,
            _guard: guard,
        }
    }
}

impl<S> futures_util::Stream for GuardedStream<S>
where
    S: futures_util::Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "LLMK_HOST", default_value = "127.0.0.1")]
    host: String,

    #[arg(long, env = "LLMK_PORT", default_value_t = 8080)]
    port: u16,

    #[arg(long, env = "LLMK_TELEMETRY_CAPACITY", default_value_t = 65536)]
    telemetry_capacity: usize,
}

#[derive(Clone)]
struct AppState {
    telemetry: Arc<TelemetryQueue>,
    http_client: Client,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    telemetry_capacity: usize,
    telemetry_len: usize,
    telemetry_drops: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxidellm=info,tower_http=info".into()),
        )
        .init();

    let args = Args::parse();
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(100)
        .build()?;

    let state = AppState {
        telemetry: Arc::new(TelemetryQueue::new(args.telemetry_capacity)),
        http_client,
    };

    let telemetry_worker = state.telemetry.clone();
    tokio::spawn(async move {
        telemetry_drain_worker(telemetry_worker).await;
    });

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "gateway listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

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
        telemetry_len: state.telemetry.len(),
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

    let accepted = state.telemetry.try_record(TelemetryEvent::request_started(
        request_id.clone(),
        started_at_ms,
    ));

    if !accepted {
        warn!("telemetry queue full while recording request start");
    }

    let upstream_url = "http://localhost:9000/v1/chat/completions";
    let mut upstream_req = state.http_client.post(upstream_url);

    for (name, value) in headers.iter() {
        if name != axum::http::header::HOST && name != axum::http::header::CONNECTION {
            upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
        }
    }

    let body_stream = req_body.into_data_stream();
    let reqwest_body = reqwest::Body::wrap_stream(body_stream);
    upstream_req = upstream_req.body(reqwest_body);

    let upstream_res = upstream_req.send().await.map_err(|e| {
        warn!("upstream request failed: {}", e);
        axum::http::StatusCode::BAD_GATEWAY
    })?;

    let mut response_builder = axum::response::Response::builder().status(upstream_res.status());

    for (name, value) in upstream_res.headers().iter() {
        if name != axum::http::header::TRANSFER_ENCODING && name != axum::http::header::CONNECTION {
            response_builder = response_builder.header(name.as_str(), value.as_bytes());
        }
    }

    let res_stream = upstream_res
        .bytes_stream()
        .map(|res| res.map_err(axum::Error::new));

    let guard = TelemetryStreamGuard {
        request_id,
        started_at_ms,
        telemetry: state.telemetry.clone(),
    };

    let guarded_stream = GuardedStream::new(res_stream, guard);
    let axum_body = Body::from_stream(guarded_stream);

    response_builder
        .body(axum_body)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

async fn telemetry_drain_worker(queue: Arc<TelemetryQueue>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

    loop {
        interval.tick().await;
        let drained = queue.drain_batch(1000);
        if !drained.is_empty() {
            info!(
                events = drained.len(),
                drops = queue.dropped(),
                "telemetry batch drained"
            );
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
