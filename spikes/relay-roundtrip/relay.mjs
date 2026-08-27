import { createServer } from "node:http";
import { randomUUID } from "node:crypto";
import { createNdjsonParser, encodeFrame } from "./ndjson.mjs";
import { createPairingAuthority } from "./pairing-authority.mjs";

const maximumRequestBytes = 8 * 1024;

function sendJson(response, statusCode, payload) {
  response.writeHead(statusCode, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
  });
  response.end(JSON.stringify(payload));
}

async function readJson(request) {
  const chunks = [];
  let size = 0;

  for await (const chunk of request) {
    size += chunk.length;
    if (size > maximumRequestBytes) {
      throw new Error("request_too_large");
    }
    chunks.push(chunk);
  }

  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

function validateReadRoomInput(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("invalid_request");
  }

  const allowedKeys = new Set(["roomId", "afterMessageId", "limit"]);
  if (Object.keys(payload).some((key) => !allowedKeys.has(key))) {
    throw new Error("invalid_request");
  }

  const roomId = payload.roomId ?? "moe-dev-room";
  const limit = payload.limit ?? 30;
  if (
    typeof roomId !== "string" ||
    roomId.length < 1 ||
    roomId.length > 128 ||
    (payload.afterMessageId !== undefined &&
      (typeof payload.afterMessageId !== "string" ||
        payload.afterMessageId.length < 1 ||
        payload.afterMessageId.length > 128)) ||
    !Number.isInteger(limit) ||
    limit < 1 ||
    limit > 30
  ) {
    throw new Error("invalid_request");
  }

  return {
    roomId,
    afterMessageId: payload.afterMessageId,
    limit,
  };
}

