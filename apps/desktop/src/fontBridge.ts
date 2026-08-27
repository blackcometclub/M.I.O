import { invoke } from "@tauri-apps/api/core";

let cachedFamilies: Promise<string[]> | null = null;

function normalizeFamilies(value: unknown) {
  if (!Array.isArray(value)) return [];
  const families = value.filter(
    (item): item is string => typeof item === "string" && item.length > 0 && item.length <= 256 && !/[\u0000-\u001f\u007f]/u.test(item),
  );
  return [...new Map(families.map((family) => [family.toLocaleLowerCase(), family])).values()]
    .sort((left, right) => left.localeCompare(right));
}

export function listInstalledFontFamilies() {
  if (!cachedFamilies) {
    cachedFamilies = "__TAURI_INTERNALS__" in window
      ? invoke<unknown>("desktop_system_font_families").then(normalizeFamilies).catch(() => [])
      : Promise.resolve([]);
  }
  return cachedFamilies;
}
