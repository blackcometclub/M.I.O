import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { writePublicEvidence } from "./evidence-utils.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..", "..");
const evidenceDirectory = join(scriptDirectory, "evidence");

for (const name of readdirSync(evidenceDirectory).filter((entry) =>
  entry.endsWith(".json"),
)) {
  const path = join(evidenceDirectory, name);
  const summary = JSON.parse(readFileSync(path, "utf8"));
  writePublicEvidence(path, summary, repositoryRoot);
  process.stdout.write(`sanitized ${name}\n`);
}
