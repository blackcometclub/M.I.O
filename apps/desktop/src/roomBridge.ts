import { invoke } from "@tauri-apps/api/core";

import { demoParticipants, ownerParticipantId } from "./mockData";
import type {
  AiConnectionMap,
  AiConnectionState,
  Participant,
  ParticipantMap,
  Room,
  RoomConductorStatus,
  RoomWorkspaceStatus,
} from "./types";

type BackendParticipant = {
  id: string;
  displayName: string;
  kind: "human" | "ai";
};

type BackendMessage = {
  id: string;
  roomId: string;
  authorId: string;
  recipients: string[];
  body: string;
  createdAt: string;
  artifactIds: string[];
  provenance?: "codexOwnerProxy";
};

type BackendRoomWriteSuccess = {
  ok: true;
  status: "appended" | "duplicate";
  message: BackendMessage;
};

type BackendRoomSummary = {
  id: string;
  name: string;
  participantIds: string[];
  latestMessageAt: string | null;
};

type BackendRoomCatalogSuccess = {
  ok: true;
  rooms: BackendRoomSummary[];
};

type BackendRoomMutationSuccess = {
  ok: true;
  status: "created" | "added" | "renamed" | "removed" | "deleted" | "duplicate";
  room: BackendRoomSummary;
};

type BackendRoomBackupSuccess = {
  ok: true;
  fileName: string;
  roomCount: number;
};

export type RoomContextReport = {
  mode: "initial" | "resumed" | "reconstructed";
  includedMessages: number;
  omittedMessages: number;
  truncatedMessages: number;
  omittedCharacters: number;
  continuitySaved: boolean;
};

type BackendRecipientDispatchResult = {
  recipientId: string;
  status: "completed" | "duplicate" | "failed" | "unknown" | "queued" | "unsupported";
  message: BackendMessage | null;
  error: { code: string; message: string } | null;
  context: RoomContextReport | null;
};

type BackendRoomContinuityResetSuccess = {
  ok: true;
  roomId: string;
  participantId: string;
  changed: boolean;
};

type BackendRoomDispatchSuccess = {
  ok: true;
  sourceMessageId: string;
  results: BackendRecipientDispatchResult[];
};

type BackendRoomDispatchUnknownsSuccess = {
  ok: true;
  roomId: string;
  unknowns: Array<{
    sourceMessageId: string;
    recipientId: string;
    code: string;
  }>;
};

type BackendRoomReadSuccess = {
  ok: true;
  room: {
    id: string;
    name: string;
    participants: BackendParticipant[];
    messages: BackendMessage[];
  };
};

type BackendAiConnectionStatus = {
  participantId: string;
  state: AiConnectionState;
  label: string;
  detail: string;
};

type BackendAiConnectionStatusSuccess = {
  ok: true;
  connections: BackendAiConnectionStatus[];
};

type BackendRoomWorkspaceStatus = {
  ok: true;
  roomId: string;
  mode: "chatOnly" | "workspace";
  folderName: string | null;
  available: boolean;
  changed: boolean;
};

type BackendRoomConductorStatus = {
  ok: true;
  roomId: string;
  conductorId: string | null;
  sendMode: "direct" | "conductor";
};

type BackendRoomOrchestrationResult = {
  ok: boolean;
  operationId: string;
  status: "completed" | "duplicate" | "failed" | "unknown";
  finalMessage: BackendMessage | null;
};

export type DesktopRoomHydration = {
  participants: ParticipantMap;
  room: Room;
};

export type DesktopRoomsHydration = {
  participants: ParticipantMap;
  rooms: Room[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isBackendAiConnectionStatus(value: unknown): value is BackendAiConnectionStatus {
  return (
    isRecord(value) &&
    typeof value.participantId === "string" &&
    (value.state === "ready" ||
      value.state === "installed" ||
      value.state === "setupRequired" ||
      value.state === "unsupported") &&
    typeof value.label === "string" &&
    typeof value.detail === "string"
  );
}

function isBackendAiConnectionStatusSuccess(
  value: unknown,
): value is BackendAiConnectionStatusSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    Array.isArray(value.connections) &&
    value.connections.every(isBackendAiConnectionStatus)
  );
}

