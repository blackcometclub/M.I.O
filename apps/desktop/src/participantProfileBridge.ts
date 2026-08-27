import { invoke } from "@tauri-apps/api/core";

import type { ParticipantProfile } from "./types";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isProfile(value: unknown): value is ParticipantProfile {
  if (!isRecord(value)
    || typeof value.participantId !== "string"
    || typeof value.displayName !== "string"
    || typeof value.aiInstructions !== "string"
    || !["providerDefault", "chatOnly", "workspaceRead", "workspaceWrite"].includes(String(value.aiAccessMode))) return false;
  if (value.avatar === null) return true;
  return isRecord(value.avatar)
    && typeof value.avatar.dataUrl === "string"
    && typeof value.avatar.scale === "number"
    && typeof value.avatar.x === "number"
    && typeof value.avatar.y === "number";
}

export async function readParticipantProfiles() {
  if (!("__TAURI_INTERNALS__" in window)) return [];
  const value = await invoke<unknown>("desktop_participant_profiles");
  if (!Array.isArray(value) || !value.every(isProfile)) throw new Error("Participant profiles were not valid.");
  return value;
}

export async function saveParticipantProfile(profile: ParticipantProfile) {
  if (!("__TAURI_INTERNALS__" in window)) return profile;
  const value = await invoke<unknown>("desktop_participant_profile_save", {
    participantId: profile.participantId,
    displayName: profile.displayName,
    avatar: profile.avatar,
    aiInstructions: profile.aiInstructions,
    aiAccessMode: profile.aiAccessMode,
  });
  if (!isProfile(value)) throw new Error("The saved participant profile was not valid.");
  return value;
}
