import assert from "node:assert/strict";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const endpoint = process.env.MOE_REMOTE_MCP_URL;

if (!endpoint) {
  throw new Error("MOE_REMOTE_MCP_URL is required");
}

const url = new URL(endpoint);
assert.equal(url.protocol, "https:", "The public probe requires HTTPS");

const client = new Client({ name: "moe-public-mcp-probe", version: "0.0.0" });

function parseTextResult(result) {
  const text = result.content.find((item) => item.type === "text")?.text;
  assert.equal(typeof text, "string");
  return JSON.parse(text);
}

try {
  await client.connect(new StreamableHTTPClientTransport(url));
  const listed = await client.listTools();
  const tools = listed.tools.map((tool) => tool.name).sort();
  assert.deepEqual(tools, ["moe_read_room", "moe_status", "ping_moe"]);

  const ping = parseTextResult(
    await client.callTool({ name: "ping_moe", arguments: {} }),
  );
  const status = parseTextResult(
    await client.callTool({ name: "moe_status", arguments: {} }),
  );
  const room =
    process.env.MOE_PUBLIC_ROOM_PROBE === "1"
      ? parseTextResult(
          await client.callTool({
            name: "moe_read_room",
            arguments: {
              roomId: "moe-dev-room",
              afterMessageId: "welcome-3",
              limit: 1,
            },
          }),
        )
      : null;

  assert.equal(ping.ok, true);
  assert.equal(status.ok, true);
  if (room) {
    assert.equal(room.room.messages.length, 1);
    assert.equal(room.room.messages[0].body, "CLAUDE_WEB_ROOM_RUNTIME_OK");
  }

  console.log(
    JSON.stringify(
      {
        result: "PASS",
        transport: "public HTTPS Streamable HTTP",
        hostType: url.hostname.endsWith(".trycloudflare.com")
          ? "temporary TryCloudflare"
          : "other",
        tools,
        ping,
        status,
        room: room
          ? {
              id: room.room.id,
              messageIds: room.room.messages.map((message) => message.id),
              marker: room.room.messages[0].body,
            }
          : "not requested",
      },
      null,
      2,
    ),
  );
} finally {
  await client.close();
}
