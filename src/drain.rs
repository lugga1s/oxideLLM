// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use crate::telemetry::{TelemetryBatch, TelemetryQueue};
use crate::unix_ms;

/// Background worker that drains the telemetry queue in micro-batches
/// and writes JSONL to disk. Flushes by time (`flush_interval_ms`) or
/// by batch size, whichever comes first. On shutdown, drains all
/// remaining events before returning.
pub async fn telemetry_drain_worker(
    queue: Arc<TelemetryQueue>,
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
    let mut last_flush = tokio::time::Instant::now();
    let tick_duration = std::time::Duration::from_millis(flush_interval_ms.min(100));
    let mut interval = tokio::time::interval(tick_duration);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let is_shutdown = matches!(
            shutdown_rx.try_recv(),
            Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        );

        let remaining = batch_size.saturating_sub(buffer.len());
        if remaining > 0 {
            let mut events = queue.drain_batch(remaining);
            buffer.append(&mut events);
        }

        let now = tokio::time::Instant::now();
        let age_ms = now.duration_since(last_flush).as_millis() as u64;

        if !buffer.is_empty()
            && (buffer.len() >= batch_size || age_ms >= flush_interval_ms || is_shutdown)
        {
            let count = buffer.len();
            let batch = TelemetryBatch {
                schema_version: 1,
                batch_id: uuid::Uuid::new_v4().to_string(),
                created_at_ms: unix_ms(),
                event_count: count,
                events: std::mem::take(&mut buffer),
            };

            if let Ok(line) = serde_json::to_string(&batch) {
                let mut write_data = line.into_bytes();
                write_data.push(b'\n');
                if let Err(e) = file.write_all(&write_data).await {
                    tracing::error!(error = %e, "failed to write telemetry to file");
                } else if let Err(e) = file.flush().await {
                    tracing::error!(error = %e, "failed to flush telemetry file");
                } else {
                    debug!(
                        events = count,
                        drops = queue.dropped(),
                        "telemetry batch written"
                    );
                }
            }
            last_flush = now;
        }

        if is_shutdown {
            loop {
                let events = queue.drain_batch(batch_size);
                if events.is_empty() {
                    break;
                }
                let count = events.len();
                let batch = TelemetryBatch {
                    schema_version: 1,
                    batch_id: uuid::Uuid::new_v4().to_string(),
                    created_at_ms: unix_ms(),
                    event_count: count,
                    events,
                };
                if let Ok(line) = serde_json::to_string(&batch) {
                    let mut write_data = line.into_bytes();
                    write_data.push(b'\n');
                    let _ = file.write_all(&write_data).await;
                }
            }
            let _ = file.flush().await;
            break;
        }

        tokio::select! {
            _ = interval.tick() => {}
            _ = &mut shutdown_rx => {}
        }
    }
}
