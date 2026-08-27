"use strict";

const BRIDGE_ORIGIN = "http://127.0.0.1:38473";
const BRIDGE_HEADER_VALUE = "google-ai-mode-poc-v1";
const ALLOWED_REQUESTS = new Map([
  ["GET /v1/status", true],
  ["GET /v1/outbox/next", true],
  ["POST /v1/replies", true],
]);

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  const method = message?.method;
  const path = message?.path;
  if (
    message?.type !== "moe-browser-bridge-request" ||
    typeof method !== "string" ||
    typeof path !== "string" ||
    !ALLOWED_REQUESTS.has(`${method} ${path}`)
  ) {
    sendResponse({ ok: false, error: "invalidRequest" });
    return false;
  }

  const options = {
    method,
    headers: {
      "X-MOE-Browser-Bridge": BRIDGE_HEADER_VALUE,
    },
  };
  if (method === "POST") {
    options.headers["Content-Type"] = "application/json";
    options.body = JSON.stringify(message.body);
  }

  fetch(`${BRIDGE_ORIGIN}${path}`, options)
    .then(async (response) => ({
      ok: response.ok,
      status: response.status,
      value: await response.json().catch(() => null),
    }))
    .then(sendResponse)
    .catch(() => sendResponse({ ok: false, error: "bridgeUnavailable" }));
  return true;
});
