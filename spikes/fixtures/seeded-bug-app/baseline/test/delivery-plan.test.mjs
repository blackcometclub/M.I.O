import assert from "node:assert/strict";
import test from "node:test";

import { buildDeliveryPlan } from "../src/delivery-plan.mjs";

test("未選択の参加者しかいない場合は配送を止める", () => {
  const result = buildDeliveryPlan([
    {
      adapterInstanceId: "codex-local-main",
      displayName: "Codex",
      selected: false,
      connection: "connected",
    },
  ]);

  assert.deepEqual(result, {
    status: "blocked",
    reason: "no_recipients",
    warning: null,
    recipients: [],
    blockedRecipients: [],
  });
});

test("接続済みと未接続の選択先を分ける", () => {
  const result = buildDeliveryPlan([
    {
      adapterInstanceId: "codex-local-main",
      displayName: "Codex",
      selected: true,
      connection: "connected",
    },
    {
      adapterInstanceId: "claude-code-qa",
      displayName: "Claude Code",
      selected: true,
      connection: "disconnected",
    },
  ]);

  assert.deepEqual(result, {
    status: "ready",
    reason: null,
    warning: "some_selected_offline",
    recipients: [
      { adapterInstanceId: "codex-local-main", displayName: "Codex" },
    ],
    blockedRecipients: [
      { adapterInstanceId: "claude-code-qa", displayName: "Claude Code" },
    ],
  });
});

test("未選択の古いregistry行は同じIDの選択済み行を隠さない", () => {
  const result = buildDeliveryPlan([
    {
      adapterInstanceId: "codex-local-main",
      displayName: "Codex (stale)",
      selected: false,
      connection: "disconnected",
    },
    {
      adapterInstanceId: "codex-local-main",
      displayName: "Codex",
      selected: true,
      connection: "connected",
    },
    {
      adapterInstanceId: "claude-code-qa",
      displayName: "Claude Code",
      selected: true,
      connection: "disconnected",
    },
  ]);

  assert.deepEqual(result, {
    status: "ready",
    reason: null,
    warning: "some_selected_offline",
    recipients: [
      { adapterInstanceId: "codex-local-main", displayName: "Codex" },
    ],
    blockedRecipients: [
      { adapterInstanceId: "claude-code-qa", displayName: "Claude Code" },
    ],
  });
});