function isBackendRoomWorkspaceStatus(value: unknown): value is BackendRoomWorkspaceStatus {
  return (
    isRecord(value) &&
    value.ok === true &&
    typeof value.roomId === "string" &&
    (value.mode === "chatOnly" || value.mode === "workspace") &&
    (value.folderName === null || typeof value.folderName === "string") &&
    typeof value.available === "boolean" &&
    typeof value.changed === "boolean" &&
    (value.mode === "workspace" ? value.folderName !== null : value.folderName === null)
  );
}

function isBackendRoomConductorStatus(
  value: unknown,
): value is BackendRoomConductorStatus {
  return (
    isRecord(value) &&
    value.ok === true &&
    typeof value.roomId === "string" &&
    (value.conductorId === null || typeof value.conductorId === "string") &&
    (value.sendMode === "direct" || value.sendMode === "conductor") &&
    (value.sendMode !== "conductor" || value.conductorId !== null)
  );
}

function isBackendRoomOrchestrationResult(
  value: unknown,
): value is BackendRoomOrchestrationResult {
  return (
    isRecord(value) &&
    typeof value.ok === "boolean" &&
    typeof value.operationId === "string" &&
    (value.status === "completed" ||
      value.status === "duplicate" ||
      value.status === "failed" ||
      value.status === "unknown") &&
    (value.finalMessage === null || isBackendMessage(value.finalMessage)) &&
    (value.ok === (value.status === "completed" || value.status === "duplicate")) &&
    (value.ok ? value.finalMessage !== null : value.finalMessage === null)
  );
}

function workspaceView(value: BackendRoomWorkspaceStatus): RoomWorkspaceStatus {
  return {
    roomId: value.roomId,
    mode: value.mode,
    folderName: value.folderName,
    available: value.available,
  };
}

function isBackendParticipant(value: unknown): value is BackendParticipant {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.displayName === "string" &&
    (value.kind === "human" || value.kind === "ai")
  );
}

function isBackendMessage(value: unknown): value is BackendMessage {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.roomId === "string" &&
    typeof value.authorId === "string" &&
    isStringArray(value.recipients) &&
    typeof value.body === "string" &&
    typeof value.createdAt === "string" &&
    isStringArray(value.artifactIds) &&
    (value.provenance === undefined || value.provenance === "codexOwnerProxy")
  );
}

function isBackendRoomReadSuccess(value: unknown): value is BackendRoomReadSuccess {
  if (!isRecord(value) || value.ok !== true || !isRecord(value.room)) {
    return false;
  }
  return (
    typeof value.room.id === "string" &&
    typeof value.room.name === "string" &&
    Array.isArray(value.room.participants) &&
    value.room.participants.every(isBackendParticipant) &&
    Array.isArray(value.room.messages) &&
    value.room.messages.every(isBackendMessage)
  );
}

function isBackendRoomSummary(value: unknown): value is BackendRoomSummary {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.name === "string" &&
    isStringArray(value.participantIds) &&
    (value.latestMessageAt === null || typeof value.latestMessageAt === "string")
  );
}

function isBackendRoomCatalogSuccess(value: unknown): value is BackendRoomCatalogSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    Array.isArray(value.rooms) &&
    value.rooms.every(isBackendRoomSummary)
  );
}

function isBackendRoomMutationSuccess(value: unknown): value is BackendRoomMutationSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    (value.status === "created" ||
      value.status === "added" ||
      value.status === "renamed" ||
      value.status === "removed" ||
      value.status === "deleted" ||
      value.status === "duplicate") &&
    isBackendRoomSummary(value.room)
  );
}

function isBackendRoomBackupSuccess(value: unknown): value is BackendRoomBackupSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    typeof value.fileName === "string" &&
    /^moe-room-backup-\d{20}\.json$/.test(value.fileName) &&
    Number.isSafeInteger(value.roomCount) &&
    Number(value.roomCount) > 0
  );
}

