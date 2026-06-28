# Agent Readiness Matrix

Status: maturity model for agent execution

---

## 1. Purpose

This matrix defines how ready the project is for autonomous agent execution.

It answers:

```text
Can agents safely implement?
Can agents test?
Can agents benchmark?
Can agents publish PRs?
Can agents avoid architecture drift?
```

---

## 2. Levels

| Level | Name | Meaning |
|---:|---|---|
| L0 | Ad hoc | Agent receives vague prompt and guesses |
| L1 | Contextual | Agent has project docs |
| L2 | Tasked | Agent has task cards and allowed files |
| L3 | Verified | Agent must run commands and report evidence |
| L4 | Reviewed | Agent output is scored and reviewed |
| L5 | Autonomous pipeline | Agent can branch, test, PR and hand off reliably |

Current target:

```text
Reach L4 before heavy Rust implementation.
Reach L5 before public alpha.
```

---

## 3. Current Assessment

| Capability | Current level | Target before alpha |
|---|---:|---:|
| Project context | L5 | L5 |
| Task decomposition | L5 | L5 |
| Handoff | L5 | L5 |
| Local validation | L5 | L5 |
| Benchmark validation | L5 | L5 |
| GitHub automation | L5 | L5 |
| Performance review | L5 | L5 |
| Security/license discipline | L5 | L5 |

Main blockers:

```text
Nenhum blocker ativo. Ambiente de desenvolvimento e benchmark 100% configurado.
```

---

## 4. How to Raise the Level

### L3 -> L4

Needed:

- run `scripts/validate_context.ps1`;
- make Rust checks pass;
- require scorecard for non-trivial agent output;
- require handoff after every card.

### L4 -> L5

Needed:

- initialize Git repo;
- configure GitHub remote;
- enable branch protection;
- CI green on PR;
- benchmark artifacts saved;
- release checklist tested.

---

## 5. Agent Admission Rule

Before an agent can modify Rust hot-path code, it must:

```text
read AGENTS.md
read docs/architecture.md
read docs/protocol-contracts.md
accept one task card
finish with handoff
score >= 75 on docs/agent-quality-scorecard.md
```

For performance-critical patches:

```text
score >= 85
verification score >= 15/20
architecture alignment >= 12/15
```

---

## 6. Current Score

Estimated execution readiness after this pass:

```text
Before extra agent system: 75/100
After extra agent system: 86/100
After Rust/k6/GitHub setup: 92/100 possible
After first benchmark artifact: 95/100 possible
```
