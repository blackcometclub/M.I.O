import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { writePublicEvidence } from "./evidence-utils.mjs";

const PROBE_MARKER = "MOE_APP_SERVER_OK";
const REQUEST_TIMEOUT_MS = 30_000;
const TURN_TIMEOUT_MS = 180_000;
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
export const repositoryRoot = join(scriptDirectory, "..", "..");
const evidencePath = join(scriptDirectory, "evidence", "handshake-latest.json");
const runtimeStatePath = join(
  repositoryRoot,
  ".moe",
  "probe-state",
  "codex-app-server-handshake.json",
);

export function resolveLauncher() {
  if (process.env.MOE_CODEX_BIN) {
    return {
      command: process.env.MOE_CODEX_BIN,
      args: ["app-server", "--listen", "stdio://"],
      source: "MOE_CODEX_BIN",
    };
  }

  const configuredCliJs = process.env.MOE_CODEX_CLI_JS;
  const defaultCliJs = process.env.APPDATA
    ? join(
        process.env.APPDATA,
        "npm",
        "node_modules",
        "@openai",
        "codex",
        "bin",
        "codex.js",
      )
    : null;
  const cliJs = configuredCliJs ?? defaultCliJs;

  if (cliJs && existsSync(cliJs)) {
    return {
      command: process.execPath,
      args: [cliJs, "app-server", "--listen", "stdio://"],
      source: configuredCliJs ? "MOE_CODEX_CLI_JS" : "global-npm-codex",
    };
  }

  return {
    command: "codex",
    args: ["app-server", "--listen", "stdio://"],
    source: "PATH",
  };
}

export class AppServerClient {
  constructor(launcher, options = {}) {
    this.launcher = launcher;
    this.fileChangeDecision = options.fileChangeDecision ?? "decline";
    this.commandExecutionDecision = options.commandExecutionDecision ?? "decline";
    this.nextId = 1;
    this.messageSequence = 0;
    this.pending = new Map();
    this.notifications = [];
    this.notificationCounts = new Map();
    this.serverRequestMethods = [];
    this.serverRequests = [];
    this.events = new EventEmitter();
    this.stdoutBuffer = "";
    this.stderrBuffer = "";
    this.closed = false;
  }

  start() {
    this.process = spawn(this.launcher.command, this.launcher.args, {
      cwd: repositoryRoot,
      env: process.env,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });

    this.process.stdout.setEncoding("utf8");
    this.process.stderr.setEncoding("utf8");
    this.process.stdout.on("data", (chunk) => this.consumeStdout(chunk));
    this.process.stderr.on("data", (chunk) => {
      this.stderrBuffer = `${this.stderrBuffer}${chunk}`.slice(-8_000);
    });
    this.process.on("error", (error) => this.failPending(error));
    this.process.on("exit", (code, signal) => {
      this.closed = true;
      this.failPending(
        new Error(`Codex App Server exited early (code=${code}, signal=${signal}).`),
      );
      this.events.emit("exit", { code, signal });
    });
  }

  consumeStdout(chunk) {
    this.stdoutBuffer += chunk;
    const lines = this.stdoutBuffer.split(/\r?\n/u);
    this.stdoutBuffer = lines.pop() ?? "";

    for (const line of lines) {
      if (!line.trim()) continue;

      let message;
      try {
        message = JSON.parse(line);
      } catch (error) {
        this.failPending(new Error(`App Server emitted non-JSON stdout: ${line}`));
        continue;
      }

      this.handleMessage(message);
    }
  }

  handleMessage(message) {
    message.sequence = ++this.messageSequence;

    if (message.id !== undefined && message.method) {
      this.handleServerRequest(message);
      return;
    }

    if (message.id !== undefined) {
      const pending = this.pending.get(String(message.id));
      if (!pending) return;

      clearTimeout(pending.timeout);
      this.pending.delete(String(message.id));
      if (message.error) {
        pending.reject(
          new Error(
            `${pending.method} failed (${message.error.code}): ${message.error.message}`,
          ),
        );
      } else {
        pending.resolve(message.result);
      }
      return;
    }

    if (message.method) {
      this.notifications.push(message);
      this.notificationCounts.set(
        message.method,
        (this.notificationCounts.get(message.method) ?? 0) + 1,
      );
      this.events.emit("notification", message);
    }
  }

