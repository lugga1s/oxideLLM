<div align="center">

<pre>
  ___   __  __ _     _       
 / _ \  \ \/ /(_) __| | ___  
| (_) |  >  <  | |/ _` |/ _ \ 
 \___/  /_/\_\_|_|\__,_|\___| 
</pre>

**High-performance LLM gateway that keeps telemetry off the critical path.**

*Single binary. Zero GC. Async telemetry. Built in Rust.*

[![CI](https://github.com/lugga1s/oxideLLM/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/lugga1s/oxideLLM/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.96+-f74c00.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.9.0--alpha-orange.svg?style=flat-square)](Cargo.toml)

[Quick Start](#quick-start) | [Benchmarks](#performance) | [Architecture](#architecture) | [Configuration](#configuration) | [Contributing](CONTRIBUTING.md)

</div>

---

## The Problem

Traditional LLM gateways couple **proxy**, **tracing**, **logging**, and **database writes** in the same synchronous request path. Under high concurrency, this turns the gateway into a serializing bottleneck:

| Path | Throughput | Efficiency | Degradation |
|---|---:|---:|---:|
| Direct to inference engine (vLLM) | ~16.0 req/s | 100% | - |
| Traditional gateway (4 workers + Postgres + Redis) | ~8.8 req/s | 55% | **-45%** |
| Traditional gateway (1 worker + Postgres + Redis) | ~3.9 req/s | 24% | **-75.6%** |

> Source: internal load tests with 500 concurrent requests against vLLM. Documented in [bottlenecks.md](.context/bottlenecks.md).

**oxideLLM** solves this by rigidly separating the data plane from telemetry: the task that owns the client socket **never waits** for disk I/O, log flushes, or database writes.

---

## Key Highlights

- **OpenAI-compatible API** - integrate once, route to any supported provider.
- **Zero-copy SSE forwarding** - chunks are forwarded as raw byte streams, no per-token JSON deserialization.
- **Async telemetry off the critical path** - events are published to a bounded MPSC queue in microseconds; a background worker micro-batches and flushes to disk.
- **Multi-upstream failover** - if the primary provider fails (429/502/503/504 or network error), the gateway transparently retries on the next healthy upstream.
- **Active health checking** - a background worker periodically pings each upstream and removes unhealthy ones from the routing table.
- **Single binary, no runtime dependencies** - compiled Rust, no GC pauses, no interpreter overhead, no container runtime required.

---

## Performance

Benchmarked with [k6](https://k6.io) under **1000 virtual users** for **30 seconds** against a Rust SSE mock server on localhost.

| Metric | Direct (baseline) | via oxideLLM | Overhead |
|---|---:|---:|---:|
| **Throughput** | 20,919 req/s | 18,118 req/s | ~13% |
| **P95 latency** | 56.51 ms | 74.51 ms | +18 ms |
| **P99 latency** | 70.16 ms | 92.47 ms | +22 ms |
| **HTTP errors** | 0.00% | 0.00% | - |

<details>
<summary><b>Environment & Reproducibility</b></summary>

```text
OS:     Linux 6.18 (WSL2 Ubuntu, ext4 filesystem)
CPU:    AMD Ryzen 5 5600G - 6 cores / 12 threads
Memory: 7.4 GiB
Rust:   rustc 1.96.0
k6:     v2.0.0 linux/amd64
Commit: 032d9285
```

**Reproduce it yourself:**

```bash
# Build release binaries
cargo build --release
cargo build --manifest-path mock/Cargo.toml --release

# Start mock server
./target/release/oxidellm-mock --host 127.0.0.1 --port 9000 &

# Start gateway
./target/release/oxidellm --host 127.0.0.1 --port 8080 \
  --upstream-base-url http://127.0.0.1:9000 &

# Baseline: direct to mock
k6 run -e VUS=1000 -e DURATION=30s \
  -e TARGET_URL=http://127.0.0.1:9000/v1/chat/completions \
  k6/proxy-vs-direct.js

# Gateway: through oxideLLM
k6 run -e VUS=1000 -e DURATION=30s \
  -e TARGET_URL=http://127.0.0.1:8080/v1/chat/completions \
  k6/proxy-vs-direct.js
