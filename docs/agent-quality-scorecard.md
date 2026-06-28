# Agent Quality Scorecard

Status: scoring rubric for agent output

Use this to judge whether an agent result is useful enough to keep.

---

## 1. Score Bands

| Score | Meaning |
|---:|---|
| 90-100 | merge-ready or very close |
| 75-89 | useful, needs review or small fixes |
| 60-74 | partially useful, risky |
| 40-59 | mostly exploratory, do not merge |
| 0-39 | harmful or misleading |

---

## 2. Dimensions

| Dimension | Weight |
|---|---:|
| Scope control | 15 |
| Correctness | 20 |
| Verification | 20 |
| Architecture alignment | 15 |
| Performance awareness | 10 |
| Security/license discipline | 10 |
| Handoff quality | 10 |

Total: 100.

---

## 3. Scoring Details

### Scope Control: 15

```text
15: exactly one task card
10: small extra changes, still coherent
5: broad changes mixed together
0: rewrote unrelated areas
```

### Correctness: 20

```text
20: behavior works and edge cases considered
15: main behavior works
10: plausible but incomplete
5: fragile
0: incorrect
```

### Verification: 20

```text
20: commands run and results reported
15: most commands run, missing tool clearly noted
10: partial validation
5: no command output
0: claims success falsely
```

### Architecture Alignment: 15

```text
15: respects hot path, bounded queues and streaming design
10: minor concerns
5: introduces future cleanup
0: violates core invariants
```

### Performance Awareness: 10

```text
10: avoids hot-path allocations/locks and notes benchmark impact
7: no obvious performance issue
3: performance unknown
0: introduces known bottleneck
```

### Security and License: 10

```text
10: no secrets, AGPL intact, safe headers/logging
7: minor documentation gap
3: unclear secret/logging risk
0: leaks secret or changes license
```

### Handoff Quality: 10

```text
10: standard handoff complete
7: usable summary
3: vague summary
0: no handoff
```

---

## 4. Merge Rule

Recommended:

```text
score >= 85: candidate for PR review
score 70-84: fix before PR
score < 70: keep as research, do not merge
```

For performance-critical code:

```text
verification must be >= 15
architecture alignment must be >= 12
```

---

## 5. Handoff Score Template

```text
Agent:
Task card:
Scope control:
Correctness:
Verification:
Architecture alignment:
Performance awareness:
Security/license:
Handoff quality:
Total:
Decision:
```
