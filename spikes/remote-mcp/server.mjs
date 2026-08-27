import { createMcpExpressApp } from "@modelcontextprotocol/sdk/server/express.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { z } from "zod";
import { createRoomSource } from "./room-source.mjs";
import { resolveRoomSnapshotFile } from "./room-snapshot.mjs";

const host = process.env.MOE_REMOTE_MCP_HOST ?? "127.0.0.1";
const requestedPort = Number.parseInt(process.env.MOE_REMOTE_MCP_PORT ?? "3108", 10);
const mcpPath = process.env.MOE_REMOTE_MCP_PATH ?? "/mcp";
const additionalAllowedHost = process.env.MOE_REMOTE_MCP_ALLOWED_HOST;
const roomSnapshotFile = resolveRoomSnapshotFile(process.env.MOE_ROOM_SNAPSHOT_FILE);
const roomSource = createRoomSource({
  relayBaseUrl: process.env.MOE_RELAY_BASE_URL,
  snapshotFile: roomSnapshotFile,
});

if (!Number.isInteger(requestedPort) || requestedPort < 0 || requestedPort > 65535) {
  throw new Error("MOE_REMOTE_MCP_PORT must be an integer between 0 and 65535");
}

if (
  !mcpPath.startsWith("/") ||
  mcpPath.includes("?") ||
  mcpPath.includes("#") ||
  mcpPath.includes("..")
) {
  throw new Error("MOE_REMOTE_MCP_PATH must be an absolute URL path without query, fragment, or traversal");
}

if (
  additionalAllowedHost &&
  !/^[a-z0-9.-]+$/u.test(additionalAllowedHost)
) {
  throw new Error("MOE_REMOTE_MCP_ALLOWED_HOST must be one lowercase hostname");
}

function textResult(payload) {
  return {
    content: [
      {
        type: "text",
        text: JSON.stringify(payload),
      },
    ],
    structuredContent: payload,
  };
}

function errorResult(code, message) {
  return {
    isError: true,
    content: [{ type: "text", text: JSON.stringify({ ok: false, code, message }) }],
    structuredContent: { ok: false, code, message },
  };
}

function createServer() {
  const server = new McpServer({
    name: "moe-remote-mcp-spike",
    version: "0.0.0",
  });

  server.registerTool(
    "ping_moe",
    {
      title: "Ping M.O.E.",
      description: "Check whether the minimal M.O.E. relay endpoint is reachable.",
      inputSchema: {},
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async () => textResult({ ok: true, service: "moe-remote-mcp-spike" }),
  );

  server.registerTool(
    "moe_read_room",
    {
      title: "Read M.O.E. Room",
      description: "Read a bounded page of messages from the M.O.E. Room source selected by the host.",
      inputSchema: {
        roomId: z.string().min(1).max(128).default("moe-dev-room"),
        afterMessageId: z.string().min(1).max(128).optional(),
        limit: z.number().int().min(1).max(30).default(30),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ roomId, afterMessageId, limit }) => {
      try {
        const result = await roomSource.read({ roomId, afterMessageId, limit });
        return result.ok
          ? textResult(result)
          : errorResult(result.code, result.message);
      } catch {
        return errorResult(
          roomSource.unavailableCode,
          roomSource.unavailableMessage,
        );
      }
    },
  );

  server.registerTool(
    "moe_status",
    {
      title: "M.O.E. Status",
      description: "Read the current non-sensitive status of the M.O.E. relay spike.",
      inputSchema: {},
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async () => {
      const sourceStatus = await roomSource.status();
      return textResult({
        ok: true,
        phase: "remote-mcp-local-spike",
        relay: sourceStatus.relay,
        desktop: sourceStatus.desktop,
        capabilities: ["ping_moe", "moe_status", "moe_read_room"],
      });
    },
  );

  return server;
}

const allowedHosts = ["127.0.0.1", "localhost", "[::1]"];
if (additionalAllowedHost) {
  allowedHosts.push(additionalAllowedHost);
}

const app = createMcpExpressApp({ host, allowedHosts });

app.get("/healthz", (_request, response) => {
  response.json({ ok: true, service: "moe-remote-mcp-spike" });
});

app.post(mcpPath, async (request, response) => {
  const server = createServer();
  const transport = new StreamableHTTPServerTransport({
    sessionIdGenerator: undefined,
    enableJsonResponse: true,
  });

  response.on("close", () => {
    void transport.close();
    void server.close();
  });

  try {
    await server.connect(transport);
    await transport.handleRequest(request, response, request.body);
  } catch (error) {
    console.error("MCP request failed", error);
    if (!response.headersSent) {
      response.status(500).json({
        jsonrpc: "2.0",
        error: { code: -32603, message: "Internal server error" },
        id: null,
      });
    }
  }
});

for (const method of ["get", "delete"]) {
  app[method](mcpPath, (_request, response) => {
    response.status(405).json({
      jsonrpc: "2.0",
      error: { code: -32000, message: "Method not allowed" },
      id: null,
    });
  });
}

const httpServer = app.listen(requestedPort, host, () => {
  const address = httpServer.address();
  const port = typeof address === "object" && address ? address.port : requestedPort;
  console.log(`M.O.E. remote MCP listening on http://${host}:${port}${mcpPath}`);
});

function shutdown() {
  httpServer.close((error) => {
    if (error) {
      console.error("Failed to close HTTP server", error);
      process.exitCode = 1;
    }
  });
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
