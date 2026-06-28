// SPDX-License-Identifier: AGPL-3.0-or-later

mod telemetry;

use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::SystemTime};

use axum::{
    Json, Router,
    extract::State,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use clap::Parser;
use serde::Serialize;
use tokio_stream::{self as stream, Stream};
use tracing::{info, warn};

use crate::telemetry::{TelemetryEvent, TelemetryQueue};

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

    let state = AppState {
        telemetry: Arc::new(TelemetryQueue::new(args.telemetry_capacity)),
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
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let started_at_ms = unix_ms();

    let accepted = state.telemetry.try_record(TelemetryEvent::request_started(
        request_id.clone(),
        started_at_ms,
    ));

    if !accepted {
        warn!("telemetry queue full while recording request start");
    }

    let completed = TelemetryEvent::request_completed(request_id, started_at_ms, unix_ms());
    let _ = state.telemetry.try_record(completed);

    let events = vec![
        Ok(Event::default().data(
            r#"{"id":"mock","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}"#,
        )),
        Ok(Event::default().data(
            r#"{"id":"mock","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":"stop"}]}"#,
        )),
        Ok(Event::default().data("[DONE]")),
    ];

    let stream = stream::iter(events);

    Sse::new(stream).keep_alive(KeepAlive::default())
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
