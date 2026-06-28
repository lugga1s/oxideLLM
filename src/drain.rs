// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use reqwest::Client;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::ResolvedUpstream;
use crate::telemetry::{TelemetryBatch, TelemetryEvent, TelemetrySender};
use crate::unix_ms;

// -- Upstream health worker ------------------------------------------

#[derive(Clone, Debug)]
pub struct UpstreamHealthState {
    states: Arc<Vec<AtomicBool>>,
}

impl UpstreamHealthState {
    pub fn new(upstream_count: usize) -> Self {
        Self {
            states: Arc::new((0..upstream_count).map(|_| AtomicBool::new(true)).collect()),
        }
    }

    pub fn is_healthy(&self, index: usize) -> bool {
        self.states
            .get(index)
            .map(|state| state.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    pub fn set_healthy(&self, index: usize, healthy: bool) -> Option<bool> {
        self.states
            .get(index)
            .map(|state| state.swap(healthy, Ordering::Relaxed))
    }

    pub fn healthy_count(&self) -> usize {
        self.states
            .iter()
            .filter(|state| state.load(Ordering::Relaxed))
            .count()
    }
}

pub async fn upstream_health_worker(
    http_client: Client,
    upstreams: Vec<ResolvedUpstream>,
    health: UpstreamHealthState,
    interval_ms: u64,
    timeout_ms: u64,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    if upstreams.is_empty() {
        return;
    }

    let mut interval = tokio::time::interval(Duration::from_millis(interval_ms.max(1)));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                check_upstreams_once(&http_client, &upstreams, &health, timeout_ms).await;
            }
            _ = &mut shutdown_rx => {
                break;
            }
        }
    }
}

async fn check_upstreams_once(
    http_client: &Client,
    upstreams: &[ResolvedUpstream],
    health: &UpstreamHealthState,
    timeout_ms: u64,
) {
    let futures: Vec<_> = upstreams
        .iter()
        .map(|upstream| check_upstream(http_client, upstream, timeout_ms))
        .collect();

    let results = futures_util::future::join_all(futures).await;

    for (index, (upstream, healthy)) in upstreams.iter().zip(results).enumerate() {
        let Some(previous) = health.set_healthy(index, healthy) else {
            continue;
        };

        if previous == healthy {
            continue;
        }

        if healthy {
            info!(
                upstream_id = %upstream.id,
                provider = %upstream.provider,
                "upstream health restored"
            );
        } else {
            warn!(
                upstream_id = %upstream.id,
                provider = %upstream.provider,
                "upstream marked unhealthy"
            );
        }
    }
}

async fn check_upstream(
    http_client: &Client,
    upstream: &ResolvedUpstream,
    timeout_ms: u64,
) -> bool {
    let health_url = upstream_health_url(upstream);
    let timeout = Duration::from_millis(timeout_ms.max(1));

    match http_client.get(&health_url).timeout(timeout).send().await {
        Ok(response) => {
            let status = response.status();
            let _ = response.bytes().await;
            status.is_success()
        }
        Err(e) => {
            debug!(
                upstream_id = %upstream.id,
                provider = %upstream.provider,
                error = %e,
                "upstream health check failed"
            );
            false
        }
    }
}

fn upstream_health_url(upstream: &ResolvedUpstream) -> String {
    format!(
        "{}{}",
        upstream.base_url.trim_end_matches('/'),
        upstream.health_path
    )
}

// -- Telemetry drain worker ------------------------------------------

