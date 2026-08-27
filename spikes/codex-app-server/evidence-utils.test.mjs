import assert from "node:assert/strict";
import test from "node:test";

import { sanitizeEvidence } from "./evidence-utils.mjs";

test("redacts runtime identifiers and local repository paths", () => {
  const repositoryRoot = "D:\\desktop\\M.O.E";
  const result = sanitizeEvidence(
    {
      threadId: "thread-secret",
      resumedTurnId: "turn-secret",
      serverUserAgent: "client/1.0 (Windows build details)",
      fixtureRoot: `${repositoryRoot}\\spikes\\fixtures\\sample`,
      nested: {
        itemId: "item-secret",
        authorName: "Local User",
        output: `A local user changed ${repositoryRoot.replaceAll("\\", "/")}/file.txt`,
      },
    },
    repositoryRoot,
  );

  assert.equal(result.threadId, "<redacted>");
  assert.equal(result.resumedTurnId, "<redacted>");
  assert.equal(result.serverUserAgent, "<redacted>");
  assert.equal(result.fixtureRoot, "<repository-root>\\spikes\\fixtures\\sample");
  assert.equal(result.nested.itemId, "<redacted>");
  assert.equal(result.nested.authorName, "<redacted>");
  assert.equal(result.nested.output, "A local user changed <repository-root>/file.txt");
});

test("preserves evidence needed to judge the probe", () => {
  const result = sanitizeEvidence(
    {
      probe: "example",
      status: "PASS",
      assertions: { markerObserved: true },
      elapsedMs: 42,
    },
    "D:\\desktop\\M.O.E",
  );

  assert.deepEqual(result, {
    probe: "example",
    status: "PASS",
    assertions: { markerObserved: true },
    elapsedMs: 42,
  });
});