```

Full methodology: [benchmarks/](benchmarks/) | Validation contract: [validation-gates.md](docs/validation-gates.md)

> **Note:** WSL2 loopback networking adds ~10-15% artificial overhead due to Hyper-V bridge packet duplication. In an isolated data-plane test (telemetry directed to `/dev/null`), raw proxy overhead measured **~1%**. Native Linux and distributed environments are expected to show lower degradation. See [ADR-0007](.context/decisions/) for details.

</details>

---

## Architecture

oxideLLM separates three processing planes so that analytics never block the client response:

```mermaid
graph LR
    Client["Client SDK / curl"]
    Gateway["oxideLLM Gateway (Axum + Tokio)"]
    Primary["Primary Upstream (Ollama, OpenAI, Groq)"]
    Fallback["Fallback Upstream (vLLM, mock, etc.)"]
    Ring["Telemetry Ring Buffer (bounded MPSC)"]
    Worker["Background Worker (micro-batching)"]
    Sink["Sink (JSONL / Parquet)"]

    Client -->|"POST /v1/chat/completions"| Gateway
    Gateway -->|"priority routing"| Primary
    Gateway -.->|"failover on 429/502/503/504"| Fallback
    Gateway -->|"non-blocking try_send()"| Ring
    Ring -->|"flush by size or time"| Worker
    Worker -->|"batch write"| Sink

    style Gateway fill:#f74c00,color:#fff,stroke:#333
    style Ring fill:#2d6a4f,color:#fff,stroke:#333
    style Worker fill:#2d6a4f,color:#fff,stroke:#333
```

### Design Principles

| Principle | Implementation |
|---|---|
| **Critical path is minimal** | Accept -> route -> forward -> stream -> emit telemetry event (non-blocking) |
| **Zero-copy by default** | SSE chunks forwarded as `Bytes` (reference-counted, no content copy) |
| **Bounded telemetry queue** | `mpsc::channel` with fixed capacity; drops are counted, never block the client |
| **Micro-batched persistence** | Background worker flushes every 500ms or every 1000 events, whichever comes first |
| **Client disconnect = upstream cancel** | When the client drops the connection, the upstream stream is cancelled immediately |

---

## Supported Providers

| Provider | Status | SSE Streaming | Failover | Health Check |
|---|---|---|---|---|
| **Ollama** | Supported | Yes | Yes | `/api/tags` |
| **OpenAI-compatible** | Supported | Yes | Yes | `/healthz` |
| **Groq** | Supported | Yes | Yes | `/healthz` |
| **vLLM** | Supported | Yes | Yes | `/health` |
| **Anthropic** | Planned | - | - | - |
| **Any OpenAI-compatible** | Pass-through | Yes | Yes | Configurable |

> oxideLLM uses the **OpenAI chat completions format** as its canonical API. Any provider that exposes a `/v1/chat/completions`-compatible endpoint works out of the box.

---

## Configuration

Configuration is resolved with the following precedence:

1. **CLI arguments** (`--port`, `--upstream-base-url`, etc.)
2. **Environment variables** (`LLMK_PORT`, `LLMK_UPSTREAM_BASE_URL`, etc.)
3. **TOML config file** (`config.toml` in the project root)

### Example `config.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[[upstreams]]
id = "primary"
provider = "ollama"
base_url = "http://127.0.0.1:11434"
priority = 1

[[upstreams]]
id = "fallback"
provider = "mock"
base_url = "http://127.0.0.1:9000"
priority = 2

[upstream_health]
interval_ms = 5000
timeout_ms = 1000

[telemetry]
capacity = 65536
log_path = "telemetry_events.jsonl"
batch_size = 1000
flush_interval_ms = 500
```

> More examples: [examples/](examples/) | Full parameter reference: [config.rs](src/config.rs)

### API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe (telemetry status) |
| `GET` | `/analytics` | Request counters and telemetry stats |
| `POST` | `/v1/chat/completions` | OpenAI-compatible chat streaming |

---

## Resilience & Failover

oxideLLM implements enterprise-grade resilience through two coordinated mechanisms:

**1. Active Background Health Checking**
A background worker periodically pings each upstream's health endpoint. Unhealthy upstreams are removed from routing until they recover.

**2. Transparent Client-Side Failover**
When an upstream returns a retryable error (429, 502, 503, 504) or a network failure, the gateway automatically retries on the next healthy upstream - without interrupting the client's streaming connection.

<details>
<summary><b>Test failover yourself</b></summary>

```toml
# config.toml - simulate primary failure
[server]
host = "127.0.0.1"
port = 8080

[upstream_health]
interval_ms = 3000
timeout_ms = 1000

[[upstreams]]
id = "primary-dead"
provider = "mock"
base_url = "http://127.0.0.1:9000"   # Not running
priority = 1

[[upstreams]]
id = "fallback-alive"
provider = "mock"
base_url = "http://127.0.0.1:9001"   # Running
priority = 2
```

```bash
# Terminal 1: start only the fallback mock
cargo run --manifest-path mock/Cargo.toml -- --port 9001

# Terminal 2: start the gateway
cargo run

