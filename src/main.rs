// SPDX-License-Identifier: AGPL-3.0-or-later

mod config;
mod drain;
mod routes;
mod stream;
mod telemetry;

use std::net::SocketAddr;
use std::time::SystemTime;

use tracing::info;

use crate::config::load_config;
use crate::drain::telemetry_drain_worker;
use crate::routes::{AppState, build_router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxidellm=info,tower_http=info".into()),
        )
        .init();

    let cfg = load_config()?;

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(1000)
        .build()?;

    let (tx, rx) = telemetry::channel(cfg.telemetry_capacity);

    let state = AppState {
        telemetry: tx.clone(),
        http_client,
        upstream_base_url: cfg.upstream_base_url,
        upstream_provider: cfg.upstream_provider,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let worker_handle = tokio::spawn(async move {
        telemetry_drain_worker(
            rx,
            tx,
            cfg.telemetry_log_path,
            cfg.telemetry_batch_size,
            cfg.telemetry_flush_interval_ms,
            shutdown_rx,
        )
        .await;
    });

    let app = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        %addr,
        upstream_url = %state.upstream_base_url,
        provider = %state.upstream_provider,
        "gateway listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("gateway shutting down, draining remaining telemetry events");
    let _ = shutdown_tx.send(());
    let _ = worker_handle.await;
    info!("telemetry flush complete");

    Ok(())
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

pub(crate) fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    use crate::telemetry::{TelemetryBatch, TelemetryEvent};

    #[tokio::test]
    async fn test_telemetry_drain_worker_batching_and_shutdown() {
        let (tx, rx) = crate::telemetry::channel(100);
        let log_path = format!("test_telemetry_{}.jsonl", uuid::Uuid::new_v4());

        // Push 3 events
        tx.try_record(TelemetryEvent::request_started("req1".into(), 100));
        tx.try_record(TelemetryEvent::request_started("req2".into(), 200));
        tx.try_record(TelemetryEvent::request_started("req3".into(), 300));

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let log_path_clone = log_path.clone();

        let worker_handle = tokio::spawn(async move {
            telemetry_drain_worker(rx, tx, log_path_clone, 2, 50, shutdown_rx).await;
        });

        // Give it some time to process the first batch (size 2) and flush by time
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Send graceful shutdown
        shutdown_tx.send(()).unwrap();
        worker_handle.await.unwrap();

        // Read file contents
        let contents = fs::read_to_string(&log_path).expect("Failed to read test log file");

        // Cleanup test file
        let _ = fs::remove_file(&log_path);

        let lines: Vec<&str> = contents.trim().split('\n').collect();
        assert!(lines.len() >= 2, "Expected at least 2 batches written");

        // First batch should have 2 events
        let batch1: TelemetryBatch = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(batch1.event_count, 2);
        assert_eq!(batch1.events.len(), 2);

        // Second batch should have 1 event
        let batch2: TelemetryBatch = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(batch2.event_count, 1);
        assert_eq!(batch2.events.len(), 1);
    }
}
