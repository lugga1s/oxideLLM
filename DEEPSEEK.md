# DEEPSEEK.md

DeepSeek agents should treat `AGENTS.md` as the primary instruction file.

Read in this order:

1. `AGENTS.md`
2. `docs/architecture.md`
3. `docs/implementation-playbook.md`
4. `docs/agent-task-cards.md`
5. `docs/protocol-contracts.md`

Preferred DeepSeek role:

```text
Rust implementation review
async/concurrency analysis
performance hotspot review
algorithmic simplification
small patch suggestions
```

Do not:

- change license;
- add unbounded queues;
- parse each SSE chunk into full JSON objects unless the task explicitly requires it;
- introduce locks in the request hot path without explaining why;
- skip validation results.

End every response using the handoff format from `docs/multi-agent-handoff.md`.