function participantView(participant: BackendParticipant): Participant {
  const known = demoParticipants[participant.id];
  const generatedBadge = Array.from(participant.displayName)
    .filter((character) => /[\p{L}\p{N}]/u.test(character))
    .slice(0, 3)
    .join("")
    .toUpperCase() || "AI";
  return {
    id: participant.id,
    canonicalName: known?.canonicalName ?? participant.displayName,
    displayName: participant.displayName,
    identityBadge: known?.identityBadge ?? (participant.kind === "human" ? "Owner" : generatedBadge),
    kind: participant.kind,
    serviceLabel:
      known?.serviceLabel ?? (participant.kind === "human" ? "Participant" : "AI participant"),
    initials:
      known?.initials ?? Array.from(participant.displayName).slice(0, 2).join(""),
    accent: known?.accent ?? "#6b7280",
    ...(known?.avatarUrl ? { avatarUrl: known.avatarUrl } : {}),
  };
}

function displayTime(createdAt: string) {
  const numericTimestamp = /^\d+$/.test(createdAt) ? Number(createdAt) : createdAt;
  const date = new Date(numericTimestamp);
  if (Number.isNaN(date.getTime())) {
    return "時刻不明";
  }
  return new Intl.DateTimeFormat("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

function messageView(message: BackendMessage, participants: ParticipantMap) {
  return {
    id: message.id,
    authorId: message.authorId,
    body: message.body,
    targetIds: message.recipients,
    sentAt: displayTime(message.createdAt),
    ...(message.provenance ? { provenance: message.provenance } : {}),
    isDemo:
      participants[message.authorId]?.kind === "ai" && message.id.startsWith("welcome-"),
  };
}

function hydrateRoom(value: BackendRoomReadSuccess): DesktopRoomHydration {
  const participants = Object.fromEntries(
    value.room.participants.map((participant) => [
      participant.id,
      participantView(participant),
    ]),
  );
  return {
    participants,
    room: {
      id: value.room.id,
      name: value.room.name,
      participantIds: value.room.participants.map((participant) => participant.id),
      messages: value.room.messages.map((message) => messageView(message, participants)),
      updatedLabel:
        value.room.messages.length > 0
          ? messageView(value.room.messages[value.room.messages.length - 1], participants).sentAt
          : "まだ会話なし",
    },
  };
}

function isBackendRoomWriteSuccess(value: unknown): value is BackendRoomWriteSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    (value.status === "appended" || value.status === "duplicate") &&
    isBackendMessage(value.message)
  );
}

function isBackendRecipientDispatchResult(
  value: unknown,
): value is BackendRecipientDispatchResult {
  return (
    isRecord(value) &&
    typeof value.recipientId === "string" &&
    (value.status === "completed" ||
      value.status === "duplicate" ||
      value.status === "failed" ||
      value.status === "unknown" ||
      value.status === "queued" ||
      value.status === "unsupported") &&
    (value.message === null || isBackendMessage(value.message)) &&
    (value.error === null ||
      (isRecord(value.error) &&
        typeof value.error.code === "string" &&
        typeof value.error.message === "string")) &&
    (value.context === null ||
      ((value.status === "completed" || value.status === "duplicate") &&
        isRoomContextReport(value.context))) &&
    (value.status === "failed" || value.status === "unknown"
      ? value.error !== null
      : value.error === null)
  );
}

function isRoomContextReport(value: unknown): value is RoomContextReport {
  return (
    isRecord(value) &&
    (value.mode === "initial" || value.mode === "resumed" || value.mode === "reconstructed") &&
    Number.isSafeInteger(value.includedMessages) &&
    Number(value.includedMessages) >= 0 &&
    Number.isSafeInteger(value.omittedMessages) &&
    Number(value.omittedMessages) >= 0 &&
    Number.isSafeInteger(value.truncatedMessages) &&
    Number(value.truncatedMessages) >= 0 &&
    Number.isSafeInteger(value.omittedCharacters) &&
    Number(value.omittedCharacters) >= 0 &&
    typeof value.continuitySaved === "boolean"
  );
}

function isBackendRoomDispatchSuccess(value: unknown): value is BackendRoomDispatchSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    typeof value.sourceMessageId === "string" &&
    Array.isArray(value.results) &&
    value.results.every(isBackendRecipientDispatchResult)
  );
}

