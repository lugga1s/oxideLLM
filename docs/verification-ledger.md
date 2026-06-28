# Verification Ledger

Status: template for recording real execution evidence

This project should keep a lightweight record of important validation runs. Do not treat this as bureaucracy; it is how we remember what was actually proven.

Recommended location:

```text
benchmarks/results/
.context/runs/
```

---

## 1. Run Record Template

```text
Run ID:
Date:
Agent:
Task card:
Commit:
Environment:
Tools:
Command:
Result:
Status: green/yellow/red
Artifact path:
Notes:
```

---

## 2. Benchmark Record Template

```text
Benchmark ID:
Date:
Commit:
Hardware:
OS:
Rust version:
k6 version:
Upstream:
Gateway config:

Direct command:
Direct RPS:
Direct P95:
Direct P99:
Direct error rate:

Gateway command:
Gateway RPS:
Gateway P95:
Gateway P99:
Gateway error rate:

RPS degradation percent:
TTFT overhead:
Status:
```

---

## 3. Missing Tool Record

```text
Tool:
Expected command:
Observed error:
Install doc:
Impact:
Next action:
```

---

## 4. Minimal Rule

If a result affects a public claim or next-stage decision, write it down.

---

## 5. Verification Records

```text
Run ID: RUN-2026-06-28-STG8-VAL
Date: 2026-06-28
Agent: Antigravity (Gemini 3.5 Flash)
Task card: Stage 8 (TC-022, TC-023, TC-024)
Commit: 461ebef
Environment: Local Windows 11
Tools: rustc 1.80+, cargo, powershell
Command: cargo test --all; .\scripts\validate_context.ps1
Result: 14 tests passed, context validation passed. Background health worker successfully detects offline upstreams and logs warning as expected.
Status: green
Artifact path: none
Notes: Validated fallback/failover routing and active background health checks. Banner and sponsors info added to startup.
```
