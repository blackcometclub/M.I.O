import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  AppServerClient,
  collectAgentText,
  repositoryRoot,
  resolveLauncher,
} from "./probe-handshake.mjs";
import { writePublicEvidence } from "./evidence-utils.mjs";

const RESUME_MARKER = "MOE_RESUME_OK";
const TURN_TIMEOUT_MS = 180_000;
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const handshakeStatePath = join(
  repositoryRoot,
  ".moe",
  "probe-state",
  "codex-app-server-handshake.json",
);
const resumeEvidencePath = join(scriptDirectory, "evidence", "resume-latest.json");
const allThreadSourceKinds = [
  "cli",
  "vscode",
  "exec",
  "appServer",
  "subAgent",
  "subAgentReview",
  "subAgentCompact",
  "subAgentThreadSpawn",
  "subAgentOther",
  "unknown",
];

function readHandshakeState() {
  const state = JSON.parse(readFileSync(handshakeStatePath, "utf8"));
  if (state.status !== "PASS" || !state.threadId) {
    throw new Error("Run the PASS handshake probe before the resume probe.");
  }
  return state;
}

function threadContainsText(thread, expectedText) {
  return (thread.turns ?? []).some((turn) =>
    (turn.items ?? []).some(
      (item) => item.type === "agentMessage" && item.text.includes(expectedText),
    ),
  );
}

function writeEvidence(summary) {
  return writePublicEvidence(resumeEvidencePath, summary, repositoryRoot);
}

async function listThreads(client, targetThreadId) {
  const threads = [];
  let cursor = null;

  for (let page = 0; page < 10; page += 1) {
    const response = await client.request("thread/list", {
      cursor,
      limit: 100,
      sortKey: "updated_at",
      sortDirection: "desc",
      sourceKinds: allThreadSourceKinds,
      archived: false,
    });
    threads.push(...response.data);

    if (threads.some((thread) => thread.id === targetThreadId) || !response.nextCursor) {
      break;
    }
    cursor = response.nextCursor;
  }

  return threads;
}

async function main() {
  const handshakeState = readHandshakeState();
  const sourceThreadId = handshakeState.threadId;
  const launcher = resolveLauncher();
  const client = new AppServerClient(launcher);
  const startedAt = new Date();
  const startedAtMs = Date.now();

  client.start();

  try {
    const initialize = await client.request("initialize", {
      clientInfo: {
        name: "moe_phase0_probe",
        title: "M.O.E. Phase 0 Probe",
        version: "0.0.0",
      },
      capabilities: null,
    });
    client.notify("initialized", {});

    const listedThreads = await listThreads(client, sourceThreadId);
    const listFound = listedThreads.some((thread) => thread.id === sourceThreadId);
    if (!listFound) {
      throw new Error(`thread/list did not return ${sourceThreadId}.`);
    }

    const read = await client.request("thread/read", {
      threadId: sourceThreadId,
      includeTurns: true,
    });
    const readThreadIdMatches = read.thread.id === sourceThreadId;
    const originalMarkerInRead = threadContainsText(
      read.thread,
      handshakeState.finalText,
    );
    if (!readThreadIdMatches || !originalMarkerInRead) {
      throw new Error("thread/read did not restore the expected handshake history.");
    }

    const resumed = await client.request("thread/resume", {
      threadId: sourceThreadId,
      cwd: repositoryRoot,
      approvalPolicy: "never",
      sandbox: "read-only",
    });
    const resumeThreadIdMatches = resumed.thread.id === sourceThreadId;
    if (!resumeThreadIdMatches) {
      throw new Error("thread/resume returned a different thread id.");
    }

    const turnStart = await client.request("turn/start", {
      threadId: sourceThreadId,
      input: [
        {
          type: "text",
          text: `Reply with exactly ${RESUME_MARKER}. Do not inspect files or run tools.`,
          text_elements: [],
        },
      ],
    });
    const resumedTurnId = turnStart.turn.id;
    const completed = await client.waitForNotification(
      "turn/completed",
      (params) =>
        params?.threadId === sourceThreadId && params?.turn?.id === resumedTurnId,
      TURN_TIMEOUT_MS,
    );
    const finalText = collectAgentText(
      client.notifications,
      sourceThreadId,
      resumedTurnId,
    ).trim();
    const markerObserved = finalText.includes(RESUME_MARKER);
    const status =
      listFound &&
      readThreadIdMatches &&
      originalMarkerInRead &&
      resumeThreadIdMatches &&
      completed.turn.status === "completed" &&
      markerObserved
        ? "PASS"
        : "FAIL";
    const summary = {
      probe: "codex-app-server-restart-resume",
      status,
      startedAt: startedAt.toISOString(),
      elapsedMs: Date.now() - startedAtMs,
      launcherSource: launcher.source,
      transport: "stdio",
      serverUserAgent: initialize.userAgent ?? null,
      sourceHandshakeStartedAt: handshakeState.startedAt,
      sourceThreadId,
      listFound,
      listedThreadCount: listedThreads.length,
      readThreadIdMatches,
      readTurnCount: read.thread.turns.length,
      originalMarkerInRead,
      resumeThreadIdMatches,
      resumedTurnId,
      resumedTurnStatus: completed.turn.status,
      markerObserved,
      finalText,
      notificationCounts: Object.fromEntries(
        [...client.notificationCounts.entries()].sort(([a], [b]) => a.localeCompare(b)),
      ),
      serverRequestMethods: client.serverRequestMethods,
    };

    const publicSummary = writeEvidence(summary);
    process.stdout.write(`${JSON.stringify(publicSummary, null, 2)}\n`);
    if (status !== "PASS") process.exitCode = 1;
  } catch (error) {
    const stderrTail = client.stderrBuffer.trim();
    if (stderrTail) process.stderr.write(`${stderrTail}\n`);
    throw error;
  } finally {
    await client.stop();
  }
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error.message}\n`);
  process.exitCode = 1;
});
