import http from "k6/http";
import { check } from "k6";

const targetUrl = __ENV.TARGET_URL || "http://localhost:8080/v1/chat/completions";
const vus = Number(__ENV.VUS || 1000);
const duration = __ENV.DURATION || "30s";
const summaryPath = __ENV.SUMMARY_PATH || "benchmarks/results/proxy-vs-direct-summary.json";
const runLabel = __ENV.RUN_LABEL || "proxy-vs-direct";
const modelName = __ENV.MODEL_NAME || "mock-model";

export const options = {
  summaryTrendStats: ["avg", "min", "med", "max", "p(90)", "p(95)", "p(99)"],
  scenarios: {
    steady: {
      executor: "constant-vus",
      vus,
      duration,
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.001"],
    http_req_duration: ["p(99)<1000"],
  },
};

export default function () {
  const payload = JSON.stringify({
    model: modelName,
    stream: true,
    messages: [{ role: "user", content: "say hello" }],
  });

  const params = {
    headers: {
      "Content-Type": "application/json",
      Accept: "text/event-stream",
    },
    timeout: "30s",
  };

  const res = http.post(targetUrl, payload, params);

  check(res, {
    "status is 200": (r) => r.status === 200,
    "contains DONE": (r) => r.body && r.body.includes("[DONE]"),
  });
}

function metricValues(data, metricName) {
  const metric = data.metrics[metricName];
  if (!metric) {
    return {};
  }

  return metric.values || metric;
}

export function handleSummary(data) {
  const reqs = metricValues(data, "http_reqs");
  const iterations = metricValues(data, "iterations");
  const failed = metricValues(data, "http_req_failed");
  const durationMetric = metricValues(data, "http_req_duration");
  const checks = metricValues(data, "checks");

  const summary = {
    benchmark_id: runLabel,
    generated_at: new Date().toISOString(),
    target_url: targetUrl,
    model_name: modelName,
    vus,
    duration,
    metrics: {
      http_reqs: {
        count: reqs.count ?? null,
        rate: reqs.rate ?? null,
      },
      iterations: {
        count: iterations.count ?? null,
        rate: iterations.rate ?? null,
      },
      http_req_failed: {
        rate: failed.rate ?? failed.value ?? null,
      },
      checks: {
        rate: checks.rate ?? checks.value ?? null,
        passes: checks.passes ?? null,
        fails: checks.fails ?? null,
      },
      http_req_duration_ms: {
        avg: durationMetric.avg ?? null,
        min: durationMetric.min ?? null,
        med: durationMetric.med ?? null,
        max: durationMetric.max ?? null,
        p90: durationMetric["p(90)"] ?? null,
        p95: durationMetric["p(95)"] ?? null,
        p99: durationMetric["p(99)"] ?? null,
      },
    },
    raw_metrics: data.metrics,
  };

  return {
    [summaryPath]: `${JSON.stringify(summary, null, 2)}\n`,
    stdout: `summary JSON saved to ${summaryPath}\n`,
  };
}
