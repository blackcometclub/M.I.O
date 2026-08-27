import assert from "node:assert/strict";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { connectDesktop } from "./desktop-link.mjs";
import { createRelay } from "./relay.mjs";

const runtimeDirectory = new URL("./runtime/", import.meta.url);
const runtimeSnapshot = new URL("./runtime/pairing-room.json", import.meta.url);
const fixtureSnapshot = new URL("../remote-mcp/fixtures/room-snapshot.json", import.meta.url);
const pairedDeviceId = "moe-desktop-paired";

async function postPair(baseUrl, body) {
  const response = await fetch(new URL("/pair", baseUrl), {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return { status: response.status, body: await response.json() };
}

async function requestJson(url, body) {
  const response = await fetch(url, {
    method: body ? "POST" : "GET",
    headers: body ? { "content-type": "application/json" } : undefined,
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
  throw new Error("Timed out waiting for pairing probe state");
}

await rm(runtimeDirectory, { recursive: true, force: true });
await mkdir(runtimeDirectory, { recursive: true });
const snapshot = JSON.parse(await readFile(fixtureSnapshot, "utf8"));
const createdAt = new Date().toISOString();
snapshot.generatedAt = createdAt;
snapshot.rooms[0].messages.push({
  id: "paired-device-message",
  roomId: "moe-dev-room",
  authorId: "codex",
  recipients: ["owner", "claude-web"],
  body: "DEVICE_PAIRING_ROOM_OK",
  createdAt,
  artifactIds: [],
});
await writeFile(runtimeSnapshot, JSON.stringify(snapshot, null, 2), "utf8");

const relay = await createRelay({ requestTimeoutMs: 1_000 });
let desktop;

try {
  const issued = relay.issuePairingCode(pairedDeviceId);
  assert.match(issued.pairingCode, /^[A-Z2-9]{4}-[A-Z2-9]{4}$/u);

  const wrongCode = await postPair(relay.baseUrl, {
    deviceId: pairedDeviceId,
    pairingCode: "AAAA-AAAA",
  });
  assert.equal(wrongCode.status, 401);
  assert.equal(wrongCode.body.code, "pairing_code_invalid");
  assert.equal(wrongCode.body.attemptsRemaining, 4);

  const paired = await postPair(relay.baseUrl, {
    deviceId: pairedDeviceId,
    pairingCode: issued.pairingCode,
  });
  assert.equal(paired.status, 200);
  assert.equal(paired.body.ok, true);
  assert.equal(typeof paired.body.deviceCredential, "string");
  assert.ok(paired.body.deviceCredential.length >= 32);

  const reused = await postPair(relay.baseUrl, {
    deviceId: pairedDeviceId,
    pairingCode: issued.pairingCode,
  });
  assert.equal(reused.status, 409);
  assert.equal(reused.body.code, "pairing_code_used");

  await assert.rejects(
    connectDesktop({
      relayUrl: relay.baseUrl,
      deviceCredential: paired.body.deviceCredential,
      deviceId: "different-device-id",
      snapshotFile: runtimeSnapshot,
    }),
    /aborted|socket hang up/u,
  );
  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "offline";
  });

  desktop = await connectDesktop({
    relayUrl: relay.baseUrl,
    deviceCredential: paired.body.deviceCredential,
    deviceId: pairedDeviceId,
    snapshotFile: runtimeSnapshot,
  });
  const pairedStatus = await requestJson(`${relay.baseUrl}/status`);
  assert.equal(pairedStatus.body.desktop, "connected");
  assert.equal(pairedStatus.body.desktopAuthentication, "paired-device");
  assert.equal(pairedStatus.body.pairedDeviceCount, 1);

  const room = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    roomId: "moe-dev-room",
    afterMessageId: "welcome-3",
    limit: 1,
  });
  assert.equal(room.status, 200);
  assert.equal(room.body.room.messages[0].body, "DEVICE_PAIRING_ROOM_OK");

  assert.equal(relay.revokeDevice(pairedDeviceId), true);
  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "offline";
  });
  desktop = null;

  await assert.rejects(
    connectDesktop({
      relayUrl: relay.baseUrl,
      deviceCredential: paired.body.deviceCredential,
      deviceId: pairedDeviceId,
      snapshotFile: runtimeSnapshot,
    }),
    /HTTP 401/u,
  );

  const expired = relay.issuePairingCode("expired-device", { ttlMs: 10 });
  await new Promise((resolve) => setTimeout(resolve, 20));
  const expiredResult = await postPair(relay.baseUrl, {
    deviceId: "expired-device",
    pairingCode: expired.pairingCode,
  });
  assert.equal(expiredResult.status, 410);
  assert.equal(expiredResult.body.code, "pairing_code_expired");

  const locked = relay.issuePairingCode("locked-device");
  let lockedResult;
  for (let attempt = 0; attempt < 5; attempt += 1) {
    lockedResult = await postPair(relay.baseUrl, {
      deviceId: "locked-device",
      pairingCode: "BBBB-BBBB",
    });
  }
  assert.equal(lockedResult.status, 429);
  assert.equal(lockedResult.body.code, "pairing_code_locked");
  const correctAfterLock = await postPair(relay.baseUrl, {
    deviceId: "locked-device",
    pairingCode: locked.pairingCode,
  });
  assert.equal(correctAfterLock.status, 429);
  assert.equal(correctAfterLock.body.code, "pairing_code_locked");

  const reissued = relay.issuePairingCode(pairedDeviceId);
  const repaired = await postPair(relay.baseUrl, {
    deviceId: pairedDeviceId,
    pairingCode: reissued.pairingCode,
  });
  assert.equal(repaired.status, 200);
  desktop = await connectDesktop({
    relayUrl: relay.baseUrl,
    deviceCredential: repaired.body.deviceCredential,
    deviceId: pairedDeviceId,
    snapshotFile: runtimeSnapshot,
  });
  const recoveredRoom = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    roomId: "moe-dev-room",
    afterMessageId: "welcome-3",
    limit: 1,
  });
  assert.equal(recoveredRoom.body.room.messages[0].body, "DEVICE_PAIRING_ROOM_OK");

  const securityState = relay.getSecurityState();
  assert.equal(securityState.rawPairingCodesStored, false);
  assert.equal(securityState.rawDeviceCredentialsStored, false);
  assert.equal(securityState.secretPersistence, "memory-only");

  console.log(
    JSON.stringify(
      {
        result: "PASS",
        pairing: {
          codeShape: "XXXX-XXXX",
          wrongAttemptDecremented: true,
          singleUse: true,
          expiry: "PASS",
          attemptLock: "PASS",
        },
        credential: {
          pairedDesktopConnection: "PASS",
          credentialBoundToDeviceId: true,
          revocationDisconnectedDevice: true,
          revokedCredentialRejected: true,
          rePairingAfterRevocation: "PASS",
        },
        roomMarker: recoveredRoom.body.room.messages[0].body,
        storage: securityState,
        publicNetworkUsed: false,
      },
      null,
      2,
    ),
  );
} finally {
  await desktop?.close();
  await relay.close();
  await rm(runtimeDirectory, { recursive: true, force: true });
}