function isBackendRoomDispatchUnknownsSuccess(
  value: unknown,
): value is BackendRoomDispatchUnknownsSuccess {
  return (
    isRecord(value) &&
    value.ok === true &&
    typeof value.roomId === "string" &&
    Array.isArray(value.unknowns) &&
    value.unknowns.every(
      (unknown) =>
        isRecord(unknown) &&
        typeof unknown.sourceMessageId === "string" &&
        typeof unknown.recipientId === "string" &&
        typeof unknown.code === "string",
    )
  );
}

export async function readDesktopRoom(roomId: string): Promise<DesktopRoomHydration> {
  const value = await invoke<unknown>("desktop_room_read", {
    roomId,
    afterMessageId: null,
    limit: 30,
  });
  if (!isBackendRoomReadSuccess(value)) {
    throw new Error("Desktop Room response was not a valid success result.");
  }
  return hydrateRoom(value);
}

export async function readDesktopAiConnectionStatuses(): Promise<AiConnectionMap> {
  const value = await invoke<unknown>("desktop_ai_connection_status");
  if (!isBackendAiConnectionStatusSuccess(value)) {
    throw new Error("Desktop AI connection status response was not valid.");
  }
  if (
    value.connections.length === 0 ||
    new Set(value.connections.map((connection) => connection.participantId)).size !==
      value.connections.length
  ) {
    throw new Error("Desktop AI connection status response contained invalid IDs.");
  }
  return Object.fromEntries(
    value.connections.map((connection) => [connection.participantId, connection]),
  );
}

export async function readDesktopRoomWorkspaceStatus(
  roomId: string,
): Promise<RoomWorkspaceStatus> {
  const value = await invoke<unknown>("desktop_room_workspace_status", { roomId });
  if (!isBackendRoomWorkspaceStatus(value) || value.roomId !== roomId) {
    throw new Error("Desktop Room workspace status response was not valid.");
  }
  return workspaceView(value);
}

export async function chooseDesktopRoomWorkspace(roomId: string) {
  const value = await invoke<unknown>("desktop_room_workspace_choose", { roomId });
  if (!isBackendRoomWorkspaceStatus(value) || value.roomId !== roomId) {
    throw new Error("Desktop Room workspace selection response was not valid.");
  }
  return { status: workspaceView(value), changed: value.changed };
}

export async function clearDesktopRoomWorkspace(roomId: string) {
  const value = await invoke<unknown>("desktop_room_workspace_clear", { roomId });
  if (
    !isBackendRoomWorkspaceStatus(value) ||
    value.roomId !== roomId ||
    value.mode !== "chatOnly" ||
    !value.changed
  ) {
    throw new Error("Desktop Room workspace clear response was not valid.");
  }
  return workspaceView(value);
}

function conductorStatusView(value: BackendRoomConductorStatus): RoomConductorStatus {
  return {
    roomId: value.roomId,
    conductorId: value.conductorId,
    sendMode: value.sendMode,
  };
}

export async function readDesktopRoomConductorStatus(
  roomId: string,
): Promise<RoomConductorStatus> {
  const value = await invoke<unknown>("desktop_room_conductor_status", { roomId });
  if (!isBackendRoomConductorStatus(value) || value.roomId !== roomId) {
    throw new Error("Desktop Room conductor status response was not valid.");
  }
  return conductorStatusView(value);
}

export async function setDesktopRoomConductor(
  roomId: string,
  conductorId: string,
): Promise<RoomConductorStatus> {
  const value = await invoke<unknown>("desktop_room_conductor_set", {
    roomId,
    conductorId,
  });
  if (
    !isBackendRoomConductorStatus(value) ||
    value.roomId !== roomId ||
    value.conductorId !== conductorId
  ) {
    throw new Error("Desktop Room conductor selection response was not valid.");
  }
  return conductorStatusView(value);
}

export async function saveDesktopRoomConductorMode(
  roomId: string,
  sendMode: "direct" | "conductor",
): Promise<RoomConductorStatus> {
  const value = await invoke<unknown>("desktop_room_conductor_mode_save", {
    roomId,
    sendMode,
  });
  if (
    !isBackendRoomConductorStatus(value) ||
    value.roomId !== roomId ||
    value.sendMode !== sendMode
  ) {
    throw new Error("Desktop Room conductor mode response was not valid.");
  }
  return conductorStatusView(value);
}