/// Background worker that drains the telemetry queue in micro-batches
/// and writes JSONL to disk. Flushes by time (`flush_interval_ms`) or
/// by batch size, whichever comes first. On shutdown, drains all
/// remaining events before returning.
pub async fn telemetry_drain_worker(
    mut receiver: mpsc::Receiver<TelemetryEvent>,
    sender: TelemetrySender,
    log_path: String,
    batch_size: usize,
    flush_interval_ms: u64,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(path = %log_path, error = %e, "failed to open telemetry log file");
            return;
        }
    };

    let mut buffer = Vec::with_capacity(batch_size);

    let flush_duration = std::time::Duration::from_millis(flush_interval_ms);
    let flush_sleep = tokio::time::sleep(flush_duration);
    tokio::pin!(flush_sleep);

    loop {
        tokio::select! {
            event_opt = receiver.recv() => {
                match event_opt {
                    Some(event) => {
                        buffer.push(event);
                        if buffer.len() >= batch_size {
                            flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
                            flush_sleep.as_mut().reset(tokio::time::Instant::now() + flush_duration);
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            _ = &mut flush_sleep => {
                if !buffer.is_empty() {
                    flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
                }
                flush_sleep.as_mut().reset(tokio::time::Instant::now() + flush_duration);
            }
            _ = &mut shutdown_rx => {
                break;
            }
        }
    }

    while let Ok(event) = receiver.try_recv() {
        buffer.push(event);
        if buffer.len() >= batch_size {
            flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
        }
    }

    if !buffer.is_empty() {
        flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
    }
}

async fn flush_buffer(buffer: &mut Vec<TelemetryEvent>, file: &mut tokio::fs::File, dropped: u64) {
    let count = buffer.len();
    let batch = TelemetryBatch {
        schema_version: 2,
        batch_id: uuid::Uuid::new_v4().to_string(),
        created_at_ms: unix_ms(),
        event_count: count,
        events: std::mem::take(buffer),
    };

    if let Ok(line) = serde_json::to_string(&batch) {
        let mut write_data = line.into_bytes();
        write_data.push(b'\n');
        if let Err(e) = file.write_all(&write_data).await {
            tracing::error!(error = %e, "failed to write telemetry to file");
        } else if let Err(e) = file.flush().await {
            tracing::error!(error = %e, "failed to flush telemetry file");
        } else {
            debug!(events = count, drops = dropped, "telemetry batch written");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;

    use super::*;

    #[test]
    fn upstream_health_state_starts_healthy_and_can_be_updated() {
        let health = UpstreamHealthState::new(2);

        assert!(health.is_healthy(0));
        assert!(health.is_healthy(1));
        assert_eq!(health.healthy_count(), 2);

        assert_eq!(health.set_healthy(1, false), Some(true));
        assert!(health.is_healthy(0));
        assert!(!health.is_healthy(1));
        assert_eq!(health.healthy_count(), 1);
        assert!(!health.is_healthy(2));
    }

    #[tokio::test]
    async fn upstream_health_worker_marks_unhealthy_upstreams() {
        let (healthy_url, healthy_handle) = spawn_health_upstream(StatusCode::OK).await;
        let (unhealthy_url, unhealthy_handle) =
            spawn_health_upstream(StatusCode::SERVICE_UNAVAILABLE).await;

        let health = UpstreamHealthState::new(2);
        let upstreams = vec![
            ResolvedUpstream {
                id: "healthy".to_string(),
                provider: "mock".to_string(),
                base_url: healthy_url,
                priority: 0,
                health_path: "/healthz".to_string(),
            },
            ResolvedUpstream {
                id: "unhealthy".to_string(),
                provider: "mock".to_string(),
                base_url: unhealthy_url,
                priority: 10,
                health_path: "/healthz".to_string(),
            },
        ];

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let worker_health = health.clone();
        let worker_handle = tokio::spawn(async move {
            upstream_health_worker(
                reqwest::Client::new(),
                upstreams,
                worker_health,
                10,
                250,
                shutdown_rx,
            )
            .await;
        });

        for _ in 0..50 {
            if health.is_healthy(0) && !health.is_healthy(1) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(health.is_healthy(0));
        assert!(!health.is_healthy(1));

        shutdown_tx.send(()).unwrap();
        worker_handle.await.unwrap();
        healthy_handle.abort();
        unhealthy_handle.abort();
    }

    async fn spawn_health_upstream(status: StatusCode) -> (String, tokio::task::JoinHandle<()>) {
        let app = axum::Router::new()
            .route("/healthz", get(health_handler))
            .with_state(status);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("health test upstream should bind");
        let addr: SocketAddr = listener
            .local_addr()
            .expect("health test upstream should expose local addr");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("health test upstream should run");
        });

        (format!("http://{}", addr), handle)
    }

    async fn health_handler(State(status): State<StatusCode>) -> impl IntoResponse {
        status
    }

    #[tokio::test]
    async fn telemetry_drain_flushes_partial_buffer_by_time() {
        let (tx, rx) = crate::telemetry::channel(100);
        let log_path = format!("test_telemetry_{}.jsonl", uuid::Uuid::new_v4());

        tx.try_record(TelemetryEvent::request_started("req_timeout".into(), 100));

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let log_path_clone = log_path.clone();

        let worker_handle = tokio::spawn(async move {
            crate::drain::telemetry_drain_worker(rx, tx, log_path_clone, 1000, 50, shutdown_rx)
                .await;
        });

        tokio::time::sleep(Duration::from_millis(200)).await;

        shutdown_tx.send(()).unwrap();
        worker_handle.await.unwrap();

        let contents = std::fs::read_to_string(&log_path).expect("Failed to read test log file");
        let _ = std::fs::remove_file(&log_path);

        let lines: Vec<&str> = contents.trim().split('\n').collect();
        assert_eq!(lines.len(), 1);

        let batch: TelemetryBatch = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(batch.event_count, 1);
        assert_eq!(batch.events.len(), 1);
    }
}