export async function createRelay({
  deviceToken,
  host = "127.0.0.1",
  port = 0,
  requestTimeoutMs = 2_000,
}) {
  if (host !== "127.0.0.1") {
    throw new Error("This spike must remain bound to 127.0.0.1");
  }
  if (
    deviceToken !== undefined &&
    (typeof deviceToken !== "string" || deviceToken.length < 32)
  ) {
    throw new Error("The optional legacy probe token must contain at least 32 characters");
  }

  let activeDesktop = null;
  const pendingRequests = new Map();
  const pairingAuthority = createPairingAuthority({
    legacyDeviceToken: deviceToken,
  });

  function rejectPending(code, message) {
    for (const pending of pendingRequests.values()) {
      clearTimeout(pending.timeout);
      pending.reject(Object.assign(new Error(message), { code }));
    }
    pendingRequests.clear();
  }

  function disconnectDesktop(connection) {
    if (activeDesktop !== connection) {
      return;
    }
    activeDesktop = null;
    rejectPending("desktop_disconnected", "Desktop disconnected before replying.");
  }

  function acceptDesktopLink(request, response) {
    const authorization = request.headers.authorization;
    const credential = authorization?.startsWith("Bearer ")
      ? authorization.slice("Bearer ".length)
      : "";
    const authentication = pairingAuthority.authenticate(credential);
    if (!authentication) {
      sendJson(response, 401, { ok: false, code: "device_unauthorized" });
      return;
    }
    if (activeDesktop) {
      sendJson(response, 409, { ok: false, code: "device_already_connected" });
      return;
    }

    response.writeHead(200, {
      "content-type": "application/x-ndjson; charset=utf-8",
      "cache-control": "no-store",
      connection: "keep-alive",
    });

    const connection = {
      id: randomUUID(),
      authenticationKind: authentication.kind,
      deviceId: authentication.deviceId,
      ready: false,
      request,
      response,
    };
    activeDesktop = connection;

    const parseFrame = createNdjsonParser((frame) => {
      if (!connection.ready) {
        if (
          frame?.type !== "hello" ||
          frame?.deviceId !== connection.deviceId ||
          !Array.isArray(frame?.capabilities) ||
          !frame.capabilities.includes("moe_read_room")
        ) {
          response.destroy(new Error("invalid_desktop_hello"));
          return;
        }

        connection.ready = true;
        response.write(
          encodeFrame({
            type: "hello_ack",
            connectionId: connection.id,
          }),
        );
        return;
      }

      if (frame?.type !== "response" || typeof frame.requestId !== "string") {
        return;
      }

      const pending = pendingRequests.get(frame.requestId);
      if (!pending) {
        return;
      }

      clearTimeout(pending.timeout);
      pendingRequests.delete(frame.requestId);
      if (frame.error) {
        pending.reject(
          Object.assign(new Error(frame.error.message ?? "Desktop request failed."), {
            code: frame.error.code ?? "desktop_request_failed",
          }),
        );
      } else {
        pending.resolve(frame.result);
      }
    });

    request.setEncoding("utf8");
    request.on("data", parseFrame);
    request.on("error", () => disconnectDesktop(connection));
    request.on("close", () => disconnectDesktop(connection));
    response.on("close", () => disconnectDesktop(connection));
  }

  async function pairDevice(request, response) {
    let payload;
    try {
      payload = await readJson(request);
    } catch (error) {
      const statusCode = error.message === "request_too_large" ? 413 : 400;
      sendJson(response, statusCode, { ok: false, code: error.message });
      return;
    }

    if (
      !payload ||
      typeof payload !== "object" ||
      Array.isArray(payload) ||
      Object.keys(payload).some(
        (key) => !["deviceId", "pairingCode"].includes(key),
      )
    ) {
      sendJson(response, 400, { ok: false, code: "invalid_request" });
      return;
    }

    const result = pairingAuthority.pair(payload);
    if (!result.ok) {
      const { status, ...errorPayload } = result;
      sendJson(response, status, errorPayload);
      return;
    }

    sendJson(response, 200, {
      ok: true,
      deviceId: result.deviceId,
      deviceCredential: result.deviceCredential,
    });
  }

  async function forwardReadRoom(request, response) {
    if (!activeDesktop?.ready) {
      sendJson(response, 503, { ok: false, code: "desktop_offline" });
      return;
    }

    let params;
    try {
      params = validateReadRoomInput(await readJson(request));
    } catch (error) {
      const code = error.message === "request_too_large" ? 413 : 400;
      sendJson(response, code, { ok: false, code: error.message });
      return;
    }

    const requestId = randomUUID();
    const resultPromise = new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        pendingRequests.delete(requestId);
        reject(Object.assign(new Error("Desktop response timed out."), { code: "desktop_timeout" }));
      }, requestTimeoutMs);
      pendingRequests.set(requestId, { resolve, reject, timeout });
    });

    activeDesktop.response.write(
      encodeFrame({
        type: "request",
        requestId,
        method: "moe_read_room",
        params,
      }),
    );

    try {
      const result = await resultPromise;
      sendJson(response, 200, result);
    } catch (error) {
      const statusCode = error.code === "desktop_timeout" ? 504 : 503;
      sendJson(response, statusCode, { ok: false, code: error.code });
    }
  }

  const server = createServer((request, response) => {
    const url = new URL(request.url, "http://127.0.0.1");
    if (request.method === "POST" && url.pathname === "/desktop-link") {
      acceptDesktopLink(request, response);
      return;
    }
    if (request.method === "POST" && url.pathname === "/pair") {
      void pairDevice(request, response);
      return;
    }
    if (request.method === "POST" && url.pathname === "/mcp/read-room") {
      void forwardReadRoom(request, response);
      return;
    }
    if (request.method === "GET" && url.pathname === "/status") {
      sendJson(response, 200, {
        ok: true,
        desktop: activeDesktop?.ready ? "connected" : "offline",
        desktopAuthentication: activeDesktop?.ready
          ? activeDesktop.authenticationKind
          : "none",
        pairedDeviceCount: pairingAuthority.getSecurityState().pairedDeviceCount,
        pendingRequestCount: pendingRequests.size,
        retainedRoomCount: 0,
      });
      return;
    }
    sendJson(response, 404, { ok: false, code: "not_found" });
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, host, resolve);
  });

  const address = server.address();
  const baseUrl = `http://${host}:${address.port}`;

  return {
    baseUrl,
    issuePairingCode(deviceId, options) {
      return pairingAuthority.issuePairingCode(deviceId, options);
    },
    revokeDevice(deviceId) {
      const revoked = pairingAuthority.revoke(deviceId);
      if (activeDesktop?.deviceId === deviceId) {
        activeDesktop.response.destroy();
        activeDesktop.request.destroy();
      }
      return revoked;
    },
    getSecurityState() {
      return pairingAuthority.getSecurityState();
    },
    async close() {
      activeDesktop?.response.destroy();
      activeDesktop?.request.destroy();
      rejectPending("relay_stopped", "Relay stopped.");
      await new Promise((resolve, reject) =>
        server.close((error) => (error ? reject(error) : resolve())),
      );
    },
  };
}
