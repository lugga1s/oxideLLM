# Context Packets

Status: ready-to-send packets for agent sessions

Use these packets when assigning work to another agent.

---

## 1. Minimal Implementation Packet

```text
You are working on oxideLLM, a Rust LLM gateway.

Read:
- AGENTS.md
- docs/implementation-playbook.md
- docs/agent-task-cards.md
- docs/multi-agent-handoff.md

Task card:
<CARD_ID>

Rules:
- one card only;
- do not change license;
- do not add synchronous persistence to the request path;
- run validation commands when available;
- report missing tools explicitly;
- finish with the handoff format.
```

---

## 2. Performance Review Packet

```text
Review this change for hot-path performance risk.

Read:
- docs/architecture.md
- docs/protocol-contracts.md
- docs/validation-gates.md
- docs/agent-quality-scorecard.md

Focus:
- SSE chunk handling;
- unnecessary String/Vec allocations;
- serde_json::Value in streaming path;
- locks/mutexes in request path;
- unbounded queues;
- disk/database writes before client response;
- missing cancellation.

Return findings ordered by severity and score the agent output.
```

---

## 3. Benchmark Packet

```text
You are the benchmark agent.

Read:
- docs/validation-gates.md
- benchmarks/README.md
- k6/proxy-vs-direct.js

Run direct and gateway tests if tools are available.
Save or summarize:
- command;
- environment;
- RPS;
- P95/P99;
- error rate;
- degradation percent.

If 1000 VUs is too much, run a smaller smoke test and mark it non-publicable.
```

---

## 4. Documentation Review Packet

```text
Review documentation for agent usability.

Read:
- README.md
- AGENTS.md
- docs/implementation-playbook.md
- docs/agent-task-cards.md
- docs/multi-agent-handoff.md

Find:
- contradictions;
- unclear instructions;
- excessive process;
- missing commands;
- claims without evidence.

Return top 10 fixes, prioritized.
```

---

## 5. GitHub PR Packet

```text
Prepare a draft PR.

Read:
- docs/github-workflow.md
- .github/PULL_REQUEST_TEMPLATE.md
- docs/agent-quality-scorecard.md

Do:
- git status;
- run validation commands possible in this environment;
- create branch if needed;
- push if remote exists;
- create draft PR with gh if authenticated.

If remote/auth is missing, stop and report blocker.
```
