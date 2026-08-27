import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { connectDesktop } from "./desktop-link.mjs";
import { createRelay } from "./relay.mjs";

const runtimeDirectory = new URL("./runtime/", import.meta.url);
const runtimeSnapshot = new URL("./runtime/room-snapshot.json", import.meta.url);
const fixtureSnapshot = new URL("../remote-mcp/fixtures/room-snapshot.json", import.meta.url);
const deviceToken = randomBytes(32).toString("hex");

async function requestJson(url, { method = "GET", body, token } = {}) {
  const response = await fetch(url, {
    method,
    headers: {
      ...(body ? { "content-type": "application/json" } : {}),
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  return { status: response.status, body: await response.json() };
}

async function waitFor(predicate, timeoutMs = 2_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error("Timed out waiting for relay state");
}

await rm(runtimeDirectory, { recursive: true, force: true });
await mkdir(runtimeDirectory, { recursive: true });
const snapshot = JSON.parse(await readFile(fixtureSnapshot, "utf8"));
snapshot.generatedAt = "2026-08-12T01:00:00+09:00";
snapshot.rooms[0].messages.push({
  id: "relay-runtime-message",
  roomId: "moe-dev-room",
  authorId: "codex",
  recipients: ["owner", "claude-web"],
  body: "DESKTOP_RELAY_ROOM_OK",
  createdAt: "2026-08-12T01:00:00+09:00",
  artifactIds: [],
});
await writeFile(runtimeSnapshot, JSON.stringify(snapshot, null, 2), "utf8");

const relay = await createRelay({ deviceToken, requestTimeoutMs: 1_000 });
let desktop;
let reconnectedDesktop;

try {
  const unauthorized = await requestJson(`${relay.baseUrl}/desktop-link`, {
    method: "POST",
    token: "not-the-device-token",
  });
  assert.equal(unauthorized.status, 401);
  assert.equal(unauthorized.body.code, "device_unauthorized");

  desktop = await connectDesktop({
    relayUrl: relay.baseUrl,
    deviceToken,
    snapshotFile: runtimeSnapshot,
  });
  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "connected";
  });

  const [earlierPage, latestPage] = await Promise.all([
    requestJson(`${relay.baseUrl}/mcp/read-room`, {
      method: "POST",
      body: { roomId: "moe-dev-room", afterMessageId: "welcome-1", limit: 1 },
    }),
    requestJson(`${relay.baseUrl}/mcp/read-room`, {
      method: "POST",
      body: { roomId: "moe-dev-room", afterMessageId: "welcome-3", limit: 1 },
    }),
  ]);
  assert.equal(earlierPage.status, 200);
  assert.equal(earlierPage.body.room.messages[0].id, "welcome-2");
  assert.equal(latestPage.status, 200);
  assert.equal(latestPage.body.room.messages[0].id, "relay-runtime-message");
  assert.equal(latestPage.body.room.messages[0].body, "DESKTOP_RELAY_ROOM_OK");

  const forbiddenPath = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    method: "POST",
    body: { roomId: "moe-dev-room", path: "C:\\private.txt" },
  });
  assert.equal(forbiddenPath.status, 400);
  assert.equal(forbiddenPath.body.code, "invalid_request");

  const connectedStatus = await requestJson(`${relay.baseUrl}/status`);
  assert.equal(connectedStatus.body.retainedRoomCount, 0);
  assert.equal(connectedStatus.body.pendingRequestCount, 0);

  await desktop.close();
  desktop = null;
  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "offline";
  });

  const offlineRead = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    method: "POST",
    body: { roomId: "moe-dev-room" },
  });
  assert.equal(offlineRead.status, 503);
  assert.equal(offlineRead.body.code, "desktop_offline");

  reconnectedDesktop = await connectDesktop({
    relayUrl: relay.baseUrl,
    deviceToken,
    snapshotFile: runtimeSnapshot,
  });
  const recoveredRead = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    method: "POST",
    body: { roomId: "moe-dev-room", afterMessageId: "welcome-3", limit: 1 },
  });
  assert.equal(recoveredRead.status, 200);
  assert.equal(recoveredRead.body.room.messages[0].body, "DESKTOP_RELAY_ROOM_OK");

  console.log(
    JSON.stringify(
      {
        result: "PASS",
        transport: "local persistent outbound HTTP NDJSON stream",
        bind: "127.0.0.1:<ephemeral>",
        authentication: "ephemeral probe-only bearer token",
        correlation: {
          concurrentRequests: 2,
          earlierMessageId: earlierPage.body.room.messages[0].id,
          latestMessageId: latestPage.body.room.messages[0].id,
        },
        roomMarker: latestPage.body.room.messages[0].body,
        boundaries: {
          rawPathRejected: true,
          relayRetainedRoomCount: connectedStatus.body.retainedRoomCount,
          offlineReadRejected: offlineRead.body.code,
          reconnectRead: "PASS",
          publicNetworkUsed: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  await desktop?.close();
  await reconnectedDesktop?.close();
  await relay.close();
  await rm(runtimeDirectory, { recursive: true, force: true });
}
