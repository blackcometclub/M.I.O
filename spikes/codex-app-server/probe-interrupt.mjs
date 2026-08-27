import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  AppServerClient,
  collectAgentText,
  repositoryRoot,
  resolveLauncher,
} from "./probe-handshake.mjs";
import { writePublicEvidence } from "./evidence-utils.mjs";

const RECOVERY_MARKER = "MOE_AFTER_INTERRUPT_OK";
const TURN_TIMEOUT_MS = 180_000;
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const evidencePath = join(scriptDirectory, "evidence", "interrupt-latest.json");

function writeEvidence(summary) {
  return writePublicEvidence(evidencePath, summary, repositoryRoot);
}

async function main() {
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

    const threadStart = await client.request("thread/start", {
      cwd: repositoryRoot,
      approvalPolicy: "never",
      sandbox: "read-only",
      ephemeral: true,
    });
    const threadId = threadStart.thread.id;

    const longTurnStart = await client.request("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: "1から10000までの番号付き一覧を作り、各行に番号と異なる短い日本語文を書いてください。10000項目すべてが終わるまで続けてください。ツールは使わないでください。",
          text_elements: [],
        },
      ],
    });
    const interruptedTurnId = longTurnStart.turn.id;
    await client.waitForNotification(
      "turn/started",
      (params) => params?.threadId === threadId && params?.turn?.id === interruptedTurnId,
      TURN_TIMEOUT_MS,
    );

    const interruptRequestedAtMs = Date.now();
    await client.request("turn/interrupt", {
      threadId,
      turnId: interruptedTurnId,
    });
    const interrupted = await client.waitForNotification(
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === interruptedTurnId,
      TURN_TIMEOUT_MS,
    );
    const interruptCompletionElapsedMs = Date.now() - interruptRequestedAtMs;

    const recoveryTurnStart = await client.request("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: `Reply with exactly ${RECOVERY_MARKER}. Do not inspect files or run tools.`,
          text_elements: [],
        },
      ],
    });
    const recoveryTurnId = recoveryTurnStart.turn.id;
    const recovered = await client.waitForNotification(
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === recoveryTurnId,
      TURN_TIMEOUT_MS,
    );
    const recoveryFinalText = collectAgentText(
      client.notifications,
      threadId,
      recoveryTurnId,
    ).trim();
    const recoveryMarkerObserved = recoveryFinalText === RECOVERY_MARKER;
    const interruptedStatusObserved = interrupted.turn.status === "interrupted";
    const status =
      interruptedStatusObserved &&
      recovered.turn.status === "completed" &&
      recoveryMarkerObserved
        ? "PASS"
        : "FAIL";
    const summary = {
      probe: "codex-app-server-turn-interrupt",
      status,
      startedAt: startedAt.toISOString(),
      elapsedMs: Date.now() - startedAtMs,
      launcherSource: launcher.source,
      transport: "stdio",
      serverUserAgent: initialize.userAgent ?? null,
      threadEphemeral: true,
      threadId,
      interruptedTurnId,
      interruptedTurnStatus: interrupted.turn.status,
      interruptedStatusObserved,
      interruptCompletionElapsedMs,
      recoveryTurnId,
      recoveryTurnStatus: recovered.turn.status,
      expectedRecoveryText: RECOVERY_MARKER,
      recoveryMarkerObserved,
      recoveryFinalText,
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
