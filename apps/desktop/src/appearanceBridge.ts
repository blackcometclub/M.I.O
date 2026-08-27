import { invoke } from "@tauri-apps/api/core";

import type { ArtworkSource } from "./artwork";

export type AppearanceImage = {
  dataUrl: string;
  fileName: string;
};

export type AppearanceSettings = {
  backgroundColor: string;
  backgroundImage: AppearanceImage | null;
  artwork: ArtworkSource | null;
};

export const defaultAppearanceSettings: AppearanceSettings = {
  backgroundColor: "#ffc126",
  backgroundImage: null,
  artwork: null,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isImage(value: unknown): value is AppearanceImage {
  return isRecord(value)
    && typeof value.dataUrl === "string"
    && typeof value.fileName === "string";
}

function isArtwork(value: unknown): value is ArtworkSource {
  const placement = isRecord(value) ? value.placement : null;
  return isImage(value)
    && isRecord(placement)
    && typeof placement.scale === "number"
    && typeof placement.x === "number"
    && typeof placement.y === "number";
}

function isAppearance(value: unknown): value is AppearanceSettings {
  return isRecord(value)
    && typeof value.backgroundColor === "string"
    && (value.backgroundImage === null || isImage(value.backgroundImage))
    && (value.artwork === null || isArtwork(value.artwork));
}

export async function readAppearanceSettings() {
  if (!("__TAURI_INTERNALS__" in window)) return defaultAppearanceSettings;
  const value = await invoke<unknown>("desktop_appearance_settings");
  if (!isAppearance(value)) throw new Error("Appearance settings were not valid.");
  return value;
}

export async function saveAppearanceSettings(appearance: AppearanceSettings) {
  if (!("__TAURI_INTERNALS__" in window)) return appearance;
  const value = await invoke<unknown>("desktop_appearance_settings_save", { appearance });
  if (!isAppearance(value)) throw new Error("Saved appearance settings were not valid.");
  return value;
}
