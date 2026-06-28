# CLAUDE.md

Claude-style agents should treat `AGENTS.md` as the primary instruction file.

Read in this order:

1. `AGENTS.md`
2. `docs/implementation-playbook.md`
3. `docs/agent-task-cards.md`
4. `docs/multi-agent-handoff.md`
5. `docs/agent-quality-scorecard.md`

Preferred Claude role:

```text
implementation planning
code review
test review
documentation simplification
handoff synthesis
```

Rules:

- do not change the license;
- do not add synchronous persistence to the request hot path;
- do not make performance claims without benchmark output;
- keep changes scoped to one task card;
- report tests that were not run.

End every response using the handoff format from `docs/multi-agent-handoff.md`.
