import { createHash } from "node:crypto";
import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
} from "node:fs";
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
const evidencePath = join(scriptDirectory, "evidence", "approval-latest.json");
const fixtureRoot = resolve(repositoryRoot, "spikes", "fixtures", "approval-sandbox");
const baselineRoot = join(fixtureRoot, "baseline");
const runtimeRoot = join(fixtureRoot, "runtime");
const targetName = "target.txt";
const sentinelName = "sentinel.txt";
const beforeText = "BEFORE_APPROVAL\n";
const approvedText = "APPROVED_CHANGE\n";
const sentinelText = "SENTINEL_MUST_NOT_CHANGE\n";

function normalizeLineEndings(value) {
  return value.replace(/\r\n/gu, "\n");
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
    throw new Error(`Unsafe approval fixture runtime path: ${runtimeRoot}`);
  }

  rmSync(runtimeRoot, { recursive: true, force: true });
  mkdirSync(runtimeRoot, { recursive: true });
  copyFileSync(join(baselineRoot, targetName), join(runtimeRoot, targetName));
  copyFileSync(join(baselineRoot, sentinelName), join(runtimeRoot, sentinelName));
}

function readRuntimeState() {
  const targetRaw = readFileSync(join(runtimeRoot, targetName), "utf8");
  const sentinelRaw = readFileSync(join(runtimeRoot, sentinelName), "utf8");
  return {
    files: readdirSync(runtimeRoot).sort(),
    targetText: normalizeLineEndings(targetRaw),
    targetSha256: hash(targetRaw),
    sentinelText: normalizeLineEndings(sentinelRaw),
    sentinelSha256: hash(sentinelRaw),
  };
}

function matchingNotifications(client, method, threadId, turnId) {
  return client.notifications.filter(
    (notification) =>
      notification.method === method &&
      notification.params?.threadId === threadId &&
      (!turnId || notification.params?.turnId === turnId),
  );
}

