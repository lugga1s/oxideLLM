# Review Gates

Status: review workflow for agent-produced work

---

## 1. Purpose

Review gates turn agent output into a controlled pipeline.

They answer:

```text
Is this safe to keep?
Is this safe to test?
Is this safe to merge?
Is this safe to publish?
```

---

## 2. Gate Types

### Gate A: Context Gate

Required for every task.

Passes when:

```text
agent names the task card
agent states allowed files
agent states commands it will run
```

### Gate B: Code Gate

Required for code changes.

Passes when:

```text
diff is scoped
no core invariant is violated
no obvious hot-path bottleneck is added
```

### Gate C: Validation Gate

Required before PR.

Passes when:

```text
commands were run or missing tools were reported
results are explicit
failures are not hidden
```

### Gate D: Performance Gate

Required for hot-path changes.

Passes when:

```text
benchmark or profiling impact is measured
or change is marked non-publicable until measured
```

### Gate E: Publication Gate

Required before README/release claims.

Passes when:

```text
artifact exists
environment is recorded
direct vs gateway comparison exists
claim matches data
```

---

## 3. Review Roles

| Role | Responsibility |
|---|---|
| Implementer | make narrow change |
| Verifier | run commands and record evidence |
| Performance reviewer | inspect hot path |
| Context reviewer | check docs/claims |
| GitHub agent | prepare PR |

One agent may perform multiple roles for small tasks, but must label which roles it performed.

---

## 4. Required Review For Each Change

| Change type | Required gates |
|---|---|
| Docs only | A, C |
| Rust non-hot-path | A, B, C |
| Rust hot-path | A, B, C, D |
| Benchmark claim | A, C, D, E |
| License/security | A, B, C, human review |
| GitHub workflow | A, C |

---

## 5. Stop Conditions

Stop and ask for direction if:

- a change requires license alteration;
- a benchmark result contradicts public positioning;
- a task needs credentials/secrets;
- a fix requires broad architecture rewrite;
- tests fail and cause is unclear after one focused attempt.

---

## 6. Review Output Template

```text
Review gate:
Change type:
Gates applied:
Pass/fail:
Blocking findings:
Non-blocking findings:
Required next action:
Scorecard total:
```
