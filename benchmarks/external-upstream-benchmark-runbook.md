# External Upstream Benchmark Runbook

This runbook defines how to measure oxideLLM against a real OpenAI-compatible upstream such as vLLM. The direct upstream result is the laboratory control. It is not the product target by itself.

The public goal is to prove controlled overhead, stable latency, and resilient gateway behavior. Competitive claims require an additional run where oxideLLM and comparable gateways are tested in the same lab against the same upstream.

---

## 1. Environment Requirements

For a meaningful real-upstream benchmark, prefer Linux native or cloud Linux with a dedicated GPU.

Minimum for scouting:

```text
1 GPU host running vLLM or another OpenAI-compatible server
1 CPU host or separate process running k6
oxideLLM running either on the GPU host for scouting or on a separate CPU host for final evidence
```

Preferred for public evidence:

```text
Host A: GPU upstream server
Host B: oxideLLM or competitor gateway, one at a time
Host C: k6 load generator
private network between hosts
```

Record:

```text
cloud/vendor
instance type
CPU
GPU
RAM
OS/kernel
Rust version
k6 version
vLLM/upstream version
oxideLLM commit
gateway configuration
```

---

## 2. Start The Upstream

Example with vLLM:

```bash
vllm serve Qwen/Qwen2.5-0.5B-Instruct \
  --host 0.0.0.0 \
  --port 8000 \
  --max-model-len 2048
```

Validate direct streaming:

```bash
curl -N -X POST http://UPSTREAM_HOST:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen/Qwen2.5-0.5B-Instruct",
    "messages": [{"role": "user", "content": "Tell me a short story."}],
    "stream": true
  }'
```

CPU-only vLLM can be used for functional scouting, but it should not be used for strong public performance claims.

---

## 3. Start oxideLLM

Compile in release mode:

```bash
cargo build --release
```

Run the gateway:

```bash
./target/release/oxidellm \
  --host 0.0.0.0 \
  --port 8080 \
  --upstream-base-url http://UPSTREAM_HOST:8000 \
  --telemetry-log-path /dev/null
```

For final evidence, run another pass with telemetry persistence enabled and report both results.

---

## 4. Run Direct And Gateway Load Tests

Direct upstream:

```bash
k6 run \
  -e RUN_LABEL=external-upstream-direct \
  -e TARGET_URL=http://UPSTREAM_HOST:8000/v1/chat/completions \
  -e MODEL_NAME=Qwen/Qwen2.5-0.5B-Instruct \
  -e SUMMARY_PATH=benchmarks/results/external-upstream-direct-summary.json \
  k6/proxy-vs-direct.js
```

Gateway:

```bash
k6 run \
  -e RUN_LABEL=external-upstream-gateway \
  -e TARGET_URL=http://GATEWAY_HOST:8080/v1/chat/completions \
  -e MODEL_NAME=Qwen/Qwen2.5-0.5B-Instruct \
  -e SUMMARY_PATH=benchmarks/results/external-upstream-gateway-summary.json \
  k6/proxy-vs-direct.js
```

Use the same payload, concurrency, duration, and model for every run. For public evidence, run at least three repetitions per scenario.

---

## 5. Competitive Benchmark Extension

Only run competitive claims after the direct-vs-gateway harness is stable.

For each comparable gateway:

```text
use the same upstream;
use the same model;
use the same prompt payload;
use the same k6 script;
use the same concurrency and duration;
disable optional features that are not enabled in oxideLLM, or document the difference;
record gateway version and config;
run at least 3 repetitions.
```

Do not claim that a competitor is slower unless the benchmark includes its configuration, environment, command, raw artifact, and summary.

---

## 6. Required Metrics

Every summary must include:

```text
RPS
P50
P95
P99
TTFT
HTTP error rate
CPU usage
memory usage
network throughput
telemetry drops
upstream failures/retries
```

If TTFT is not captured by the load generator, use oxideLLM telemetry and state that direct TTFT was not measured externally.

---

## 7. Reporting Template

```text
Benchmark:
Date:
Commit:
Environment:
Upstream:
Model:
Load profile:
Direct result:
oxideLLM result:
Competitor result, if any:
RPS degradation:
P95 delta:
P99 delta:
TTFT delta:
Error rate:
CPU/memory notes:
Status:
Claim allowed:
Claim not allowed:
Next action:
```

Allowed public wording example:

```text
In this lab, oxideLLM added <X>% throughput overhead and <Y> ms P99 overhead
against <upstream>, while keeping telemetry off the request path. In the same
lab, it delivered <A> better throughput / <B> lower P99 than <gateway>.
```

Forbidden wording without evidence:

```text
oxideLLM is faster than all gateways.
oxideLLM is production-ready.
oxideLLM has zero overhead.
```
