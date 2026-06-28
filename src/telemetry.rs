// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crossbeam_queue::ArrayQueue;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TelemetryEvent {
    RequestStarted {
        request_id: String,
        started_at_ms: u64,
    },
    RequestCompleted {
        request_id: String,
        started_at_ms: u64,
        completed_at_ms: u64,
        total_latency_ms: u64,
    },
}

impl TelemetryEvent {
    pub fn request_started(request_id: String, started_at_ms: u64) -> Self {
        Self::RequestStarted {
            request_id,
            started_at_ms,
        }
    }

    pub fn request_completed(request_id: String, started_at_ms: u64, completed_at_ms: u64) -> Self {
        Self::RequestCompleted {
            request_id,
            started_at_ms,
            completed_at_ms,
            total_latency_ms: completed_at_ms.saturating_sub(started_at_ms),
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
        assert!(queue.try_record(TelemetryEvent::request_completed("a".into(), 1, 5)));

        let batch = queue.drain_batch(10);

        assert_eq!(batch.len(), 2);
        assert_eq!(queue.len(), 0);
    }
}
