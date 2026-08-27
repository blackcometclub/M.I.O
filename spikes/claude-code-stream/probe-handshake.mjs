import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const marker = "MOE_CLAUDE_OK";
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const evidencePath = join(scriptDirectory, "evidence", "handshake-latest.json");

function resolveLauncher() {
  if (process.env.MOE_CLAUDE_BIN) {
    return { command: process.env.MOE_CLAUDE_BIN, source: "MOE_CLAUDE_BIN" };
  }

  const defaultWindowsPath = process.env.USERPROFILE
    ? join(process.env.USERPROFILE, ".local", "bin", "claude.exe")
    : null;
  if (defaultWindowsPath && existsSync(defaultWindowsPath)) {
    return { command: defaultWindowsPath, source: "windows-native-default" };
  }

  return { command: "claude", source: "PATH" };
}

function run(launcher, args, timeout = 30_000) {
  const result = spawnSync(launcher.command, args, {
    cwd: scriptDirectory,
    encoding: "utf8",
    timeout,
    windowsHide: true,
    maxBuffer: 10 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  return {
    exitCode: result.status,
    signal: result.signal,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

function parseJson(value) {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function parseJsonLines(value) {
  return value
    .split(/\r?\n/gu)
    .filter((line) => line.trim())
    .map((line) => parseJson(line))
    .filter(Boolean);
}

function countBy(values, selector) {
  const counts = new Map();
  for (const value of values) {
    const key = selector(value);
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return Object.fromEntries(
    [...counts.entries()].sort(([left], [right]) => left.localeCompare(right)),
  );
}

function sanitizeAuthentication(rawStatus) {
  const status = parseJson(rawStatus);
  if (!status) return null;
  return {
    loggedIn: status.loggedIn === true,
    authMethod: status.authMethod ?? null,
    apiProvider: status.apiProvider ?? null,
  };
}

function extractText(events) {
  const partialText = events
    .filter(
      (event) =>
        event.type === "stream_event" && event.event?.delta?.type === "text_delta",
    )
    .map((event) => event.event.delta.text)
    .join("");
  if (partialText) return partialText;

  return events
    .filter((event) => event.type === "assistant")
    .flatMap((event) => event.message?.content ?? [])
    .filter((content) => content.type === "text")
    .map((content) => content.text)
    .join("");
}

function writeEvidence(summary) {
  mkdirSync(dirname(evidencePath), { recursive: true });
  writeFileSync(evidencePath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function main() {
  const launcher = resolveLauncher();
  const startedAt = new Date();
  const startedAtMs = Date.now();
  const versionResult = run(launcher, ["--version"]);
  const authResult = run(launcher, ["auth", "status"]);
  const authentication = sanitizeAuthentication(authResult.stdout);
  const streamResult = run(
    launcher,
    [
      "-p",
      `Reply with exactly ${marker}. Do not add punctuation or explanation.`,
      "--model",
      "fable",
      "--output-format",
      "stream-json",
      "--verbose",
      "--include-partial-messages",
      "--tools",
      "",
      "--permission-mode",
      "dontAsk",
      "--safe-mode",
    ],
    180_000,
  );
  const events = parseJsonLines(streamResult.stdout);
  const init = events.find(
    (event) => event.type === "system" && event.subtype === "init",
  );
  const result = [...events].reverse().find((event) => event.type === "result");
  const finalText = extractText(events).trim();
  const errorCategory =
    events.find((event) => event.error)?.error ??
    (result?.api_error_status ? "api_error" : null);
  const accessBlocked =
    errorCategory === "oauth_org_not_allowed" ||
    result?.result?.includes("disabled Claude subscription access") === true;
  const markerObserved = finalText === marker || result?.result?.trim() === marker;
  const status = markerObserved && !result?.is_error
    ? "PASS"
    : accessBlocked
      ? "BLOCKED"
      : "FAIL";
  const summary = {
    probe: "claude-code-stream-handshake",
    status,
    startedAt: startedAt.toISOString(),
    elapsedMs: Date.now() - startedAtMs,
    launcherSource: launcher.source,
    cliVersion: versionResult.stdout.trim(),
    authentication,
    transport: "subprocess/stream-json",
    requestedModel: "fable",
    observedModel: init?.model ?? null,
    toolsExposed: init?.tools ?? null,
    permissionModeReported: init?.permissionMode ?? null,
    streamExitCode: streamResult.exitCode,
    eventCounts: countBy(events, (event) =>
      event.subtype ? `${event.type}/${event.subtype}` : event.type,
    ),
    markerObserved,
    resultIsError: result?.is_error ?? null,
    resultSubtype: result?.subtype ?? null,
    apiErrorStatus: result?.api_error_status ?? null,
    errorCategory,
    totalCostUsd: result?.total_cost_usd ?? null,
    manualActionRequired: accessBlocked
      ? "Enable Claude Code programmatic access for this account, or explicitly choose API-key authentication."
      : null,
  };

  writeEvidence(summary);
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  if (status !== "PASS") process.exitCode = accessBlocked ? 2 : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  }
}
