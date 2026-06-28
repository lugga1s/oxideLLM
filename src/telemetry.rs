// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    RequestStarted {
        request_id: String,
        started_at_ms: u64,
    },
    RequestCompleted {
        request_id: String,
        started_at_ms: u64,
        completed_at_ms: u64,
        ttft_ms: Option<u64>,
        total_latency_ms: u64,
        bytes_in: usize,
        bytes_out: usize,
        status: String,
        error_class: Option<String>,
        upstream_id: Option<String>,
    },
}

impl TelemetryEvent {
    pub fn request_started(request_id: String, started_at_ms: u64) -> Self {
        Self::RequestStarted {
            request_id,
            started_at_ms,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn request_completed(
        request_id: String,
        started_at_ms: u64,
        completed_at_ms: u64,
        ttft_ms: Option<u64>,
        bytes_in: usize,
        bytes_out: usize,
        status: String,
        error_class: Option<String>,
        upstream_id: Option<String>,
    ) -> Self {
        Self::RequestCompleted {
            request_id,
            started_at_ms,
            completed_at_ms,
            ttft_ms,
            total_latency_ms: completed_at_ms.saturating_sub(started_at_ms),
            bytes_in,
            bytes_out,
            status,
            error_class,
            upstream_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryBatch {
    pub schema_version: u32,
    pub batch_id: String,
    pub created_at_ms: u64,
    pub event_count: usize,
    pub events: Vec<TelemetryEvent>,
}

#[derive(Debug, Clone)]
pub struct TelemetrySender {
    sender: tokio::sync::mpsc::Sender<TelemetryEvent>,
    dropped: Arc<AtomicU64>,
    capacity: usize,
}

impl TelemetrySender {
    pub fn try_record(&self, event: TelemetryEvent) -> bool {
        match self.sender.try_send(event) {
            Ok(_) => true,
            Err(_) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

pub fn channel(capacity: usize) -> (TelemetrySender, tokio::sync::mpsc::Receiver<TelemetryEvent>) {
    assert!(capacity > 0, "telemetry capacity must be greater than zero");
    let (tx, rx) = tokio::sync::mpsc::channel(capacity);
    let sender = TelemetrySender {
        sender: tx,
        dropped: Arc::new(AtomicU64::new(0)),
        capacity,
    };
    (sender, rx)
}

#[cfg(test)]
mod tests {
    use super::{TelemetryEvent, channel};

    #[test]
    fn drops_when_queue_is_full() {
        let (sender, mut receiver) = channel(1);

        assert!(sender.try_record(TelemetryEvent::request_started("a".into(), 1)));
        assert!(!sender.try_record(TelemetryEvent::request_started("b".into(), 2)));
        assert_eq!(sender.dropped(), 1);

        assert!(receiver.try_recv().is_ok());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn telemetry_queue_overflow() {
        let capacity = 5;
        let (sender, mut receiver) = channel(capacity);

        for i in 0..capacity {
            assert!(sender.try_record(TelemetryEvent::request_started(
                format!("req_{}", i),
                i as u64
            )));
        }

        // Overflows on next
        assert!(!sender.try_record(TelemetryEvent::request_started("overflow".into(), 100)));
        assert_eq!(sender.dropped(), 1);

        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, capacity);
    }
}
