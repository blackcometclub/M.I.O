import assert from "node:assert/strict";
import { stat, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { createRelay } from "./relay.mjs";

const executable = fileURLToPath(
  new URL(
    process.platform === "win32"
      ? "../../target/debug/moe-rust-relay-pairing-spike.exe"
      : "../../target/debug/moe-rust-relay-pairing-spike",
    import.meta.url,
  ),
);
const evidenceFile = new URL(
  "./evidence/rust-product-connection-latest.json",
  import.meta.url,
);

async function startRust(environment, forbiddenSecret) {
  await stat(executable);
  const child = spawn(executable, [], {
    env: { ...process.env, ...environment },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  const stdout = [];
  const stderr = [];
  const completion = new Promise((resolve, reject) => {
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.once("error", reject);
    child.once("close", (code) => {
      const result = {
        code,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      };
      if (
        forbiddenSecret &&
        (result.stdout.includes(forbiddenSecret) || result.stderr.includes(forbiddenSecret))
      ) {
        reject(new Error("Rust connection probe output leaked the pairing code"));
        return;
      }
      resolve(result);
    });
  });
  return { child, completion };
}

async function runRust(environment) {
  const process = await startRust(environment, "");
  const result = await process.completion;
  if (result.code !== 0) {
    throw new Error(`Rust cleanup failed with exit code ${result.code}: ${result.stderr}`);
  }
  return result.stdout.trim();
}

async function requestJson(url, body) {
  const response = await fetch(url, {
    method: body ? "POST" : "GET",
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  return { status: response.status, body: await response.json() };
}

async function waitFor(predicate, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error("Timed out waiting for Rust product connection state");
}

const nonce = `${process.pid}-${Date.now().toString(16)}`;
const accountId = `rustconnect-${nonce}`;
const deviceId = `moe-desktop-connect-${nonce}`;
const relay = await createRelay({ requestTimeoutMs: 3_000 });
let pairingCode = "";
let rustProcess;
let rustCompleted = false;
let primaryError;
let evidence;

try {
  const issued = relay.issuePairingCode(deviceId);
  pairingCode = issued.pairingCode;
  rustProcess = await startRust(
    {
      MOE_RELAY_PAIRING_BASE_URL: relay.baseUrl,
      MOE_RELAY_PAIRING_ACCOUNT: accountId,
      MOE_RELAY_PAIRING_DEVICE: deviceId,
      MOE_RELAY_PAIRING_CODE: pairingCode,
      MOE_RELAY_CONNECTION_INTEGRATION: "1",
    },
    pairingCode,
  );

  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "connected";
  });
  const connectedStatus = await requestJson(`${relay.baseUrl}/status`);
  assert.equal(connectedStatus.body.desktopAuthentication, "paired-device");

  const room = await requestJson(`${relay.baseUrl}/mcp/read-room`, {
    roomId: "moe-dev-room",
    limit: 1,
  });
  assert.equal(room.status, 200);
  assert.equal(room.body.room.messages[0].body, "RUST_PRODUCT_CONNECTION_OK");

  const rustResult = await rustProcess.completion;
  rustCompleted = true;
  if (rustResult.code !== 0) {
    throw new Error(
      `Rust connection probe failed with exit code ${rustResult.code}: ${rustResult.stderr}`,
    );
  }
  const rustEvidence = JSON.parse(rustResult.stdout);
  assert.equal(rustEvidence.result, "PASS");
  assert.equal(rustEvidence.credentialLoadedByProductManager, true);
  assert.equal(rustEvidence.authorizationWrittenFromBorrowedSecret, true);
  assert.equal(rustEvidence.authenticatedHello, true);
  assert.equal(rustEvidence.roomRead, "RUST_PRODUCT_CONNECTION_OK");
  assert.equal(rustEvidence.serviceReportedConnected, true);
  assert.equal(rustEvidence.serviceReturnedOfflineOnDrop, true);
  assert.equal(rustEvidence.automaticReconnect, true);
  assert.equal(rustEvidence.retryTimerElapsed, true);
  assert.equal(rustEvidence.runtimeGenerationAdvanced, true);
  assert.equal(rustEvidence.cancellationInterruptedSocket, true);
  assert.equal(rustEvidence.orchestratorReturnedOfflineOnStop, true);
  assert.equal(rustEvidence.probeCredentialRemoved, true);
  assert.equal(rustEvidence.connectionRejectedAfterDelete, true);
  assert.equal(rustEvidence.serviceReportedSafeErrorAfterDelete, true);

  await waitFor(async () => {
    const status = await requestJson(`${relay.baseUrl}/status`);
    return status.body.desktop === "offline";
  });

  evidence = {
    result: "PASS",
    date: new Date().toISOString(),
    path: "Windows Credential Manager -> moe-relay-client -> Desktop orchestrator -> Rust chunked NDJSON transport -> Node loopback Relay -> disconnect -> automatic retry",
    pairingCodeOnCommandLine: false,
    pairingCodeInOutput: false,
    deviceCredentialEnteredWebView: false,
    deviceCredentialReturnedToOrchestrator: false,
    credentialLoadedByProductManager: true,
    authorizationWrittenFromBorrowedSecret: true,
    authenticatedHello: true,
    serviceReportedConnected: true,
    serviceReturnedOfflineOnDrop: true,
    desktopAuthentication: "paired-device",
    roomMarker: room.body.room.messages[0].body,
    disconnectObserved: true,
    automaticReconnect: true,
    retryTimerElapsed: true,
    runtimeGenerationAdvanced: true,
    cancellationInterruptedSocket: true,
    orchestratorReturnedOfflineOnStop: true,
    probeCredentialRemoved: true,
    connectionRejectedAfterDelete: true,
    serviceReportedSafeErrorAfterDelete: true,
    publicNetworkUsed: false,
  };
} catch (error) {
  primaryError = error;
} finally {
  pairingCode = "";
  if (rustProcess && !rustCompleted) {
    rustProcess.child.kill();
    try {
      await rustProcess.completion;
    } catch (processError) {
      primaryError ??= processError;
    }
  }
  try {
    const cleanupOutput = await runRust({
      MOE_RELAY_PAIRING_ACCOUNT: accountId,
      MOE_RELAY_PAIRING_CLEANUP_ONLY: "1",
    });
    assert.equal(cleanupOutput, "CLEANUP_OK");
  } catch (cleanupError) {
    primaryError = primaryError
      ? new AggregateError(
          [primaryError, cleanupError],
          "Connection probe and credential cleanup both failed",
        )
      : cleanupError;
  }
  relay.revokeDevice(deviceId);
  await relay.close();
}

if (primaryError) {
  throw primaryError;
}

await writeFile(evidenceFile, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
console.log(JSON.stringify(evidence, null, 2));
