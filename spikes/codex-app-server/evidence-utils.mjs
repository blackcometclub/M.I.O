import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const redactedKeyPattern = /(?:(?:thread|turn|item|session)id|(?:author|display)name)$/iu;
const redactedKeys = new Set(["serverUserAgent"]);

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function sanitizeString(value, repositoryRoot) {
  if (!repositoryRoot) return value;

  const rootPattern = new RegExp(escapeRegExp(repositoryRoot), "giu");
  const slashRootPattern = new RegExp(
    escapeRegExp(repositoryRoot.replaceAll("\\", "/")),
    "giu",
  );
  return value
    .replace(rootPattern, "<repository-root>")
    .replace(slashRootPattern, "<repository-root>");
}

export function sanitizeEvidence(value, repositoryRoot) {
  if (Array.isArray(value)) {
    return value.map((entry) => sanitizeEvidence(entry, repositoryRoot));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [
        key,
        redactedKeys.has(key) || redactedKeyPattern.test(key)
          ? "<redacted>"
          : sanitizeEvidence(entry, repositoryRoot),
      ]),
    );
  }
  if (typeof value === "string") return sanitizeString(value, repositoryRoot);
  return value;
}

export function writePublicEvidence(path, summary, repositoryRoot) {
  const publicSummary = sanitizeEvidence(summary, repositoryRoot);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(publicSummary, null, 2)}\n`, "utf8");
  return publicSummary;
}