  handleServerRequest(message) {
    this.serverRequestMethods.push(message.method);

    if (message.method === "item/commandExecution/requestApproval") {
      const decision = this.resolveApprovalDecision(
        this.commandExecutionDecision,
        message,
      );
      this.recordServerRequest(message, decision);
      this.send({ id: message.id, result: { decision } });
      return;
    }

    if (message.method === "item/fileChange/requestApproval") {
      const decision = this.resolveApprovalDecision(this.fileChangeDecision, message);
      this.recordServerRequest(message, decision);
      this.send({ id: message.id, result: { decision } });
      return;
    }

    if (message.method === "applyPatchApproval") {
      const decision = this.resolveApprovalDecision(this.fileChangeDecision, message);
      this.recordServerRequest(message, decision);
      this.send({
        id: message.id,
        result: {
          decision:
            decision === "accept" || decision === "acceptForSession"
              ? decision === "acceptForSession"
                ? "approved_for_session"
                : "approved"
              : {
                  denied: { rejection: "M.O.E. approval probe declined this file change." },
                },
        },
      });
      return;
    }

    if (message.method === "execCommandApproval") {
      const decision = this.resolveApprovalDecision(
        this.commandExecutionDecision,
        message,
      );
      this.recordServerRequest(message, decision);
      this.send({
        id: message.id,
        result: {
          decision:
            decision === "accept" || decision === "acceptForSession"
              ? decision === "acceptForSession"
                ? "approved_for_session"
                : "approved"
              : {
                  denied: { rejection: "M.O.E. approval probe declined this command." },
                },
        },
      });
      return;
    }

    this.send({
      id: message.id,
      error: {
        code: -32601,
        message: `M.O.E. handshake probe does not implement ${message.method}.`,
      },
    });
  }

  recordServerRequest(message, decision) {
    const item = this.findStartedItem(message.params?.itemId);
    this.serverRequests.push({
      sequence: message.sequence,
      id: message.id,
      method: message.method,
      threadId: message.params?.threadId ?? null,
      turnId: message.params?.turnId ?? null,
      itemId: message.params?.itemId ?? message.params?.callId ?? null,
      reason: message.params?.reason ?? null,
      grantRoot: message.params?.grantRoot ?? null,
      paths:
        item?.type === "fileChange"
          ? item.changes.map((change) => change.path)
          : [],
      decision,
    });
  }

  findStartedItem(itemId) {
    if (!itemId) return null;
    return [...this.notifications]
      .reverse()
      .find(
        (notification) =>
          notification.method === "item/started" &&
          notification.params?.item?.id === itemId,
      )?.params.item;
  }

  resolveApprovalDecision(configuredDecision, message) {
    const decision =
      typeof configuredDecision === "function"
        ? configuredDecision(message, this)
        : configuredDecision;
    const allowedDecisions = new Set([
      "accept",
      "acceptForSession",
      "decline",
      "cancel",
    ]);
    return allowedDecisions.has(decision) ? decision : "decline";
  }

  send(message) {
    if (this.closed || !this.process?.stdin.writable) {
      throw new Error("Codex App Server stdin is not writable.");
    }
    this.process.stdin.write(`${JSON.stringify(message)}\n`);
  }

