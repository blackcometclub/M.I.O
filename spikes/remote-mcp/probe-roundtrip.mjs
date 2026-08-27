import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { resolveRoomSnapshotFile } from "./room-snapshot.mjs";

const runtimeDirectory = new URL("./runtime/", import.meta.url);
const runtimeSnapshot = new URL("./runtime/room-snapshot.json", import.meta.url);
const fixtureSnapshot = new URL("./fixtures/room-snapshot.json", import.meta.url);

assert.throws(
  () => resolveRoomSnapshotFile(fileURLToPath(fixtureSnapshot)),
  /runtime directory/u,
);

await rm(runtimeDirectory, { recursive: true, force: true });
await mkdir(runtimeDirectory, { recursive: true });
const snapshot = JSON.parse(await readFile(fixtureSnapshot, "utf8"));
snapshot.generatedAt = "2026-08-12T00:30:00+09:00";
snapshot.rooms[0].messages.push({
  id: "probe-runtime-message",
  roomId: "moe-dev-room",
  authorId: "codex",
  recipients: ["owner", "claude-web"],
  body: "REMOTE_ROOM_RUNTIME_OK",
  createdAt: "2026-08-12T00:30:00+09:00",
  artifactIds: [],
});
await writeFile(runtimeSnapshot, JSON.stringify(snapshot, null, 2), "utf8");

const serverProcess = spawn(process.execPath, ["server.mjs"], {
  cwd: new URL(".", import.meta.url),
  env: {
    ...process.env,
    MOE_REMOTE_MCP_HOST: "127.0.0.1",
    MOE_REMOTE_MCP_PORT: "0",
    MOE_REMOTE_MCP_PATH: "/local-probe/mcp",
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
      const match = chunk.match(/http:\/\/127\.0\.0\.1:(\d+)\/local-probe\/mcp/);
      if (match) {
        clearTimeout(timeout);
        resolve(`http://127.0.0.1:${match[1]}/local-probe/mcp`);
      }
    });
  });
}

function parseTextResult(result) {
  const text = result.content.find((item) => item.type === "text")?.text;
  assert.equal(typeof text, "string");
  return JSON.parse(text);
}

let client;

try {
  const endpoint = await waitForEndpoint();
  client = new Client({ name: "moe-remote-mcp-probe", version: "0.0.0" });
  const transport = new StreamableHTTPClientTransport(new URL(endpoint));
  await client.connect(transport);

  const toolList = await client.listTools();
  const toolNames = toolList.tools.map((tool) => tool.name).sort();
  assert.deepEqual(toolNames, ["moe_read_room", "moe_status", "ping_moe"]);
  assert.ok(toolList.tools.every((tool) => tool.annotations?.readOnlyHint === true));

  const ping = parseTextResult(
    await client.callTool({ name: "ping_moe", arguments: {} }),
  );
  const status = parseTextResult(
    await client.callTool({ name: "moe_status", arguments: {} }),
  );
  const room = parseTextResult(
    await client.callTool({
      name: "moe_read_room",
      arguments: {
        roomId: "moe-dev-room",
        afterMessageId: "welcome-3",
        limit: 1,
      },
    }),
  );
  const missingRoom = await client.callTool({
    name: "moe_read_room",
    arguments: { roomId: "not-registered" },
  });
  const missingCursor = await client.callTool({
    name: "moe_read_room",
    arguments: {
      roomId: "moe-dev-room",
      afterMessageId: "not-registered",
    },
  });

  assert.deepEqual(ping, { ok: true, service: "moe-remote-mcp-spike" });
  assert.equal(status.ok, true);
  assert.equal(status.relay, "local-only");
  assert.equal(status.desktop, "not-connected");
  assert.deepEqual(status.capabilities, ["ping_moe", "moe_status", "moe_read_room"]);
  assert.equal(room.ok, true);
  assert.equal(room.room.id, "moe-dev-room");
  assert.equal(room.room.messages.length, 1);
  assert.equal(room.room.messages[0].id, "probe-runtime-message");
  assert.equal(room.room.messages[0].body, "REMOTE_ROOM_RUNTIME_OK");
  assert.equal(room.page.returned, 1);
  assert.equal(missingRoom.isError, true);
  assert.equal(parseTextResult(missingRoom).code, "room_not_found");
  assert.equal(missingCursor.isError, true);
  assert.equal(parseTextResult(missingCursor).code, "cursor_not_found");

  console.log(
    JSON.stringify(
      {
        result: "PASS",
        transport: "Streamable HTTP (JSON response, stateless)",
        endpoint: "http://127.0.0.1:<ephemeral>/<secret>/mcp",
        tools: toolNames,
        ping,
        status,
        room: {
          id: room.room.id,
          messageIds: room.room.messages.map((message) => message.id),
          returned: room.page.returned,
          runtimeMarkerObserved: room.room.messages[0].body,
        },
        negativeCases: {
          missingRoom: "PASS",
          missingCursor: "PASS",
          snapshotOutsideRuntime: "PASS",
        },
      },
      null,
      2,
    ),
  );
} finally {
  await client?.close();
  serverProcess.kill("SIGTERM");
  await rm(runtimeDirectory, { recursive: true, force: true });
}
