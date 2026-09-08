import { useEffect } from "react";

const editableInputTypes = new Set([
  "email",
  "number",
  "password",
  "search",
  "tel",
  "text",
  "url",
]);

function keepsNativeEditingMenu(target: EventTarget | null) {
  if (!(target instanceof Element)) return false;
  const editable = target.closest("input, textarea, [contenteditable]");
  if (editable instanceof HTMLTextAreaElement) {
    return !editable.disabled;
  }
  if (editable instanceof HTMLInputElement) {
    return !editable.disabled && editableInputTypes.has(editable.type);
  }
  return editable instanceof HTMLElement && editable.isContentEditable;
}

export function useContextMenuPolicy() {
  useEffect(() => {
    const suppressUnexpectedNativeMenu = (event: MouseEvent) => {
      if (!keepsNativeEditingMenu(event.target)) {
        event.preventDefault();
      }
    };
    document.addEventListener("contextmenu", suppressUnexpectedNativeMenu);
    return () => document.removeEventListener("contextmenu", suppressUnexpectedNativeMenu);
  }, []);
}
