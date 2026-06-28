import http from "k6/http";
import { check } from "k6";

const targetUrl = __ENV.TARGET_URL || "http://localhost:8080/v1/chat/completions";
const vus = Number(__ENV.VUS || 1000);
const duration = __ENV.DURATION || "30s";

export const options = {
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
    model: "mock-model",
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

