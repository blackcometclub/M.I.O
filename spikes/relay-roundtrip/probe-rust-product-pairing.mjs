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
  "./evidence/rust-product-pairing-latest.json",
  import.meta.url,
);

async function runRustProcess(environment, forbiddenSecret) {
  await stat(executable);
  const result = await new Promise((resolve, reject) => {
    const child = spawn(executable, [], {
      env: { ...process.env, ...environment },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    const stdout = [];
    const stderr = [];
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.once("error", reject);
    child.once("close", (code) =>
      resolve({
        code,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      }),
    );
  });
  if (
    forbiddenSecret &&
    (result.stdout.includes(forbiddenSecret) || result.stderr.includes(forbiddenSecret))
  ) {
    throw new Error("Rust pairing probe output leaked the pairing code");
  }
  return result;
}

async function runRust(environment, forbiddenSecret) {
  const result = await runRustProcess(environment, forbiddenSecret);
  if (result.code !== 0) {
    throw new Error(`Rust pairing probe failed with exit code ${result.code}: ${result.stderr}`);
  }
  return result.stdout.trim();
}

const nonce = `${process.pid}-${Date.now().toString(16)}`;
const accountId = `rustpair-${nonce}`;
const deviceId = `moe-desktop-rust-${nonce}`;
const relay = await createRelay({});
let pairingCode = "";
let primaryError;
let evidence;

try {
  const issued = relay.issuePairingCode(deviceId);
  pairingCode = issued.pairingCode;
  const rejected = await runRustProcess(
    {
      MOE_RELAY_PAIRING_BASE_URL: relay.baseUrl,
      MOE_RELAY_PAIRING_ACCOUNT: accountId,
      MOE_RELAY_PAIRING_DEVICE: deviceId,
      MOE_RELAY_PAIRING_CODE: "BBBB-BBBB",
    },
    pairingCode,
  );
  assert.notEqual(rejected.code, 0);
  assert.match(rejected.stderr, /InvalidCode|relay pairing code was rejected/u);

  const output = await runRust(
    {
      MOE_RELAY_PAIRING_BASE_URL: relay.baseUrl,
      MOE_RELAY_PAIRING_ACCOUNT: accountId,
      MOE_RELAY_PAIRING_DEVICE: deviceId,
      MOE_RELAY_PAIRING_CODE: pairingCode,
    },
    pairingCode,
  );
  const rustResult = JSON.parse(output);
  assert.equal(rustResult.result, "PASS");
  assert.equal(rustResult.pairingResponseHandledInRust, true);
  assert.equal(rustResult.credentialEnteredWebView, false);
  assert.equal(rustResult.credentialStoredInWindowsCredentialManager, true);
  assert.equal(rustResult.probeCredentialRemoved, true);

  const relaySecurity = relay.getSecurityState();
  assert.equal(relaySecurity.pairedDeviceCount, 1);
  assert.equal(relaySecurity.rawPairingCodesStored, false);
  assert.equal(relaySecurity.rawDeviceCredentialsStored, false);

  evidence = {
    result: "PASS",
    date: new Date().toISOString(),
    path: "Node loopback Relay /pair -> Rust pairing transport -> moe-relay-client -> Windows Credential Manager",
    pairingCodeOnCommandLine: false,
    pairingCodeInOutput: false,
    rejectedCodeHandled: true,
    deviceCredentialEnteredWebView: false,
    deviceCredentialReturnedToOrchestrator: false,
    credentialStoredByProductManager: true,
    probeCredentialRemoved: true,
    relayRawPairingCodesStored: false,
    relayRawDeviceCredentialsStored: false,
    publicNetworkUsed: false,
  };
} catch (error) {
  primaryError = error;
} finally {
  pairingCode = "";
  try {
    const cleanupOutput = await runRust(
      {
        MOE_RELAY_PAIRING_ACCOUNT: accountId,
        MOE_RELAY_PAIRING_CLEANUP_ONLY: "1",
      },
      "",
    );
    assert.equal(cleanupOutput, "CLEANUP_OK");
  } catch (cleanupError) {
    primaryError = primaryError
      ? new AggregateError(
          [primaryError, cleanupError],
          "Pairing probe and credential cleanup both failed",
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
