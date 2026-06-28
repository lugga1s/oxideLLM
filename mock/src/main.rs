// SPDX-License-Identifier: AGPL-3.0-or-later
//
// High-performance SSE mock server for oxideLLM benchmarks.
// Replaces the Python mock_server.py that saturated under load
// due to GIL contention and `Connection: close`.

use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use tokio::time::sleep;
use tokio_stream::StreamExt;

#[derive(Debug, Parser, Clone)]
#[command(author, version, about = "SSE mock server for oxideLLM benchmarks")]
struct Args {
    /// Host to bind to
    #[arg(long, env = "MOCK_HOST", default_value = "0.0.0.0")]
    host: String,

    /// Port to bind to
    #[arg(long, env = "MOCK_PORT", default_value_t = 9000)]
    port: u16,

    /// Number of SSE chunks to send (excluding [DONE])
    #[arg(long, env = "MOCK_CHUNKS", default_value_t = 2)]
    chunks: usize,

    /// Delay between chunks in microseconds (1000 = 1ms)
    #[arg(long, env = "MOCK_CHUNK_DELAY_US", default_value_t = 1000)]
    chunk_delay_us: u64,
}

#[derive(Clone)]
struct MockState {
    chunks: usize,
    chunk_delay: Duration,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let state = MockState {
        chunks: args.chunks,
        chunk_delay: Duration::from_micros(args.chunk_delay_us),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(healthz))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("invalid bind address");

    eprintln!("mock SSE server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}

async fn healthz() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

async fn chat_completions(State(state): State<MockState>) -> Response {
    // Pre-build all chunks to avoid allocation in the hot loop.
    let words = [
        "hello",
        " world",
        " from",
        " the",
        " mock",
        " server",
        " streaming",
        " response",
    ];

    let mut sse_frames: Vec<String> = Vec::with_capacity(state.chunks + 1);

    for i in 0..state.chunks {
        let word = words[i % words.len()];
        let is_last = i == state.chunks - 1;
        let finish = if is_last {
            r#","finish_reason":"stop""#
        } else {
            ""
        };
        sse_frames.push(format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{word}\"}},\"index\":0{finish}}}]}}\n\n"
        ));
    }
    sse_frames.push("data: [DONE]\n\n".to_string());

    let delay = state.chunk_delay;

    let stream = tokio_stream::iter(sse_frames).then(move |frame| async move {
        if !delay.is_zero() {
            sleep(delay).await;
        }
        Ok::<_, std::convert::Infallible>(frame)
    });

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        // No Connection: close — keep-alive is the HTTP/1.1 default.
        .body(Body::from_stream(stream))
        .unwrap()
}