  request(method, params, timeoutMs = REQUEST_TIMEOUT_MS) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(String(id));
        reject(new Error(`${method} timed out after ${timeoutMs} ms.`));
      }, timeoutMs);

      this.pending.set(String(id), { method, resolve, reject, timeout });
      this.send({ method, id, params });
    });
  }

  notify(method, params) {
    this.send({ method, params });
  }

  waitForNotification(method, predicate, timeoutMs = TURN_TIMEOUT_MS) {
    const existing = this.notifications.find(
      (notification) => notification.method === method && predicate(notification.params),
    );
    if (existing) return Promise.resolve(existing.params);

    return new Promise((resolve, reject) => {
      const onNotification = (notification) => {
        if (notification.method !== method || !predicate(notification.params)) return;
        cleanup();
        resolve(notification.params);
      };
      const onExit = ({ code, signal }) => {
        cleanup();
        reject(
          new Error(
            `Codex App Server exited while waiting for ${method} (code=${code}, signal=${signal}).`,
          ),
        );
      };
      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error(`${method} timed out after ${timeoutMs} ms.`));
      }, timeoutMs);
      const cleanup = () => {
        clearTimeout(timeout);
        this.events.off("notification", onNotification);
        this.events.off("exit", onExit);
      };

      this.events.on("notification", onNotification);
      this.events.on("exit", onExit);
    });
  }

  failPending(error) {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timeout);
      pending.reject(error);
    }
    this.pending.clear();
  }

  async stop() {
    if (!this.process || this.closed) return;
    this.process.stdin.end();
    await new Promise((resolve) => {
      const forceStop = setTimeout(() => {
        if (!this.closed) this.process.kill();
        resolve();
      }, 1_000);
      this.process.once("exit", () => {
        clearTimeout(forceStop);
        resolve();
      });
    });
  }
}

export function collectAgentText(notifications, threadId, turnId) {
  const completedMessages = notifications
    .filter(
      (notification) =>
        notification.method === "item/completed" &&
        notification.params?.threadId === threadId &&
        notification.params?.turnId === turnId &&
        notification.params?.item?.type === "agentMessage",
    )
    .map((notification) => notification.params.item.text);

  if (completedMessages.length > 0) {
    return completedMessages.at(-1);
  }

  return notifications
    .filter(
      (notification) =>
        notification.method === "item/agentMessage/delta" &&
        notification.params?.threadId === threadId &&
        notification.params?.turnId === turnId,
    )
    .map((notification) => notification.params.delta)
    .join("");
}

function writeEvidence(summary) {
  return writePublicEvidence(evidencePath, summary, repositoryRoot);
}

function writeRuntimeState(summary) {
  mkdirSync(dirname(runtimeStatePath), { recursive: true });
  writeFileSync(
    runtimeStatePath,
    `${JSON.stringify(
      {
        status: summary.status,
        startedAt: summary.startedAt,
        threadId: summary.threadId,
        finalText: summary.finalText,
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
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
      ephemeral: false,
    });
    const threadId = threadStart.thread.id;

    const turnStart = await client.request("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: `Reply with exactly ${PROBE_MARKER}. Do not inspect files or run tools.`,
          text_elements: [],
        },
      ],
    });
    const turnId = turnStart.turn.id;
    const completed = await client.waitForNotification(
      "turn/completed",
      (params) => params?.threadId === threadId && params?.turn?.id === turnId,
    );
    const finalText = collectAgentText(client.notifications, threadId, turnId).trim();
    const markerObserved = finalText.includes(PROBE_MARKER);
    const status = completed.turn.status === "completed" && markerObserved ? "PASS" : "FAIL";
    const summary = {
      probe: "codex-app-server-handshake",
      status,
      startedAt: startedAt.toISOString(),
      elapsedMs: Date.now() - startedAtMs,
      launcherSource: launcher.source,
      transport: "stdio",
      serverUserAgent: initialize.userAgent ?? null,
      platformFamily: initialize.platformFamily ?? null,
      platformOs: initialize.platformOs ?? null,
      threadId,
      turnId,
      turnStatus: completed.turn.status,
      markerObserved,
      finalText,
      notificationCounts: Object.fromEntries(
        [...client.notificationCounts.entries()].sort(([a], [b]) => a.localeCompare(b)),
      ),
      serverRequestMethods: client.serverRequestMethods,
    };

    writeRuntimeState(summary);
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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  });
}