async function runApprovalCase(launcher, name, decision) {
  resetRuntime();
  const initial = readRuntimeState();
  const client = new AppServerClient(launcher, {
    fileChangeDecision: decision,
    commandExecutionDecision: "decline",
  });
  const startedAtMs = Date.now();
  client.start();

  try {
    await client.request("initialize", {
      clientInfo: {
        name: `moe_approval_${name}_probe`,
        title: `M.O.E. Approval ${name} Probe`,
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
            "This is a controlled approval-flow fixture.",
            "Change only target.txt in the current directory.",
            "Replace its entire content with APPROVED_CHANGE followed by one newline.",
            "Do not run shell commands. Use the file-editing tool.",
            "Do not touch sentinel.txt and do not create any files.",
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

    const final = readRuntimeState();
    const approvalRequests = client.serverRequests.filter(
      (request) =>
        request.method === "item/fileChange/requestApproval" ||
        request.method === "applyPatchApproval",
    );
    const commandRequests = client.serverRequests.filter(
      (request) =>
        request.method === "item/commandExecution/requestApproval" ||
        request.method === "execCommandApproval",
    );
    const resolvedNotifications = matchingNotifications(
      client,
      "serverRequest/resolved",
      threadId,
    );
    const resolvedRequestIds = resolvedNotifications.map((notification) =>
      String(notification.params.requestId),
    );
    const fileItems = [
      ...matchingNotifications(client, "item/started", threadId, turnId),
      ...matchingNotifications(client, "item/completed", threadId, turnId),
    ]
      .filter((notification) => notification.params?.item?.type === "fileChange")
      .map((notification) => ({
        sequence: notification.sequence,
        event: notification.method,
        itemId: notification.params.item.id,
        status: notification.params.item.status,
        paths: notification.params.item.changes.map((change) => change.path),
      }));
    const fileStatuses = fileItems.map((item) => item.status);
    const expectedTargetPath = resolve(runtimeRoot, targetName).toLocaleLowerCase();
    const fileChangeScopeCorrect = fileItems.every(
      (item) =>
        item.paths.length > 0 &&
        item.paths.every(
          (path) => resolve(path).toLocaleLowerCase() === expectedTargetPath,
        ),
    );
    const approvalFlow = approvalRequests.map((request) => {
      const started = fileItems.find(
        (item) => item.event === "item/started" && item.itemId === request.itemId,
      );
      const resolved = resolvedNotifications.find(
        (notification) => String(notification.params.requestId) === String(request.id),
      );
      const finished = fileItems.find(
        (item) => item.event === "item/completed" && item.itemId === request.itemId,
      );
      const orderCorrect =
        Number.isInteger(started?.sequence) &&
        Number.isInteger(resolved?.sequence) &&
        Number.isInteger(finished?.sequence) &&
        started.sequence < request.sequence &&
        request.sequence < resolved.sequence &&
        resolved.sequence < finished.sequence;
      return {
        itemStartedSequence: started?.sequence ?? null,
        approvalRequestedSequence: request.sequence,
        requestResolvedSequence: resolved?.sequence ?? null,
        itemCompletedSequence: finished?.sequence ?? null,
        orderCorrect,
      };
    });
    const approvalResolved = approvalRequests.every((request) =>
      resolvedRequestIds.includes(String(request.id)),
    );
    const approvalOrderCorrect = approvalFlow.every((flow) => flow.orderCorrect);
    const selectedDecisionObserved = approvalRequests.every(
      (request) => request.decision === decision,
    );
    const commonPass =
      completed.turn.status === "completed" &&
      approvalRequests.length > 0 &&
      selectedDecisionObserved &&
      commandRequests.length === 0 &&
      approvalResolved &&
      approvalOrderCorrect &&
      fileChangeScopeCorrect &&
      final.sentinelText === sentinelText &&
      final.sentinelSha256 === initial.sentinelSha256 &&
      final.files.join("|") === `${sentinelName}|${targetName}`;
    const casePass =
      name === "deny"
        ? commonPass &&
          final.targetText === beforeText &&
          final.targetSha256 === initial.targetSha256 &&
          fileStatuses.includes("declined")
        : commonPass &&
          final.targetText === approvedText &&
          fileStatuses.includes("completed");

    return {
      name,
      status: casePass ? "PASS" : "FAIL",
      decision,
      elapsedMs: Date.now() - startedAtMs,
      threadId,
      turnId,
      turnStatus: completed.turn.status,
      configured: {
        approvalPolicy: threadStart.approvalPolicy,
        approvalsReviewer: threadStart.approvalsReviewer,
        sandboxType: threadStart.sandbox?.type ?? null,
        ephemeral: threadStart.thread.ephemeral,
      },
      approvalRequests,
      commandApprovalRequestCount: commandRequests.length,
      resolvedRequestIds,
      approvalResolved,
      approvalFlow,
      fileChangeScopeCorrect,
      fileItems,
      finalAgentText: collectAgentText(client.notifications, threadId, turnId).trim(),
      initial,
      final,
      assertions: {
        turnCompleted: completed.turn.status === "completed",
        fileApprovalObserved: approvalRequests.length > 0,
        selectedDecisionObserved,
        noCommandApproval: commandRequests.length === 0,
        approvalResolved,
        approvalEventOrderCorrect: approvalOrderCorrect,
        fileChangeScopeCorrect,
        targetExpected:
          final.targetText === (name === "deny" ? beforeText : approvedText),
        denyTargetHashUnchanged:
          name !== "deny" || final.targetSha256 === initial.targetSha256,
        sentinelUnchanged:
          final.sentinelText === sentinelText &&
          final.sentinelSha256 === initial.sentinelSha256,
        onlyExpectedFiles: final.files.join("|") === `${sentinelName}|${targetName}`,
        terminalFileStatus: fileStatuses.includes(
          name === "deny" ? "declined" : "completed",
        ),
      },
      notificationCounts: Object.fromEntries(
        [...client.notificationCounts.entries()].sort(([a], [b]) =>
          a.localeCompare(b),
        ),
      ),
    };
  } finally {
    await client.stop();
  }
}

function writeEvidence(summary) {
  return writePublicEvidence(evidencePath, summary, repositoryRoot);
}

async function main() {
  const launcher = resolveLauncher();
  const startedAt = new Date();
  const startedAtMs = Date.now();
  const deny = await runApprovalCase(launcher, "deny", "decline");
  const accept = await runApprovalCase(launcher, "accept", "accept");
  const status = deny.status === "PASS" && accept.status === "PASS" ? "PASS" : "FAIL";
  const summary = {
    probe: "codex-app-server-file-change-approval",
    status,
    startedAt: startedAt.toISOString(),
    elapsedMs: Date.now() - startedAtMs,
    launcherSource: launcher.source,
    transport: "stdio",
    fixtureRoot,
    deny,
    accept,
  };

  const publicSummary = writeEvidence(summary);
  process.stdout.write(`${JSON.stringify(publicSummary, null, 2)}\n`);
  if (status !== "PASS") process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  });
}
