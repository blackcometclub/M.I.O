import { request as httpRequest } from "node:http";
import { loadRoomSnapshot, readRoom } from "../remote-mcp/room-snapshot.mjs";
import { createNdjsonParser, encodeFrame } from "./ndjson.mjs";

export async function connectDesktop({
  relayUrl,
  deviceToken,
  deviceCredential = deviceToken,
  deviceId = "moe-desktop-probe",
  snapshotFile,
}) {
  const endpoint = new URL("/desktop-link", relayUrl);
  let responseStream;
  let closed = false;
  let resolveHandshake;
  let rejectHandshake;
  const handshake = new Promise((resolve, reject) => {
    resolveHandshake = resolve;
    rejectHandshake = reject;
  });

  const request = httpRequest(
    endpoint,
    {
      method: "POST",
      headers: {
        authorization: `Bearer ${deviceCredential}`,
        "content-type": "application/x-ndjson",
      },
    },
    (response) => {
      responseStream = response;
      if (response.statusCode !== 200) {
        rejectHandshake(new Error(`Relay rejected Desktop link with HTTP ${response.statusCode}`));
        request.destroy();
        return;
      }

      response.setEncoding("utf8");
      const parseFrame = createNdjsonParser((frame) => {
        if (frame?.type === "hello_ack") {
          resolveHandshake({ connectionId: frame.connectionId });
          return;
        }
        if (
          frame?.type !== "request" ||
          frame?.method !== "moe_read_room" ||
          typeof frame.requestId !== "string"
        ) {
          return;
        }

        void (async () => {
          try {
            const snapshot = await loadRoomSnapshot(snapshotFile);
            const result = readRoom(snapshot, frame.params);
            request.write(
              encodeFrame({
                type: "response",
                requestId: frame.requestId,
                result,
              }),
            );
          } catch {
            request.write(
              encodeFrame({
                type: "response",
                requestId: frame.requestId,
                error: {
                  code: "desktop_snapshot_unavailable",
                  message: "The host-selected Desktop snapshot is unavailable.",
                },
              }),
            );
          }
        })();
      });

      response.on("data", parseFrame);
      response.on("error", rejectHandshake);
      response.on("aborted", () => {
        rejectHandshake(new Error("Relay aborted the Desktop handshake"));
      });
    },
  );

  request.on("error", (error) => {
    if (!closed) {
      rejectHandshake(error);
    }
  });
  request.flushHeaders();
  request.write(
    encodeFrame({
      type: "hello",
      deviceId,
      protocolVersion: "0.1.0",
      capabilities: ["moe_read_room"],
    }),
  );

  const connected = await handshake;
  return {
    ...connected,
    async close() {
      closed = true;
      responseStream?.destroy();
      request.destroy();
      await new Promise((resolve) => setImmediate(resolve));
    },
  };
}
