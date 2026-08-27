import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import {
  AppServerClient,
  collectAgentText,
  repositoryRoot,
  resolveLauncher,
} from "./probe-handshake.mjs";
import { writePublicEvidence } from "./evidence-utils.mjs";

const EXPECTED_CODE = "NEKOMIMI-42";
const TURN_TIMEOUT_MS = 180_000;
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const imagePath = join(scriptDirectory, "fixtures", "local-image-vision.png");
const evidencePath = join(scriptDirectory, "evidence", "image-latest.json");

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex").toUpperCase();
}

function writeEvidence(summary) {
  return writePublicEvidence(evidencePath, summary, repositoryRoot);
}

async function main() {
  if (!existsSync(imagePath)) {
    throw new Error(
      "Image fixture is missing. Run generate-image-fixture.ps1 before this probe.",
    );
  }

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

    const turnStart = await client.request("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: "添付画像を目で確認してください。画像下部の「CODE:」の直後に書かれている文字列だけを、説明や記号を足さずに回答してください。",
          text_elements: [],
        },
        {
          type: "localImage",
          path: imagePath,
          detail: "original",
        },
      ],
    });
    const turnId = turnStart.turn.id;
    const completed = await client.waitForNotification(
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === turnId,
      TURN_TIMEOUT_MS,
    );
    const finalText = collectAgentText(client.notifications, threadId, turnId).trim();
    const exactCodeObserved = finalText === EXPECTED_CODE;
    const status =
      completed.turn.status === "completed" && exactCodeObserved ? "PASS" : "FAIL";
    const summary = {
      probe: "codex-app-server-local-image",
      status,
      startedAt: startedAt.toISOString(),
      elapsedMs: Date.now() - startedAtMs,
      launcherSource: launcher.source,
      transport: "stdio",
      serverUserAgent: initialize.userAgent ?? null,
      imagePath: relative(repositoryRoot, imagePath).replaceAll("\\", "/"),
      imageSha256: sha256(imagePath),
      imageDimensions: { width: 1200, height: 800 },
      imageDetail: "original",
      threadEphemeral: true,
      threadId,
      turnId,
      turnStatus: completed.turn.status,
      expectedCode: EXPECTED_CODE,
      exactCodeObserved,
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
