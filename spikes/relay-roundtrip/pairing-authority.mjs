import {
  createHmac,
  randomBytes,
  randomInt,
  timingSafeEqual,
} from "node:crypto";

const pairingAlphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

function validateDeviceId(deviceId) {
  if (
    typeof deviceId !== "string" ||
    deviceId.length < 1 ||
    deviceId.length > 128 ||
    !/^[a-z0-9][a-z0-9._-]*$/u.test(deviceId)
  ) {
    throw new Error("invalid_device_id");
  }
  return deviceId;
}

function generatePairingCode() {
  let code = "";
  for (let index = 0; index < 8; index += 1) {
    code += pairingAlphabet[randomInt(pairingAlphabet.length)];
  }
  return `${code.slice(0, 4)}-${code.slice(4)}`;
}

function normalizePairingCode(code) {
  return typeof code === "string"
    ? code.toUpperCase().replace(/[\s-]/gu, "")
    : "";
}

export function createPairingAuthority({
  legacyDeviceToken,
  now = () => Date.now(),
  defaultTtlMs = 60_000,
  maximumAttempts = 5,
} = {}) {
  const inMemoryHashKey = randomBytes(32);
  const pairingRecords = new Map();
  const credentialDeviceByHash = new Map();
  const credentialHashByDevice = new Map();
  const legacyHash = legacyDeviceToken ? hashSecret(legacyDeviceToken) : null;

  function hashSecret(secret) {
    return createHmac("sha256", inMemoryHashKey).update(secret).digest();
  }

  function hashesMatch(left, right) {
    return left.length === right.length && timingSafeEqual(left, right);
  }

  function removeDeviceCredential(deviceId) {
    const credentialHash = credentialHashByDevice.get(deviceId);
    if (!credentialHash) {
      return false;
    }
    credentialHashByDevice.delete(deviceId);
    credentialDeviceByHash.delete(credentialHash.toString("hex"));
    return true;
  }

  return {
    issuePairingCode(deviceId, { ttlMs = defaultTtlMs } = {}) {
      validateDeviceId(deviceId);
      if (!Number.isInteger(ttlMs) || ttlMs < 1 || ttlMs > 10 * 60_000) {
        throw new Error("invalid_pairing_ttl");
      }

      const pairingCode = generatePairingCode();
      const expiresAt = now() + ttlMs;
      pairingRecords.set(deviceId, {
        codeHash: hashSecret(normalizePairingCode(pairingCode)),
        expiresAt,
        attemptsRemaining: maximumAttempts,
        state: "pending",
      });
      return {
        pairingCode,
        expiresAt: new Date(expiresAt).toISOString(),
        attempts: maximumAttempts,
      };
    },

    pair({ deviceId, pairingCode }) {
      try {
        validateDeviceId(deviceId);
      } catch {
        return { ok: false, status: 400, code: "invalid_device_id" };
      }

      const record = pairingRecords.get(deviceId);
      if (!record) {
        return { ok: false, status: 401, code: "pairing_code_invalid" };
      }
      if (record.state === "used") {
        return { ok: false, status: 409, code: "pairing_code_used" };
      }
      if (record.state === "locked") {
        return { ok: false, status: 429, code: "pairing_code_locked" };
      }
      if (record.state === "expired" || now() > record.expiresAt) {
        record.state = "expired";
        return { ok: false, status: 410, code: "pairing_code_expired" };
      }

      const candidateHash = hashSecret(normalizePairingCode(pairingCode));
      if (!hashesMatch(record.codeHash, candidateHash)) {
        record.attemptsRemaining -= 1;
        if (record.attemptsRemaining <= 0) {
          record.state = "locked";
          return { ok: false, status: 429, code: "pairing_code_locked" };
        }
        return {
          ok: false,
          status: 401,
          code: "pairing_code_invalid",
          attemptsRemaining: record.attemptsRemaining,
        };
      }

      record.state = "used";
      const deviceCredential = randomBytes(32).toString("base64url");
      const credentialHash = hashSecret(deviceCredential);
      removeDeviceCredential(deviceId);
      credentialHashByDevice.set(deviceId, credentialHash);
      credentialDeviceByHash.set(credentialHash.toString("hex"), deviceId);
      return {
        ok: true,
        status: 200,
        deviceId,
        deviceCredential,
      };
    },

    authenticate(deviceCredential) {
      if (typeof deviceCredential !== "string" || deviceCredential.length < 32) {
        return null;
      }
      const candidateHash = hashSecret(deviceCredential);
      if (legacyHash && hashesMatch(legacyHash, candidateHash)) {
        return { deviceId: "moe-desktop-probe", kind: "legacy-probe-token" };
      }
      const deviceId = credentialDeviceByHash.get(candidateHash.toString("hex"));
      return deviceId ? { deviceId, kind: "paired-device" } : null;
    },

    revoke(deviceId) {
      validateDeviceId(deviceId);
      return removeDeviceCredential(deviceId);
    },

    getSecurityState() {
      return {
        pairingRecordCount: pairingRecords.size,
        pairedDeviceCount: credentialHashByDevice.size,
        rawPairingCodesStored: false,
        rawDeviceCredentialsStored: false,
        secretPersistence: "memory-only",
      };
    },
  };
}