export async function clearDesktopRoomConductor(
  roomId: string,
): Promise<RoomConductorStatus> {
  const value = await invoke<unknown>("desktop_room_conductor_clear", { roomId });
  if (
    !isBackendRoomConductorStatus(value) ||
    value.roomId !== roomId ||
    value.conductorId !== null ||
    value.sendMode !== "direct"
  ) {
    throw new Error("Desktop Room conductor clear response was not valid.");
  }
  return conductorStatusView(value);
}

export async function readDesktopRooms(): Promise<DesktopRoomsHydration> {
  const value = await invoke<unknown>("desktop_room_list");
  if (!isBackendRoomCatalogSuccess(value)) {
    throw new Error("Desktop Room catalog response was not valid.");
  }
  if (value.rooms.length === 0) {
    throw new Error("Desktop Room catalog was empty.");
  }
  if (new Set(value.rooms.map((room) => room.id)).size !== value.rooms.length) {
    throw new Error("Desktop Room catalog contained duplicate IDs.");
  }
  const hydrations = await Promise.all(value.rooms.map((room) => readDesktopRoom(room.id)));
  if (
    !hydrations.every(
      (hydration, index) =>
        hydration.room.id === value.rooms[index].id &&
        hydration.room.name === value.rooms[index].name &&
        hydration.room.participantIds.length === value.rooms[index].participantIds.length &&
        hydration.room.participantIds.every(
          (participantId, participantIndex) =>
            participantId === value.rooms[index].participantIds[participantIndex],
        ),
    )
  ) {
    throw new Error("Desktop Room catalog did not match the bounded Room reads.");
  }
  return {
    participants: Object.assign({}, ...hydrations.map((hydration) => hydration.participants)),
    rooms: hydrations.map((hydration) => hydration.room),
  };
}

export async function createDesktopRoom(input: {
  roomId: string;
  name: string;
}): Promise<Room> {
  const value = await invoke<unknown>("desktop_room_create", {
    roomId: input.roomId,
    name: input.name,
  });
  if (!isBackendRoomMutationSuccess(value)) {
    throw new Error("Desktop Room create response was not valid.");
  }
  if (
    (value.status !== "created" && value.status !== "duplicate") ||
    value.room.id !== input.roomId ||
    value.room.name !== input.name ||
    value.room.participantIds.length !== 2 ||
    value.room.participantIds[0] !== ownerParticipantId ||
    value.room.participantIds[1] !== "codex" ||
    value.room.latestMessageAt !== null
  ) {
    throw new Error("Desktop Room create response did not match the request.");
  }
  return {
    id: value.room.id,
    name: value.room.name,
    participantIds: value.room.participantIds,
    messages: [],
    updatedLabel: "まだ会話なし",
  };
}

export async function addDesktopRoomParticipant(input: {
  currentParticipantIds: string[];
  participantId: string;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_add_participant", {
    roomId: input.roomId,
    participantId: input.participantId,
  });
  if (!isBackendRoomMutationSuccess(value)) {
    throw new Error("Desktop Room participant response was not valid.");
  }
  const expected = input.currentParticipantIds.includes(input.participantId)
    ? input.currentParticipantIds
    : [...input.currentParticipantIds, input.participantId];
  if (
    (value.status !== "added" && value.status !== "duplicate") ||
    value.room.id !== input.roomId ||
    value.room.participantIds.length !== expected.length ||
    !value.room.participantIds.every((id, index) => id === expected[index])
  ) {
    throw new Error("Desktop Room participant response did not match the request.");
  }
  return value.room.participantIds;
}

export async function renameDesktopRoom(input: {
  currentParticipantIds: string[];
  name: string;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_rename", {
    roomId: input.roomId,
    name: input.name,
  });
  if (!isBackendRoomMutationSuccess(value)) {
    throw new Error("Desktop Room rename response was not valid.");
  }
  if (
    (value.status !== "renamed" && value.status !== "duplicate") ||
    value.room.id !== input.roomId ||
    value.room.name !== input.name ||
    value.room.participantIds.length !== input.currentParticipantIds.length ||
    !value.room.participantIds.every(
      (id, index) => id === input.currentParticipantIds[index],
    )
  ) {
    throw new Error("Desktop Room rename response did not match the request.");
  }
  return value.room.name;
}

