---
applyTo: "**"
---

# oxideLLM Repository Instructions

Use `README.md`, `docs/architecture.md`, and `docs/validation-gates.md` as the public source of truth.

The product goal is a local-first Rust LLM gateway with:

- SSE streaming;
- low overhead proxying;
- telemetry off the critical path;
- benchmark-first claims;
- AGPL-3.0-or-later license.

Prefer small changes tied to a documented validation gate or public issue.

Never add:

- synchronous Postgres writes in the request path;
- unbounded telemetry queues;
- broad provider support before the OpenAI-compatible path works;
- benchmark claims without artifacts.

When in doubt, improve the shortest path to:

```text
mock -> gateway -> k6 -> upstream real -> telemetry minimum
```
