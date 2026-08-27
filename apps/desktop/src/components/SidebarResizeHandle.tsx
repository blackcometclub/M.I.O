import { useRef, type KeyboardEvent, type PointerEvent } from "react";

import { useUiPreferences } from "../uiPreferences";

type DragState = { pointerId: number; startWidth: number; startX: number };

export function SidebarResizeHandle() {
  const { setSidebarWidth, sidebarWidth, t } = useUiPreferences();
  const dragRef = useRef<DragState | null>(null);

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { pointerId: event.pointerId, startWidth: sidebarWidth, startX: event.clientX };
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    setSidebarWidth(drag.startWidth + event.clientX - drag.startX);
  }

  function handlePointerEnd(event: PointerEvent<HTMLDivElement>) {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    dragRef.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "ArrowLeft") setSidebarWidth(sidebarWidth - (event.shiftKey ? 40 : 10));
    else if (event.key === "ArrowRight") setSidebarWidth(sidebarWidth + (event.shiftKey ? 40 : 10));
    else if (event.key === "Home") setSidebarWidth(220);
    else return;
    event.preventDefault();
  }

  return (
    <div
      aria-label={t("resizeRoomTree")}
      aria-orientation="vertical"
      aria-valuemax={420}
      aria-valuemin={180}
      aria-valuenow={Math.round(sidebarWidth)}
      className="sidebar-resize-handle"
      onKeyDown={handleKeyDown}
      onPointerCancel={handlePointerEnd}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      role="separator"
      tabIndex={0}
      title={t("resizeRoomTree")}
    />
  );
}
