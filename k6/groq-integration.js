import http from "k6/http";
import { check, sleep } from "k6";

const targetUrl = __ENV.TARGET_URL || "http://localhost:8080/v1/chat/completions";
const groqApiKey = __ENV.GROQ_API_KEY;

export const options = {
  scenarios: {
    integration: {
      executor: "constant-vus",
      vus: 2,
      duration: "10s",
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
  },
};

export default function () {
  if (!groqApiKey) {
    throw new Error("GROQ_API_KEY environment variable is required");
  }

  const payload = JSON.stringify({
    model: "llama-3.1-8b-instant",
    stream: true,
    messages: [{ role: "user", content: "say hello in one word" }],
  });

  const params = {
    headers: {
      "Content-Type": "application/json",
      "Authorization": `Bearer ${groqApiKey}`,
      Accept: "text/event-stream",
    },
    timeout: "10s",
  };

  const res = http.post(targetUrl, payload, params);

  check(res, {
    "status is 200": (r) => r.status === 200,
    "contains DONE": (r) => r.body && r.body.includes("[DONE]"),
  });

  // Pacing to respect Groq free tier rate limits (30 RPM)
  sleep(5);
}