export async function removeDesktopRoomParticipant(input: {
  currentParticipantIds: string[];
  participantId: string;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_remove_participant", {
    roomId: input.roomId,
    participantId: input.participantId,
  });
  if (!isBackendRoomMutationSuccess(value)) {
    throw new Error("Desktop Room participant removal response was not valid.");
  }
  const expected = input.currentParticipantIds.filter((id) => id !== input.participantId);
  if (
    (value.status !== "removed" && value.status !== "duplicate") ||
    value.room.id !== input.roomId ||
    value.room.participantIds.length !== expected.length ||
    !value.room.participantIds.every((id, index) => id === expected[index])
  ) {
    throw new Error("Desktop Room participant removal did not match the request.");
  }
  return value.room.participantIds;
}

export async function deleteDesktopRoom(input: {
  name: string;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_delete", {
    roomId: input.roomId,
  });
  if (!isBackendRoomMutationSuccess(value)) {
    throw new Error("Desktop Room deletion response was not valid.");
  }
  if (
    value.status !== "deleted" ||
    value.room.id !== input.roomId ||
    value.room.name !== input.name
  ) {
    throw new Error("Desktop Room deletion response did not match the request.");
  }
}

export async function backupDesktopRooms() {
  const value = await invoke<unknown>("desktop_room_backup");
  if (!isBackendRoomBackupSuccess(value)) {
    throw new Error("Desktop Room backup response was not valid.");
  }
  return value;
}

export async function restoreLatestDesktopRoomBackup() {
  const value = await invoke<unknown>("desktop_room_restore_latest_backup");
  if (!isBackendRoomBackupSuccess(value)) {
    throw new Error("Desktop Room restore response was not valid.");
  }
  return value;
}

export async function writeDesktopRoomMessage(input: {
  body: string;
  messageId: string;
  participants: ParticipantMap;
  recipientIds: string[];
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_write_message", {
    roomId: input.roomId,
    messageId: input.messageId,
    recipientIds: input.recipientIds,
    body: input.body,
  });
  if (!isBackendRoomWriteSuccess(value)) {
    throw new Error("Desktop Room write response was not valid.");
  }
  if (
    value.message.id !== input.messageId ||
    value.message.roomId !== input.roomId ||
    value.message.authorId !== ownerParticipantId ||
    value.message.body !== input.body ||
    value.message.recipients.length !== input.recipientIds.length ||
    !value.message.recipients.every((id, index) => id === input.recipientIds[index]) ||
    value.message.artifactIds.length !== 0 ||
    value.message.provenance !== undefined
  ) {
    throw new Error("Desktop Room write response did not match the request.");
  }
  return messageView(value.message, input.participants);
}

type DesktopRoomDispatchInput = {
  messageId: string;
  participants: ParticipantMap;
  recipientIds: string[];
  roomId: string;
};

function desktopRoomDispatchView(
  value: BackendRoomDispatchSuccess,
  input: DesktopRoomDispatchInput,
) {
  if (
    value.sourceMessageId !== input.messageId ||
    value.results.length !== input.recipientIds.length ||
    !value.results.every(
      (result, index) => result.recipientId === input.recipientIds[index],
    )
  ) {
    throw new Error("Desktop Room dispatch response did not match the request.");
  }

  const messages = value.results.flatMap((result) => {
    if (
      result.status === "unsupported" ||
      result.status === "queued" ||
      result.status === "failed" ||
      result.status === "unknown"
    ) {
      if (result.message !== null) {
        throw new Error("Non-completed participant returned a Room message.");
      }
      return [];
    }
    const message = result.message;
    if (
      message === null ||
      message.roomId !== input.roomId ||
      message.authorId !== result.recipientId ||
      message.recipients.length !== 1 ||
      message.recipients[0] !== ownerParticipantId ||
      message.body.trim().length === 0 ||
      message.artifactIds.length !== 0
    ) {
      throw new Error("AI dispatch message did not match the Room contract.");
    }
    return [messageView(message, input.participants)];
  });

  return {
    messages,
    contextReports: value.results.flatMap((result) =>
      result.context === null
        ? []
        : [{ participantId: result.recipientId, ...result.context }],
    ),
    unsupportedRecipientIds: value.results
      .filter((result) => result.status === "unsupported")
      .map((result) => result.recipientId),
    queuedRecipientIds: value.results
      .filter((result) => result.status === "queued")
      .map((result) => result.recipientId),
    failedRecipients: value.results
      .filter((result) => result.status === "failed")
      .map((result) => ({
        recipientId: result.recipientId,
        code: result.error!.code,
      })),
    unknownRecipients: value.results
      .filter((result) => result.status === "unknown")
      .map((result) => ({
        recipientId: result.recipientId,
        code: result.error!.code,
      })),
  };
}

