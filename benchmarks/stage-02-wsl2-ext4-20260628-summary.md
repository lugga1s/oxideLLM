# Stage 2 WSL2 ext4 Benchmark Summary

Date: 2026-06-28  
Commit: `032d9285c7bdfaa50614368592213907613cbafe`  
Run ID: `stage-02-wsl2-ext4-20260628-032d928`  
Status: green for local WSL2/localhost gate  

## Environment

OS: `Linux-6.18.33.2-microsoft-standard-WSL2-x86_64-with-glibc2.43`  
CPU: AMD Ryzen 5 5600G with Radeon Graphics, 6 cores / 12 threads exposed to WSL2  
Memory: 7.4 GiB total, 6.6 GiB available at inspection time  
Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`  
Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`  
k6: `k6 v2.0.0 (commit/8c3be52cc1, go1.26.3, linux/amd64)`  

The benchmark was run from a temporary clone inside the WSL Linux filesystem:

```text
/root/oxidellm-bench-nLy5E2
```

This avoids the WSL warning about intensive I/O on Windows-mounted paths such as `/mnt/c/...`.

## Commands

```bash
RUN_ID=stage-02-wsl2-ext4-20260628-032d928 \
  bash scripts/run_stage2_benchmark_wsl.sh
```

The script ran:

```bash
cargo build --release
cargo build --manifest-path mock/Cargo.toml --release
k6 run -e VUS=1000 -e DURATION=30s -e TARGET_URL=http://127.0.0.1:9000/v1/chat/completions k6/proxy-vs-direct.js
k6 run -e VUS=1000 -e DURATION=30s -e TARGET_URL=http://127.0.0.1:8080/v1/chat/completions k6/proxy-vs-direct.js
```

Raw local artifacts:

```text
benchmarks/results/stage-02-wsl2-ext4-20260628-032d928-direct.json
benchmarks/results/stage-02-wsl2-ext4-20260628-032d928-gateway.json
benchmarks/results/stage-02-wsl2-ext4-20260628-032d928-summary.json
```

## Result

| Metric | Direct mock | Gateway -> mock |
|---|---:|---:|
| RPS | 20,919.35 | 18,118.36 |
| P95 latency | 56.51 ms | 74.51 ms |
| P99 latency | 70.16 ms | 92.47 ms |
| HTTP error rate | 0.00% | 0.00% |

RPS degradation:

```text
((20919.35483040676 - 18118.355617652458) / 20919.35483040676) * 100 = 13.39%
```

## Gate Comparison

Relevant gate from `docs/validation-gates.md` for WSL2/localhost:

```text
degradacao_rps_percent < 15
P99 registrado e comparado
http_req_failed < 0.1%
```

Outcome:

```text
degradacao_rps_percent = 13.39%
P99 direct = 70.16 ms
P99 gateway = 92.47 ms
http_req_failed = 0.00%
Status = green for local WSL2/localhost Stage 2
```

## Notes

Two prior runs from the Windows-mounted workspace (`/mnt/c/...`) passed functionally but exceeded the local performance gate:

| Run | Commit | Degradation | Status |
|---|---|---:|---|
| `stage-02-wsl2-20260628-184307-cb3e6d9` | `cb3e6d9` | 15.69% | yellow |
| `stage-02-wsl2-20260628-1850-cb3e6d9-rerun` | `cb3e6d9` | 18.74% | yellow |
| `stage-02-wsl2-20260628-optimized-032d928` | `032d928` | 16.73% | yellow |

Those runs should not be used for public performance claims. The ext4 run is the cleaner local evidence because it avoids Windows-mounted filesystem overhead noted by WSL.

This is still a local WSL2/localhost benchmark, not an external-upstream or distributed-network result.
