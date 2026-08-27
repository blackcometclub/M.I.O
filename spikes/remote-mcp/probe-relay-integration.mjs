import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { spawn } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { connectDesktop } from "../relay-roundtrip/desktop-link.mjs";
import { createRelay } from "../relay-roundtrip/relay.mjs";

const runtimeDirectory = new URL("../relay-roundtrip/runtime/", import.meta.url);
const runtimeSnapshot = new URL(
  "../relay-roundtrip/runtime/mcp-integration-room.json",
  import.meta.url,
);
const fixtureSnapshot = new URL("./fixtures/room-snapshot.json", import.meta.url);
const deviceToken = randomBytes(32).toString("hex");

function parseTextResult(result) {
  const text = result.content.find((item) => item.type === "text")?.text;
  assert.equal(typeof text, "string");
  return JSON.parse(text);
}

async function waitFor(predicate, timeoutMs = 2_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error("Timed out waiting for integration state");
}

async function relayStatus(baseUrl) {
  const response = await fetch(new URL("/status", baseUrl));
  return response.json();
}

await rm(runtimeDirectory, { recursive: true, force: true });
await mkdir(runtimeDirectory, { recursive: true });
const snapshot = JSON.parse(await readFile(fixtureSnapshot, "utf8"));
const createdAt = new Date().toISOString();
snapshot.generatedAt = createdAt;
snapshot.rooms[0].messages.push({
  id: "mcp-relay-desktop-message",
  roomId: "moe-dev-room",
  authorId: "codex",
  recipients: ["owner", "claude-web"],
  body: "REMOTE_MCP_RELAY_DESKTOP_OK",
  createdAt,
  artifactIds: [],
});
await writeFile(runtimeSnapshot, JSON.stringify(snapshot, null, 2), "utf8");

const relay = await createRelay({ deviceToken, requestTimeoutMs: 1_000 });
let desktop = await connectDesktop({
  relayUrl: relay.baseUrl,
  deviceToken,
  snapshotFile: runtimeSnapshot,
});

const serverProcess = spawn(process.execPath, ["server.mjs"], {
  cwd: new URL(".", import.meta.url),
  env: {
    ...process.env,
    MOE_REMOTE_MCP_HOST: "127.0.0.1",
    MOE_REMOTE_MCP_PORT: "0",
    MOE_REMOTE_MCP_PATH: "/relay-integration/mcp",
    MOE_RELAY_BASE_URL: relay.baseUrl,
  },
  stdio: ["ignore", "pipe", "pipe"],
});

let stderr = "";
serverProcess.stderr.setEncoding("utf8");
serverProcess.stderr.on("data", (chunk) => {
  stderr += chunk;
});

function waitForEndpoint() {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Timed out waiting for MCP server. ${stderr}`));
    }, 10_000);

    serverProcess.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`MCP server exited before startup (code ${code}). ${stderr}`));
    });

    serverProcess.stdout.setEncoding("utf8");
    serverProcess.stdout.on("data", (chunk) => {
      const match = chunk.match(
        /http:\/\/127\.0\.0\.1:(\d+)\/relay-integration\/mcp/u,
      );
      if (match) {
        clearTimeout(timeout);
        resolve(`http://127.0.0.1:${match[1]}/relay-integration/mcp`);
      }
    });
  });
}

let client;
try {
  const endpoint = await waitForEndpoint();
  client = new Client({
    name: "moe-relay-integration-probe",
    version: "0.0.0",
  });
  await client.connect(new StreamableHTTPClientTransport(new URL(endpoint)));

  const connectedStatus = parseTextResult(
    await client.callTool({ name: "moe_status", arguments: {} }),
  );
  assert.equal(connectedStatus.relay, "local-outbound-link");
  assert.equal(connectedStatus.desktop, "connected");

  const firstRead = parseTextResult(
    await client.callTool({
      name: "moe_read_room",
      arguments: {
        roomId: "moe-dev-room",
        afterMessageId: "welcome-3",
        limit: 1,
      },
    }),
  );
  assert.equal(firstRead.room.messages.length, 1);
  assert.equal(firstRead.room.messages[0].id, "mcp-relay-desktop-message");
  assert.equal(firstRead.room.messages[0].body, "REMOTE_MCP_RELAY_DESKTOP_OK");

  await desktop.close();
  desktop = null;
  await waitFor(async () => (await relayStatus(relay.baseUrl)).desktop === "offline");

  const offlineResult = await client.callTool({
    name: "moe_read_room",
    arguments: { roomId: "moe-dev-room", limit: 1 },
  });
  assert.equal(offlineResult.isError, true);
  assert.equal(parseTextResult(offlineResult).code, "desktop_offline");

  desktop = await connectDesktop({
    relayUrl: relay.baseUrl,
    deviceToken,
    snapshotFile: runtimeSnapshot,
  });
  const recoveredStatus = parseTextResult(
    await client.callTool({ name: "moe_status", arguments: {} }),
  );
  assert.equal(recoveredStatus.desktop, "connected");

  const recoveredRead = parseTextResult(
    await client.callTool({
      name: "moe_read_room",
      arguments: {
        roomId: "moe-dev-room",
        afterMessageId: "welcome-3",
        limit: 1,
      },
    }),
  );
  assert.equal(
    recoveredRead.room.messages[0].body,
    "REMOTE_MCP_RELAY_DESKTOP_OK",
  );

  const finalRelayStatus = await relayStatus(relay.baseUrl);
  assert.equal(finalRelayStatus.retainedRoomCount, 0);
  assert.equal(finalRelayStatus.pendingRequestCount, 0);

  console.log(
    JSON.stringify(
      {
        result: "PASS",
        path: [
          "official MCP client",
          "Remote MCP",
          "local Relay",
          "Desktop room source",
          "local Relay",
          "Remote MCP result",
        ],
        status: connectedStatus,
        room: {
          id: firstRead.room.id,
          messageId: firstRead.room.messages[0].id,
          marker: firstRead.room.messages[0].body,
        },
        recovery: {
          offlineError: "desktop_offline",
          statusAfterReconnect: recoveredStatus.desktop,
          readAfterReconnect: "PASS",
        },
        relayRetention: {
          rooms: finalRelayStatus.retainedRoomCount,
          pendingRequests: finalRelayStatus.pendingRequestCount,
        },
        publicNetworkUsed: false,
      },
      null,
      2,
    ),
  );
} finally {
  await client?.close();
  serverProcess.kill("SIGTERM");
  await desktop?.close();
  await relay.close();
  await rm(runtimeDirectory, { recursive: true, force: true });
}
