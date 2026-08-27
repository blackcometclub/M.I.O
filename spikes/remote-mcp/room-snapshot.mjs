import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { z } from "zod";

const identifier = z.string().min(1).max(128);

const participantSchema = z
  .object({
    id: identifier,
    displayName: z.string().min(1).max(200),
    kind: z.enum(["human", "ai"]),
  })
  .strict();

const messageSchema = z
  .object({
    id: identifier,
    roomId: identifier,
    authorId: identifier,
    recipients: z.array(identifier).max(100),
    body: z.string().max(100_000),
    createdAt: z.iso.datetime({ offset: true }),
    artifactIds: z.array(identifier).max(100),
  })
  .strict();

const roomSchema = z
  .object({
    id: identifier,
    name: z.string().min(1).max(200),
    participantIds: z.array(identifier).max(100),
    messages: z.array(messageSchema).max(10_000),
  })
  .strict();

const snapshotSchema = z
  .object({
    schemaVersion: z.literal("0.1.0"),
    generatedAt: z.iso.datetime({ offset: true }),
    participants: z.array(participantSchema).max(1_000),
    rooms: z.array(roomSchema).max(1_000),
  })
  .strict();

const moduleDirectory = dirname(fileURLToPath(import.meta.url));
export const roomRuntimeRoot = resolve(moduleDirectory, "runtime");

export function resolveRoomSnapshotFile(configuredPath) {
  const candidate = resolve(
    configuredPath ?? resolve(roomRuntimeRoot, "room-snapshot.json"),
  );
  const relativePath = relative(roomRuntimeRoot, candidate);

  if (relativePath.startsWith("..") || isAbsolute(relativePath)) {
    throw new Error("Room snapshot must stay inside the remote-mcp runtime directory");
  }

  return candidate;
}

export async function loadRoomSnapshot(snapshotFile) {
  const raw = await readFile(snapshotFile, "utf8");
  return snapshotSchema.parse(JSON.parse(raw.replace(/^\uFEFF/u, "")));
}

export function readRoom(snapshot, { roomId, afterMessageId, limit }) {
  const room = snapshot.rooms.find((candidate) => candidate.id === roomId);
  if (!room) {
    return { ok: false, code: "room_not_found", message: "The requested room is not available." };
  }

  let startIndex;
  if (afterMessageId) {
    const cursorIndex = room.messages.findIndex(
      (message) => message.id === afterMessageId,
    );
    if (cursorIndex < 0) {
      return { ok: false, code: "cursor_not_found", message: "The requested message cursor is not available." };
    }
    startIndex = cursorIndex + 1;
  } else {
    startIndex = Math.max(0, room.messages.length - limit);
  }

  const messages = room.messages.slice(startIndex, startIndex + limit);
  const participantSet = new Set(room.participantIds);
  const participants = snapshot.participants.filter((participant) =>
    participantSet.has(participant.id),
  );

  return {
    ok: true,
    snapshotGeneratedAt: snapshot.generatedAt,
    room: {
      id: room.id,
      name: room.name,
      participants,
      messages,
    },
    page: {
      afterMessageId: afterMessageId ?? null,
      limit,
      returned: messages.length,
      hasMoreBefore: !afterMessageId && startIndex > 0,
      hasMoreAfter: startIndex + messages.length < room.messages.length,
      nextAfterMessageId: messages.at(-1)?.id ?? afterMessageId ?? null,
    },
  };
}
