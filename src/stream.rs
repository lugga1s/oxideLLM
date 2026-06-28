// SPDX-License-Identifier: AGPL-3.0-or-later

//! Async response stream wrapper with time-to-first-token accumulator.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;

use crate::telemetry::{TelemetryEvent, TelemetrySender};
use crate::unix_ms;

// -- Guard that fires a completion event on drop ----------------------

pub(crate) struct TelemetryStreamGuard {
    pub request_id: String,
    pub started_at_ms: u64,
    pub started_at_mono: tokio::time::Instant,
    pub telemetry: TelemetrySender,
    pub ttft_ms: Option<u64>,
    pub bytes_in: usize,
    pub bytes_out: usize,
    pub status: String,
    pub error_class: Option<String>,
    pub upstream_id: Option<String>,
}

impl Drop for TelemetryStreamGuard {
    fn drop(&mut self) {
        let completed_at_ms = unix_ms();
        let completed = TelemetryEvent::request_completed(
            self.request_id.clone(),
            self.started_at_ms,
            completed_at_ms,
            self.ttft_ms,
            self.bytes_in,
            self.bytes_out,
            self.status.clone(),
            self.error_class.clone(),
            self.upstream_id.clone(),
        );
        self.telemetry.try_record(completed);
    }
}

// -- Stream wrapper that tracks TTFT and byte counts ------------------

pub(crate) struct GuardedStream<S> {
    inner: S,
    _guard: TelemetryStreamGuard,
}

impl<S> GuardedStream<S> {
    pub fn new(inner: S, guard: TelemetryStreamGuard) -> Self {
        Self {
            inner,
            _guard: guard,
        }
    }
}

impl<S, T, E> Stream for GuardedStream<S>
where
    S: Stream<Item = Result<T, E>> + Unpin,
    T: AsRef<[u8]>,
    E: std::fmt::Display,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self._guard.bytes_out += bytes.as_ref().len();
                if self._guard.ttft_ms.is_none() {
                    let elapsed = self._guard.started_at_mono.elapsed().as_millis() as u64;
                    self._guard.ttft_ms = Some(elapsed);
                }
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(e))) => {
                self._guard.status = "error".to_string();
                self._guard.error_class = Some(e.to_string());
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                self._guard.status = "ok".to_string();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