# Terminal 3: send a request
curl -N -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "mock", "messages": [{"role": "user", "content": "test"}], "stream": true}'
```

The gateway will log `WARN upstream marked unhealthy upstream_id="primary-dead"` and transparently route to the fallback. When the primary recovers, you'll see `INFO upstream health restored`.

</details>

---

## Testing

```bash
# Full quality gate
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

**Current test suite (v0.9.0):** `14 passed | 0 failed | 0 ignored`

Tests cover: multi-upstream parsing, SSE parsing, proxy failover, health checking, telemetry overflow, body size limits, timeout enforcement, header filtering, model-based routing, and end-to-end integration.

---

## How oxideLLM is Different

| | oxideLLM | Traditional LLM Gateways |
|---|---|---|
| **Runtime** | Compiled Rust (no GC, no interpreter) | Python/Node.js (GC pauses, interpreter overhead) |
| **Telemetry** | Async, off critical path (bounded MPSC + micro-batching) | Synchronous logging, tracing, and DB writes per request |
| **SSE Handling** | Zero-copy byte stream forwarding | Per-token JSON parse -> object -> re-serialize |
| **Database on Hot Path** | Never (by design invariant) | Often (Postgres/Redis per request) |
| **Deployment** | Single static binary | Python env + Postgres + Redis + workers |
| **Measured Overhead** | ~13% on localhost (WSL2), ~1% data-plane isolated | Up to 75.6% observed under load |

> This is not a critique of specific projects - it's a comparison of **architectural patterns**. Run times with synchronous persistence are excellent for many use cases but create bottlenecks under high-concurrency LLM streaming workloads.

---

## Roadmap

- [x] **Stage 0** - Repository foundation, CI, Rust scaffold
- [x] **Stage 1** - Mock SSE server + baseline benchmarks
- [x] **Stage 2** - Proxy pass-through with bounded telemetry
- [x] **Stage 5** - Micro-batched async persistence
- [x] **Stage 7** - GitHub-ready (templates, CI, docs)
- [x] **Stage 8** - Multi-upstream failover + active health checking
- [ ] **Stage 3** - Lock-free contention validation (`perf stat`, flamegraph)
- [ ] **Stage 4** - Memory allocation profiling (`heaptrack`, DHAT)
- [ ] **Stage 6** - vLLM native parity benchmark (bare metal)
- [ ] **Future** - Prometheus `/metrics` endpoint
- [ ] **Future** - Rate limiting per tenant/route/model
- [ ] **Future** - Intelligent semantic/cost-based routing
- [ ] **Future** - Multi-stage Dockerfile + pre-built release binaries

---

## Sponsors

oxideLLM is **100% free and open-source**. If the gateway reduces your infrastructure costs and downtime, consider supporting voluntary development:

<div align="center">

[**GitHub Sponsors**](https://github.com/sponsors/lugga1s) | [**Buy Me a Coffee**](https://www.buymeacoffee.com/lugga1s)

</div>

---

## License

**AGPL-3.0-or-later** - see [LICENSE](LICENSE) for full text and [licensing-strategy.md](docs/licensing-strategy.md) for the commercial open-source policy.

---

## Contributing

Contributions are welcome! Please read the [Contributing Guide](CONTRIBUTING.md) and [Security Policy](SECURITY.md) before submitting changes.

**Quick checklist:** `cargo fmt` | `cargo check` | `cargo test` | `cargo clippy` | no secrets committed.

---

<details>
<summary><b>Technical Documentation</b></summary>

> Internal engineering manuals used to keep the project aligned between humans and AI agents.

| Document | Purpose |
|---|---|
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guide and PR checklist |
| [SECURITY.md](SECURITY.md) | Security policy and responsible disclosure |
| [examples/](examples/) | Ready-to-use TOML configurations |
| [docs/architecture.md](docs/architecture.md) | Rust architecture blueprint |
| [docs/implementation-playbook.md](docs/implementation-playbook.md) | Implementation history and stages |
| [docs/agent-task-cards.md](docs/agent-task-cards.md) | Completed task cards |
| [docs/validation-gates.md](docs/validation-gates.md) | Quality gate contracts |
| [docs/verification-ledger.md](docs/verification-ledger.md) | Test execution records |
| [docs/production-ritual.md](docs/production-ritual.md) | Release semaphores |
| [docs/github-workflow.md](docs/github-workflow.md) | Commit, PR, and tag standards |
| [docs/operational-priorities.md](docs/operational-priorities.md) | Functional MVP philosophy |
| [docs/tooling-setup.md](docs/tooling-setup.md) | Rust, k6, and Docker installation |
| [benchmarks/](benchmarks/) | Benchmark results and methodology |

</details>