export async function dispatchDesktopRoomMessage(input: DesktopRoomDispatchInput) {
  const value = await invoke<unknown>("desktop_room_dispatch_message", {
    roomId: input.roomId,
    messageId: input.messageId,
  });
  if (!isBackendRoomDispatchSuccess(value)) {
    throw new Error("Desktop Room dispatch response was not valid.");
  }
  return desktopRoomDispatchView(value, input);
}

export async function dispatchDesktopRoomRecipient(input: {
  messageId: string;
  participantId: string;
  participants: ParticipantMap;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_dispatch_recipient", {
    roomId: input.roomId,
    messageId: input.messageId,
    recipientId: input.participantId,
  });
  if (!isBackendRoomDispatchSuccess(value)) {
    throw new Error("Desktop Room recipient dispatch response was not valid.");
  }
  return desktopRoomDispatchView(value, {
    messageId: input.messageId,
    participants: input.participants,
    recipientIds: [input.participantId],
    roomId: input.roomId,
  });
}

export async function orchestrateDesktopRoomMessage(input: {
  messageId: string;
  participants: ParticipantMap;
  roomId: string;
}) {
  const value = await invoke<unknown>("desktop_room_orchestrate_message", {
    roomId: input.roomId,
    messageId: input.messageId,
  });
  if (!isBackendRoomOrchestrationResult(value)) {
    throw new Error("Desktop Room orchestration response was not valid.");
  }
  if (value.finalMessage === null) {
    return { status: value.status, message: null } as const;
  }
  if (
    value.finalMessage.roomId !== input.roomId ||
    value.finalMessage.recipients.length !== 1 ||
    value.finalMessage.recipients[0] !== ownerParticipantId ||
    value.finalMessage.body.trim().length === 0 ||
    value.finalMessage.artifactIds.length !== 0
  ) {
    throw new Error("Conductor final message did not match the Room contract.");
  }
  return {
    status: value.status,
    message: messageView(value.finalMessage, input.participants),
  } as const;
}

export async function resetDesktopRoomAiContinuity(
  roomId: string,
  participantId: string,
) {
  const value = await invoke<unknown>("desktop_room_ai_continuity_reset", {
    roomId,
    participantId,
  });
  if (
    !isRecord(value) ||
    value.ok !== true ||
    value.roomId !== roomId ||
    value.participantId !== participantId ||
    typeof value.changed !== "boolean"
  ) {
    throw new Error("Desktop Room continuity reset response was not valid.");
  }
  return (value as BackendRoomContinuityResetSuccess).changed;
}

export async function readDesktopRoomDispatchUnknowns(roomId: string) {
  const value = await invoke<unknown>("desktop_room_dispatch_unknowns", { roomId });
  if (!isBackendRoomDispatchUnknownsSuccess(value) || value.roomId !== roomId) {
    throw new Error("Desktop Room dispatch unknowns response was not valid.");
  }
  return value.unknowns;
}

export function browserBridgeReplyView(
  value: unknown,
  participants: ParticipantMap,
): { roomId: string; message: ReturnType<typeof messageView> } {
  if (
    !isBackendMessage(value) ||
    value.authorId !== "gemini" ||
    value.recipients.length !== 1 ||
    value.recipients[0] !== ownerParticipantId ||
    value.artifactIds.length !== 0 ||
    value.body.trim().length === 0
  ) {
    throw new Error("Browser Bridge reply did not match the Room contract.");
  }
  return { roomId: value.roomId, message: messageView(value, participants) };
}
