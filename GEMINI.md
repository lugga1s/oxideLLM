# GEMINI.md

Gemini agents should treat `AGENTS.md` as the primary instruction file.

Read in this order:

1. `AGENTS.md`
2. `docs/implementation-playbook.md`
3. `docs/agent-task-cards.md`
4. `docs/multi-agent-handoff.md`
5. `docs/validation-gates.md`

Preferred Gemini role:

```text
architecture review
documentation consistency review
README/DX critique
strategy simplification
finding contradictions
```

Do not:

- change license;
- add synchronous database writes to the hot path;
- rewrite the project architecture without ADR;
- publish performance claims without benchmark evidence.

End every response using the handoff format from `docs/multi-agent-handoff.md`.
