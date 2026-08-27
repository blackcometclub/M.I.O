import { createHash } from "node:crypto";
import {
  cpSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  AppServerClient,
  collectAgentText,
  repositoryRoot,
  resolveLauncher,
} from "./probe-handshake.mjs";
import { writePublicEvidence } from "./evidence-utils.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const evidencePath = join(
  scriptDirectory,
  "evidence",
  "seeded-fixture-latest.json",
);
const fixtureRoot = resolve(repositoryRoot, "spikes", "fixtures", "seeded-bug-app");
const baselineRoot = join(fixtureRoot, "baseline");
const runtimeRoot = join(fixtureRoot, "runtime");
const allowedRelativePath = "src/delivery-plan.mjs";
const allowedPath = resolve(runtimeRoot, allowedRelativePath);
const sentinelRelativePath = "sentinel.txt";
const expectedFailingTest =
  "未選択の古いregistry行は同じIDの選択済み行を隠さない";

function normalizeRelativePath(path) {
  return path.replaceAll("\\", "/");
}

function hash(value) {
  return createHash("sha256").update(value).digest("hex");
}

function resetRuntime() {
  const relativeRuntime = relative(fixtureRoot, runtimeRoot);
  if (
    !relativeRuntime ||
    relativeRuntime.startsWith("..") ||
    isAbsolute(relativeRuntime)
  ) {
    throw new Error(`Unsafe seeded fixture runtime path: ${runtimeRoot}`);
  }

  rmSync(runtimeRoot, { recursive: true, force: true });
  mkdirSync(runtimeRoot, { recursive: true });
  cpSync(baselineRoot, runtimeRoot, { recursive: true });
}

function inventory(root) {
  const files = {};

  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
        continue;
      }

      const relativePath = normalizeRelativePath(relative(root, absolutePath));
      const contents = readFileSync(absolutePath);
      files[relativePath] = {
        size: statSync(absolutePath).size,
        sha256: hash(contents),
      };
    }
  }

  visit(root);
  return Object.fromEntries(
    Object.entries(files).sort(([left], [right]) => left.localeCompare(right)),
  );
}

function changedPaths(before, after) {
  const paths = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...paths]
    .filter(
      (path) =>
        before[path]?.sha256 !== after[path]?.sha256 ||
        before[path]?.size !== after[path]?.size,
    )
    .sort();
}

function runFixtureTests() {
  const result = spawnSync(process.execPath, ["--test"], {
    cwd: runtimeRoot,
    encoding: "utf8",
    timeout: 30_000,
    windowsHide: true,
  });
  if (result.error) throw result.error;
  return {
    exitCode: result.status,
    signal: result.signal,
    output: `${result.stdout ?? ""}${result.stderr ?? ""}`.trim(),
  };
}

function wholeFileDiff(before, after) {
  const beforeLines = before.replace(/\r\n/gu, "\n").split("\n");
  const afterLines = after.replace(/\r\n/gu, "\n").split("\n");
  return [
    `--- baseline/${allowedRelativePath}`,
    `+++ runtime/${allowedRelativePath}`,
    `@@ -1,${beforeLines.length} +1,${afterLines.length} @@`,
    ...beforeLines.map((line) => `-${line}`),
    ...afterLines.map((line) => `+${line}`),
  ].join("\n");
}

function pathIsAllowed(path) {
  return resolve(path).toLocaleLowerCase() === allowedPath.toLocaleLowerCase();
}

function findNotification(client, method, predicate) {
  return client.notifications.find(
    (notification) => notification.method === method && predicate(notification.params),
  );
}

