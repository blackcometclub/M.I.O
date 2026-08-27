import { mkdir, readFile, writeFile } from "node:fs/promises";

const fixtureFile = new URL("./fixtures/room-snapshot.json", import.meta.url);
const runtimeDirectory = new URL("./runtime/", import.meta.url);
const runtimeFile = new URL("./runtime/room-snapshot.json", import.meta.url);
const snapshot = JSON.parse(await readFile(fixtureFile, "utf8"));
const createdAt = new Date().toISOString();

snapshot.generatedAt = createdAt;
snapshot.rooms[0].messages.push({
  id: "claude-web-room-probe",
  roomId: "moe-dev-room",
  authorId: "codex",
  recipients: ["owner", "claude-web"],
  body: "CLAUDE_WEB_ROOM_RUNTIME_OK",
  createdAt,
  artifactIds: [],
});

await mkdir(runtimeDirectory, { recursive: true });
await writeFile(runtimeFile, JSON.stringify(snapshot, null, 2), "utf8");

console.log(
  JSON.stringify({
    result: "READY",
    roomId: "moe-dev-room",
    lastMessageId: "claude-web-room-probe",
    marker: "CLAUDE_WEB_ROOM_RUNTIME_OK",
  }),
);
