import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PointerEvent } from "react";

const directions = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"] as const;

export function WindowResizeHandles() {
  if (!("__TAURI_INTERNALS__" in window)) return null;
  function beginResize(event: PointerEvent<HTMLDivElement>, direction: (typeof directions)[number]) {
    if (event.button !== 0) return;
    event.preventDefault();
    void getCurrentWindow().startResizeDragging(direction).catch((reason: unknown) => console.error(`Window resize ${direction} failed`, reason));
  }
  return <>{directions.map((direction) => <div aria-hidden="true" className={`window-resize-handle is-${direction.toLowerCase()}`} key={direction} onPointerDown={(event) => beginResize(event, direction)} />)}</>;
}
