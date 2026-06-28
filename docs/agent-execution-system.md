# Agent Execution System

Status: operating system for AI-driven execution

---

## 1. Goal

Raise the probability that autonomous agents can finish the project by reducing ambiguity, enforcing evidence, and keeping work small.

The execution system has five layers:

```text
context
task card
implementation
verification
handoff
```

If one layer is missing, agent reliability drops.

---

## 2. Layer 1: Context

Context files tell agents what the project is.

Primary:

```text
AGENTS.md
docs/implementation-playbook.md
docs/architecture.md
docs/validation-gates.md
```

Compatibility:

```text
GEMINI.md
DEEPSEEK.md
CLAUDE.md
.github/copilot-instructions.md
.github/instructions/oxidellm.instructions.md
```

Rule:

```text
Context should be stable. Do not rewrite it every session.
```

---

## 3. Layer 2: Task Card

Task cards prevent overreach.

Every task must specify:

- objective;
- allowed files;
- commands;
- success condition;
- forbidden changes.

If a task cannot fit in a card, split it.

---

## 4. Layer 3: Implementation

Implementation should be narrow.

Good:

```text
Add upstream base_url config.
Write test for telemetry overflow.
Make k6 export summary JSON.
```

Bad:

```text
Make the gateway production ready.
Add all providers.
Refactor architecture.
```

---

## 5. Layer 4: Verification

Verification must use artifacts.

Accepted evidence:

- command output;
- test result;
- benchmark summary;
- generated file;
- diff;
- issue/PR URL;
- explicit missing-tool report.

Not accepted:

- "should work";
- "likely passes";
- "not tested but simple";
- "performance should be flat".

---

## 6. Layer 5: Handoff

Every agent must hand off in the standard format.

The next agent should be able to continue without reading the entire conversation.

---

## 7. Agent Failure Modes

| Failure mode | Prevention |
|---|---|
| Scope explosion | one task card only |
| Fake validation | require command output |
| Architecture drift | ADR required |
| Performance theater | benchmark artifact required |
| Context loss | handoff format |
| Model-specific behavior | adapter files for Gemini/Claude/Copilot |
| Rework loops | verification ledger |

---

## 8. Escalation

If an agent is stuck:

```text
stop adding code
write the blocker
write the exact command that failed
write the smallest next diagnostic
```

Do not compensate for uncertainty with bigger refactors.
