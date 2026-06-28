// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crossbeam_queue::ArrayQueue;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
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
        }
    }
}

#[derive(Debug)]
pub struct TelemetryQueue {
    queue: Arc<ArrayQueue<TelemetryEvent>>,
    dropped: AtomicU64,
}

impl TelemetryQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "telemetry capacity must be greater than zero");
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            dropped: AtomicU64::new(0),
        }
    }

    pub fn try_record(&self, event: TelemetryEvent) -> bool {
        if self.queue.push(event).is_ok() {
            true
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    pub fn drain_batch(&self, max: usize) -> Vec<TelemetryEvent> {
        let mut batch = Vec::with_capacity(max.min(self.queue.len()));

        for _ in 0..max {
            if let Some(event) = self.queue.pop() {
                batch.push(event);
            } else {
                break;
            }
        }

        batch
    }

    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::{TelemetryEvent, TelemetryQueue};

    #[test]
    fn drops_when_queue_is_full() {
        let queue = TelemetryQueue::new(1);

        assert!(queue.try_record(TelemetryEvent::request_started("a".into(), 1)));
        assert!(!queue.try_record(TelemetryEvent::request_started("b".into(), 2)));
        assert_eq!(queue.dropped(), 1);
    }

    #[test]
    fn drains_batch() {
        let queue = TelemetryQueue::new(4);

        assert!(queue.try_record(TelemetryEvent::request_started("a".into(), 1)));
        assert!(queue.try_record(TelemetryEvent::request_completed(
            "a".into(),
            1,
            5,
            Some(2),
            100,
            200,
            "ok".to_string(),
            None
        )));

        let batch = queue.drain_batch(10);

        assert_eq!(batch.len(), 2);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn telemetry_queue_overflow() {
        let capacity = 5;
        let queue = TelemetryQueue::new(capacity);

        for i in 0..capacity {
            assert!(queue.try_record(TelemetryEvent::request_started(
                format!("req_{}", i),
                i as u64
            )));
        }

        // Overflows on next
        assert!(!queue.try_record(TelemetryEvent::request_started("overflow".into(), 100)));
        assert_eq!(queue.dropped(), 1);
        assert_eq!(queue.len(), capacity);

        let batch = queue.drain_batch(capacity + 2);
        assert_eq!(batch.len(), capacity);
        assert_eq!(queue.len(), 0);
    }
}