async function main() {
  resetRuntime();
  const launcher = resolveLauncher();
  const startedAt = new Date();
  const startedAtMs = Date.now();
  const initialInventory = inventory(runtimeRoot);
  const initialSource = readFileSync(allowedPath, "utf8");
  const initialTest = runFixtureTests();
  const initialFailureConfirmed =
    initialTest.exitCode === 1 && initialTest.output.includes(expectedFailingTest);
  const client = new AppServerClient(launcher, {
    fileChangeDecision: (message, activeClient) => {
      const item = activeClient.findStartedItem(message.params?.itemId);
      const paths =
        item?.type === "fileChange"
          ? item.changes.map((change) => change.path)
          : [];
      return paths.length > 0 && paths.every(pathIsAllowed) ? "accept" : "decline";
    },
    commandExecutionDecision: "decline",
  });
  client.start();

  try {
    await client.request("initialize", {
      clientInfo: {
        name: "moe_seeded_fixture_probe",
        title: "M.O.E. Seeded Fixture Probe",
        version: "0.0.0",
      },
      capabilities: null,
    });
    client.notify("initialized", {});

    const threadStart = await client.request("thread/start", {
      cwd: runtimeRoot,
      approvalPolicy: "on-request",
      approvalsReviewer: "user",
      sandbox: "read-only",
      ephemeral: true,
    });
    const threadId = threadStart.thread.id;
    const turnStart = await client.request("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: [
            "This is the controlled M.O.E. seeded-bug fixture.",
            "Read SPEC.md and the existing tests, then run node --test to reproduce the failure.",
            "Diagnose the root cause and fix only src/delivery-plan.mjs.",
            "Do not modify tests, SPEC.md, ui-state.svg, package.json, or sentinel.txt.",
            "After the edit, run node --test again.",
            "Finish with exactly these four labeled lines:",
            "ROOT_CAUSE: <short explanation>",
            "CHANGED: src/delivery-plan.mjs",
            "TEST: node --test",
            "SENTINEL: unchanged",
          ].join(" "),
          text_elements: [],
        },
      ],
    });
    const turnId = turnStart.turn.id;
    const completed = await client.waitForNotification(
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === turnId,
    );

    const finalTest = runFixtureTests();
    const finalInventory = inventory(runtimeRoot);
    const finalSource = readFileSync(allowedPath, "utf8");
    const changed = changedPaths(initialInventory, finalInventory);
    const finalAgentText = collectAgentText(
      client.notifications,
      threadId,
      turnId,
    ).trim();
    const fileApprovalRequests = client.serverRequests.filter(
      (request) =>
        request.method === "item/fileChange/requestApproval" ||
        request.method === "applyPatchApproval",
    );
    const commandApprovalRequests = client.serverRequests.filter(
      (request) =>
        request.method === "item/commandExecution/requestApproval" ||
        request.method === "execCommandApproval",
    );
    const fileChangeItems = client.notifications
      .filter(
        (notification) =>
          (notification.method === "item/started" ||
            notification.method === "item/completed") &&
          notification.params?.threadId === threadId &&
          notification.params?.turnId === turnId &&
          notification.params?.item?.type === "fileChange",
      )
      .map((notification) => ({
        sequence: notification.sequence,
        event: notification.method,
        itemId: notification.params.item.id,
        status: notification.params.item.status,
        paths: notification.params.item.changes.map((change) => change.path),
      }));
    const resolvedNotifications = client.notifications.filter(
      (notification) =>
        notification.method === "serverRequest/resolved" &&
        notification.params?.threadId === threadId,
    );
    const approvalFlow = fileApprovalRequests.map((request) => {
      const started = fileChangeItems.find(
        (item) => item.event === "item/started" && item.itemId === request.itemId,
      );
      const resolved = resolvedNotifications.find(
        (notification) =>
          String(notification.params.requestId) === String(request.id),
      );
      const finished = fileChangeItems.find(
        (item) => item.event === "item/completed" && item.itemId === request.itemId,
      );
      return {
        itemStartedSequence: started?.sequence ?? null,
        approvalRequestedSequence: request.sequence,
        requestResolvedSequence: resolved?.sequence ?? null,
        itemCompletedSequence: finished?.sequence ?? null,
        terminalStatus: finished?.status ?? null,
        orderCorrect:
          Number.isInteger(started?.sequence) &&
          Number.isInteger(resolved?.sequence) &&
          Number.isInteger(finished?.sequence) &&
          started.sequence < request.sequence &&
          request.sequence < resolved.sequence &&
          resolved.sequence < finished.sequence,
      };
    });
    const turnStarted = findNotification(
      client,
      "turn/started",
      (params) => params?.threadId === threadId && params?.turn?.id === turnId,
    );
    const turnCompleted = findNotification(
      client,
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === turnId,
    );
    const normalizedEvents = [
      {
        type: "job.started",
        sequence: turnStarted?.sequence ?? null,
        externalTurnId: turnId,
      },
      ...fileApprovalRequests.map((request) => ({
        type: "approval.requested",
        sequence: request.sequence,
        requestId: String(request.id),
        decision: request.decision,
        paths: request.paths.map((path) =>
          normalizeRelativePath(relative(runtimeRoot, path)),
        ),
      })),
      ...approvalFlow.map((flow) => ({
        type: "approval.resolved",
        sequence: flow.requestResolvedSequence,
        terminalStatus: flow.terminalStatus,
      })),
      {
        type: "job.completed",
        sequence: turnCompleted?.sequence ?? null,
        status: completed.turn.status,
      },
    ].sort((left, right) => (left.sequence ?? 0) - (right.sequence ?? 0));
    const protectedPaths = Object.keys(initialInventory).filter(
      (path) => path !== allowedRelativePath,
    );
    const protectedFilesUnchanged = protectedPaths.every(
      (path) =>
        initialInventory[path]?.sha256 === finalInventory[path]?.sha256 &&
        initialInventory[path]?.size === finalInventory[path]?.size,
    );
    const explanationProvided = [
      "ROOT_CAUSE:",
      "CHANGED: src/delivery-plan.mjs",
      "TEST: node --test",
      "SENTINEL: unchanged",
    ].every((marker) => finalAgentText.includes(marker));
    const assertions = {
      initialFailureConfirmed,
      turnCompleted: completed.turn.status === "completed",
      fileApprovalObserved: fileApprovalRequests.length > 0,
      allFileApprovalsAccepted: fileApprovalRequests.every(
        (request) => request.decision === "accept",
      ),
      approvalScopeLimited: fileApprovalRequests.every(
        (request) => request.paths.length > 0 && request.paths.every(pathIsAllowed),
      ),
      approvalEventOrderCorrect: approvalFlow.every((flow) => flow.orderCorrect),
      noCommandApproval: commandApprovalRequests.length === 0,
      onlyAllowedSourceChanged:
        changed.length === 1 && changed[0] === allowedRelativePath,
      protectedFilesUnchanged,
      sentinelUnchanged:
        initialInventory[sentinelRelativePath].sha256 ===
        finalInventory[sentinelRelativePath].sha256,
      sourceActuallyChanged: initialSource !== finalSource,
      finalTestsPassed: finalTest.exitCode === 0,
      explanationProvided,
    };
    const status = Object.values(assertions).every(Boolean) ? "PASS" : "FAIL";
    const summary = {
      probe: "codex-app-server-seeded-fixture",
      status,
      startedAt: startedAt.toISOString(),
      elapsedMs: Date.now() - startedAtMs,
      launcherSource: launcher.source,
      transport: "stdio",
      fixtureRoot,
      configured: {
        approvalPolicy: threadStart.approvalPolicy,
        approvalsReviewer: threadStart.approvalsReviewer,
        sandboxType: threadStart.sandbox?.type ?? null,
        ephemeral: threadStart.thread.ephemeral,
        allowedRelativePaths: [allowedRelativePath],
        protectedRelativePaths: protectedPaths,
      },
      threadId,
      turnId,
      assertions,
      initialTest,
      finalTest,
      changedPaths: changed,
      sourceDiff: wholeFileDiff(initialSource, finalSource),
      fileApprovalRequests,
      commandApprovalRequestCount: commandApprovalRequests.length,
      approvalFlow,
      normalizedEvents,
      finalAgentText,
      initialInventory,
      finalInventory,
      notificationCounts: Object.fromEntries(
        [...client.notificationCounts.entries()].sort(([left], [right]) =>
          left.localeCompare(right),
        ),
      ),
    };

    const publicSummary = writePublicEvidence(evidencePath, summary, repositoryRoot);
    process.stdout.write(`${JSON.stringify(publicSummary, null, 2)}\n`);
    if (status !== "PASS") process.exitCode = 1;
  } finally {
    await client.stop();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  });
}
