// SPDX-License-Identifier: AGPL-3.0-or-later

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::debug;

use crate::telemetry::{TelemetryBatch, TelemetryEvent, TelemetrySender};
use crate::unix_ms;

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

    loop {
        tokio::select! {
            event_opt = receiver.recv() => {
                match event_opt {
                    Some(event) => {
                        buffer.push(event);
                        if buffer.len() >= batch_size {
                            flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(flush_interval_ms)) => {
                if !buffer.is_empty() {
                    flush_buffer(&mut buffer, &mut file, sender.dropped()).await;
                }
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
        schema_version: 1,
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
