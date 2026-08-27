import {
  loadRoomSnapshot,
  readRoom as readSnapshotRoom,
} from "./room-snapshot.mjs";

const relayTimeoutMs = 2_000;

function validateLocalRelayBaseUrl(configuredUrl) {
  if (!configuredUrl) {
    return null;
  }

  const url = new URL(configuredUrl);
  if (
    url.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) ||
    (url.pathname !== "/" && url.pathname !== "") ||
    url.username ||
    url.password ||
    url.search ||
    url.hash
  ) {
    throw new Error(
      "MOE_RELAY_BASE_URL must be an uncredentialed loopback HTTP origin for this spike",
    );
  }

  return url;
}

async function fetchRelayJson(url, init) {
  const response = await fetch(url, {
    ...init,
    signal: AbortSignal.timeout(relayTimeoutMs),
  });
  const payload = await response.json();
  return { response, payload };
}

export function createRoomSource({ relayBaseUrl, snapshotFile }) {
  const relayUrl = validateLocalRelayBaseUrl(relayBaseUrl);
  if (!relayUrl) {
    return {
      kind: "snapshot",
      unavailableCode: "room_snapshot_unavailable",
      unavailableMessage: "The host-selected room snapshot is unavailable or invalid.",
      async read(params) {
        const snapshot = await loadRoomSnapshot(snapshotFile);
        return readSnapshotRoom(snapshot, params);
      },
      async status() {
        return { relay: "local-only", desktop: "not-connected" };
      },
    };
  }

  return {
    kind: "relay",
    unavailableCode: "relay_unavailable",
    unavailableMessage: "The local Relay is unavailable.",
    async read(params) {
      const { response, payload } = await fetchRelayJson(
        new URL("/mcp/read-room", relayUrl),
        {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(params),
        },
      );

      if (!response.ok) {
        return {
          ok: false,
          code: payload?.code ?? "relay_request_failed",
          message: "The Relay could not complete the Room request.",
        };
      }
      return payload;
    },
    async status() {
      try {
        const { response, payload } = await fetchRelayJson(
          new URL("/status", relayUrl),
        );
        if (!response.ok) {
          return { relay: "unavailable", desktop: "unknown" };
        }
        return {
          relay: "local-outbound-link",
          desktop: payload.desktop,
        };
      } catch {
        return { relay: "unavailable", desktop: "unknown" };
      }
    },
  };
}
