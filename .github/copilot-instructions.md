# GitHub Copilot Instructions

This repository is the oxideLLM project: a Rust LLM gateway focused on streaming performance and telemetry off the critical path.

Before making code changes:

- read `AGENTS.md`;
- read `docs/implementation-playbook.md`;
- choose one task card from `docs/agent-task-cards.md`;
- validate with commands from `docs/validation-gates.md`.

Hard rules:

- license is `AGPL-3.0-or-later`;
- do not introduce synchronous database writes in the request path;
- do not introduce unbounded queues;
- do not parse every SSE chunk into full JSON unless the task explicitly requires it;
- do not claim performance success without benchmark output.

Rust checks:

```bash
cargo fmt --check
cargo check --all-targets
cargo test --all
cargo clippy --all-targets -- -D warnings
```

If a tool is missing, say so clearly instead of implying success.
