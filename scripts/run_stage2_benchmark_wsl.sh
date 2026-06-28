#!/usr/bin/env bash
set -euo pipefail

RUN_ID="${RUN_ID:-stage-02-wsl2-$(date -u +%Y%m%d-%H%M%S)-$(git rev-parse --short HEAD)}"
VUS="${VUS:-1000}"
DURATION="${DURATION:-30s}"
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/oxidellm-target}"
RESULT_DIR="${RESULT_DIR:-benchmarks/results}"
MOCK_PORT="${MOCK_PORT:-9000}"
GATEWAY_PORT="${GATEWAY_PORT:-8080}"

if ! command -v cargo >/dev/null 2>&1 && [[ -f "${HOME}/.cargo/env" ]]; then
  # rustup does not always populate PATH for non-interactive WSL shells.
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

MOCK_PID=""
GATEWAY_PID=""

cleanup() {
  if [[ -n "${GATEWAY_PID}" ]]; then
    kill "${GATEWAY_PID}" 2>/dev/null || true
  fi
  if [[ -n "${MOCK_PID}" ]]; then
    kill "${MOCK_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

wait_for_url() {
  local url="$1"
  local name="$2"

  for _ in $(seq 1 60); do
    if curl -sf "${url}" >/dev/null; then
      return 0
    fi
    sleep 1
  done

  echo "${name} did not become ready at ${url}" >&2
  return 1
}

mkdir -p "${RESULT_DIR}"

export CARGO_TARGET_DIR="${TARGET_DIR}"

echo "building release binaries"
cargo build --release
cargo build --manifest-path mock/Cargo.toml --release

MOCK_LOG="/tmp/oxidellm-${RUN_ID}-mock.log"
GATEWAY_LOG="/tmp/oxidellm-${RUN_ID}-gateway.log"
rm -f "${MOCK_LOG}" "${GATEWAY_LOG}"

echo "starting mock on port ${MOCK_PORT}"
"${TARGET_DIR}/release/oxidellm-mock" \
  --host 127.0.0.1 \
  --port "${MOCK_PORT}" \
  >"${MOCK_LOG}" 2>&1 &
MOCK_PID="$!"

echo "starting gateway on port ${GATEWAY_PORT}"
"${TARGET_DIR}/release/oxidellm" \
  --host 127.0.0.1 \
  --port "${GATEWAY_PORT}" \
  --telemetry-log-path /dev/null \
  >"${GATEWAY_LOG}" 2>&1 &
GATEWAY_PID="$!"

if ! wait_for_url "http://127.0.0.1:${MOCK_PORT}/healthz" "mock"; then
  tail -80 "${MOCK_LOG}" >&2 || true
  exit 1
fi

if ! wait_for_url "http://127.0.0.1:${GATEWAY_PORT}/healthz" "gateway"; then
  tail -80 "${GATEWAY_LOG}" >&2 || true
  exit 1
fi

echo "servers ready mock_pid=${MOCK_PID} gateway_pid=${GATEWAY_PID}"

DIRECT_SUMMARY="${RESULT_DIR}/${RUN_ID}-direct.json"
GATEWAY_SUMMARY="${RESULT_DIR}/${RUN_ID}-gateway.json"
COMBINED_SUMMARY="${RESULT_DIR}/${RUN_ID}-summary.json"

echo "running direct benchmark: VUS=${VUS} DURATION=${DURATION}"
k6 run \
  -e RUN_LABEL="${RUN_ID}-direct" \
  -e TARGET_URL="http://127.0.0.1:${MOCK_PORT}/v1/chat/completions" \
  -e SUMMARY_PATH="${DIRECT_SUMMARY}" \
  -e VUS="${VUS}" \
  -e DURATION="${DURATION}" \
  k6/proxy-vs-direct.js

echo "running gateway benchmark: VUS=${VUS} DURATION=${DURATION}"
k6 run \
  -e RUN_LABEL="${RUN_ID}-gateway" \
  -e TARGET_URL="http://127.0.0.1:${GATEWAY_PORT}/v1/chat/completions" \
  -e SUMMARY_PATH="${GATEWAY_SUMMARY}" \
  -e VUS="${VUS}" \
  -e DURATION="${DURATION}" \
  k6/proxy-vs-direct.js

python3 - "${RUN_ID}" "${DIRECT_SUMMARY}" "${GATEWAY_SUMMARY}" "${COMBINED_SUMMARY}" <<'PY'
import json
import platform
import subprocess
import sys
from pathlib import Path

run_id, direct_path, gateway_path, combined_path = sys.argv[1:]
direct = json.loads(Path(direct_path).read_text())
gateway = json.loads(Path(gateway_path).read_text())

def metric(summary, *path):
    value = summary
    for part in path:
        value = value[part]
    return value

direct_rps = metric(direct, "metrics", "http_reqs", "rate")
gateway_rps = metric(gateway, "metrics", "http_reqs", "rate")
degradation = ((direct_rps - gateway_rps) / direct_rps) * 100 if direct_rps else None

summary = {
    "run_id": run_id,
    "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
    "os": platform.platform(),
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
    "k6": subprocess.check_output(["k6", "version"], text=True).strip(),
    "direct_summary": direct_path,
    "gateway_summary": gateway_path,
    "direct_rps": direct_rps,
    "gateway_rps": gateway_rps,
    "degradation_percent": degradation,
    "direct_p95_ms": metric(direct, "metrics", "http_req_duration_ms", "p95"),
    "gateway_p95_ms": metric(gateway, "metrics", "http_req_duration_ms", "p95"),
    "direct_p99_ms": metric(direct, "metrics", "http_req_duration_ms", "p99"),
    "gateway_p99_ms": metric(gateway, "metrics", "http_req_duration_ms", "p99"),
    "direct_error_rate": metric(direct, "metrics", "http_req_failed", "rate"),
    "gateway_error_rate": metric(gateway, "metrics", "http_req_failed", "rate"),
}

Path(combined_path).write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
PY

echo "combined summary saved to ${COMBINED_SUMMARY}"
