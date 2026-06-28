// SPDX-License-Identifier: AGPL-3.0-or-later

//! oxideLLM - High-performance LLM gateway/proxy library.

#![warn(missing_docs)]

pub mod config;
pub mod drain;
pub mod models;
pub mod proxy;
pub mod routes;
pub mod sse;
pub mod stream;
pub mod telemetry;

/// Helper to get current Unix epoch time in milliseconds.
pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
