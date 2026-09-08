import { invoke } from "@tauri-apps/api/core";

export type CommandConfirmationAction =
  | "gitPush"
  | "packageInstall"
  | "cargoFetch"
  | "unregisteredTool"
  | "credentialUse"
  | "administratorOperation"
  | "destructiveOperation";

export type CommandConfirmationReason =
  | "externalMutation"
  | "networkDownload"
  | "unregisteredTool"
  | "credentialUse"
  | "privilegeExpansion"
  | "destructiveChange";

export type CommandConfirmationDecision =
  | "allowOnce"
  | "allowRoomSession"
  | "deny"
  | "dismissed";

export type CommandConfirmationRequest = {
  requestId: string;
  roomId: string;
  action: CommandConfirmationAction;
  reason: CommandConfirmationReason;
  targetLabel: string;
  roomSessionAllowed: boolean;
};

type CommandConfirmationPending = {
  ok: true;
  roomId: string;
  requests: CommandConfirmationRequest[];
};

export type CommandConfirmationResolution = {
  ok: true;
  requestId: string;
  status: "authorizationReady" | "denied";
  denial:
    | "ownerDenied"
    | "dismissed"
    | "timedOut"
    | "sessionLifetimeNotAllowed"
    | null;
};

type CommandRoomSessionStatus = {
  ok: true;
  roomId: string;
  active: boolean;
};

const actions = new Set<CommandConfirmationAction>([
  "gitPush",
  "packageInstall",
  "cargoFetch",
  "unregisteredTool",
  "credentialUse",
  "administratorOperation",
  "destructiveOperation",
]);
const reasons = new Set<CommandConfirmationReason>([
  "externalMutation",
  "networkDownload",
  "unregisteredTool",
  "credentialUse",
  "privilegeExpansion",
  "destructiveChange",
]);
const decisions = new Set<CommandConfirmationDecision>([
  "allowOnce",
  "allowRoomSession",
  "deny",
  "dismissed",
]);
const denials = new Set<NonNullable<CommandConfirmationResolution["denial"]>>([
  "ownerDenied",
  "dismissed",
  "timedOut",
  "sessionLifetimeNotAllowed",
]);
const reasonForAction: Record<CommandConfirmationAction, CommandConfirmationReason> = {
  gitPush: "externalMutation",
  packageInstall: "networkDownload",
  cargoFetch: "networkDownload",
  unregisteredTool: "unregisteredTool",
  credentialUse: "credentialUse",
  administratorOperation: "privilegeExpansion",
  destructiveOperation: "destructiveChange",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]) {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length
    && actual.every((key, index) => key === expected[index]);
}

function isIdentifier(value: unknown, maximumBytes: number): value is string {
  return typeof value === "string"
    && value.length > 0
    && new TextEncoder().encode(value).length <= maximumBytes
    && /^[\x21-\x7e]+$/u.test(value);
}

function isTargetLabel(value: unknown): value is string {
  return typeof value === "string"
    && value.length > 0
    && Array.from(value).length <= 256
    && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function isAction(value: unknown): value is CommandConfirmationAction {
  return typeof value === "string" && actions.has(value as CommandConfirmationAction);
}

function isReason(value: unknown): value is CommandConfirmationReason {
  return typeof value === "string" && reasons.has(value as CommandConfirmationReason);
}

function isRequest(value: unknown, roomId: string): value is CommandConfirmationRequest {
  if (!isRecord(value) || !hasExactKeys(value, [
    "requestId",
    "roomId",
    "action",
    "reason",
    "targetLabel",
    "roomSessionAllowed",
  ])) {
    return false;
  }
  if (
    !isIdentifier(value.requestId, 128)
    || value.roomId !== roomId
    || !isAction(value.action)
    || !isReason(value.reason)
    || value.reason !== reasonForAction[value.action]
    || !isTargetLabel(value.targetLabel)
    || typeof value.roomSessionAllowed !== "boolean"
  ) {
    return false;
  }
  return value.roomSessionAllowed
    === ["gitPush", "packageInstall", "cargoFetch"].includes(value.action);
}

function isPending(value: unknown, roomId: string): value is CommandConfirmationPending {
  if (!isRecord(value) || !hasExactKeys(value, ["ok", "roomId", "requests"])) {
    return false;
  }
  if (value.ok !== true || value.roomId !== roomId || !Array.isArray(value.requests)) {
    return false;
  }
  const requestIds = new Set<string>();
  return value.requests.every((request) => {
    if (!isRequest(request, roomId) || requestIds.has(request.requestId)) return false;
    requestIds.add(request.requestId);
    return true;
  });
}

function isResolution(
  value: unknown,
  requestId: string,
): value is CommandConfirmationResolution {
  if (!isRecord(value) || !hasExactKeys(value, [
    "ok",
    "requestId",
    "status",
    "denial",
  ])) {
    return false;
  }
  if (value.ok !== true || value.requestId !== requestId) return false;
  if (value.status === "authorizationReady") return value.denial === null;
  return value.status === "denied"
    && typeof value.denial === "string"
    && denials.has(value.denial as NonNullable<CommandConfirmationResolution["denial"]>);
}

function isRoomSessionStatus(
  value: unknown,
  roomId: string,
): value is CommandRoomSessionStatus {
  return isRecord(value)
    && hasExactKeys(value, ["ok", "roomId", "active"])
    && value.ok === true
    && value.roomId === roomId
    && typeof value.active === "boolean";
}

function requireIdentifier(value: string, maximumBytes: number) {
  if (!isIdentifier(value, maximumBytes)) {
    throw new Error("Command confirmation identifier was invalid.");
  }
}

export async function readDesktopCommandConfirmations(roomId: string) {
  requireIdentifier(roomId, 256);
  const value = await invoke<unknown>("desktop_command_confirmation_pending", { roomId });
  if (!isPending(value, roomId)) {
    throw new Error("Desktop command confirmation response was invalid.");
  }
  return value.requests;
}

export async function activateDesktopCommandRoomSession(roomId: string) {
  requireIdentifier(roomId, 256);
  const value = await invoke<unknown>("desktop_command_room_session_activate", { roomId });
  if (!isRoomSessionStatus(value, roomId)) {
    throw new Error("Desktop command Room session response was invalid.");
  }
  return value;
}

export async function deactivateDesktopCommandRoomSession(roomId: string) {
  requireIdentifier(roomId, 256);
  const value = await invoke<unknown>("desktop_command_room_session_deactivate", { roomId });
  if (!isRoomSessionStatus(value, roomId) || value.active) {
    throw new Error("Desktop command Room session response was invalid.");
  }
  return value;
}

export async function resolveDesktopCommandConfirmation(input: {
  roomId: string;
  requestId: string;
  decision: CommandConfirmationDecision;
}) {
  requireIdentifier(input.roomId, 256);
  requireIdentifier(input.requestId, 128);
  if (!decisions.has(input.decision)) {
    throw new Error("Command confirmation decision was invalid.");
  }
  const value = await invoke<unknown>("desktop_command_confirmation_resolve", input);
  if (!isResolution(value, input.requestId)) {
    throw new Error("Desktop command confirmation resolution was invalid.");
  }
  return value;
}
