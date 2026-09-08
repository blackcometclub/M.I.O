import { useEffect, useRef, useState, type MouseEvent } from "react";

import { isBundledRoom } from "../roomPolicies";
import type { Room } from "../types";
import { useUiPreferences } from "../uiPreferences";

type RoomSidebarProps = {
  activeRoomId: string;
  activeTurnRoomId: string | null;
  coreLabel: string;
  coreReady: boolean;
  isBusy: boolean;
  onCreateRoom: () => void;
  onDeleteRoom: (roomId: string) => Promise<boolean>;
  onOpenRoomSettings: (roomId: string) => void;
  onSelectRoom: (roomId: string) => void;
  rooms: Room[];
};

type RoomContextMenuState = {
  deleteArmed: boolean;
  deleteFailed: boolean;
  roomId: string;
  x: number;
  y: number;
};

export function RoomSidebar({
  activeRoomId,
  activeTurnRoomId,
  coreLabel,
  coreReady,
  isBusy,
  onCreateRoom,
  onDeleteRoom,
  onOpenRoomSettings,
  onSelectRoom,
  rooms,
}: RoomSidebarProps) {
  const { setSidebarFontScale, sidebarFontScale, t } = useUiPreferences();
  const [contextMenu, setContextMenu] = useState<RoomContextMenuState | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const contextRoom = contextMenu
    ? rooms.find((room) => room.id === contextMenu.roomId) ?? null
    : null;
  const displayUpdatedLabel = (room: Room) => room.messages.length === 0
    ? t("noConversation")
    : room.updatedLabel === "いま" ? t("now") : room.updatedLabel;
  const contextRoomBusy = isBusy || contextRoom?.id === activeTurnRoomId;

  useEffect(() => {
    if (!contextMenu) return;
    const close = (event: PointerEvent) => {
      if (event.target instanceof Node && contextMenuRef.current?.contains(event.target)) return;
      setContextMenu(null);
    };
    const closeFromKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setContextMenu(null);
    };
    const closeFromWindow = () => setContextMenu(null);
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", closeFromKey);
    window.addEventListener("blur", closeFromWindow);
    window.addEventListener("resize", closeFromWindow);
    const frame = window.requestAnimationFrame(() => {
      contextMenuRef.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    });
    return () => {
      window.cancelAnimationFrame(frame);
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", closeFromKey);
      window.removeEventListener("blur", closeFromWindow);
      window.removeEventListener("resize", closeFromWindow);
    };
  }, [contextMenu]);

  function openContextMenu(event: MouseEvent, roomId: string) {
    event.preventDefault();
    const menuWidth = 196;
    const menuHeight = 184;
    setContextMenu({
      deleteArmed: false,
      deleteFailed: false,
      roomId,
      x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
      y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
    });
  }

  async function confirmDelete() {
    if (!contextRoom || isBundledRoom(contextRoom.id) || contextRoomBusy) return;
    if (await onDeleteRoom(contextRoom.id)) {
      setContextMenu(null);
      return;
    }
    setContextMenu((current) => current ? { ...current, deleteFailed: true } : null);
  }

  return (
    <aside className="room-sidebar">
      <div className="sidebar-title-row">
        <div>
          <span className="sidebar-eyebrow">ROOMS</span>
          <h2>{t("rooms")}</h2>
        </div>
        <div className="sidebar-title-actions">
          <div className="sidebar-font-controls">
            <button
              disabled={sidebarFontScale <= 0.8}
              onClick={() => setSidebarFontScale(sidebarFontScale - 0.05)}
              title={t("sidebarTextSmaller")}
              type="button"
            ><span aria-hidden="true">A−</span><span className="sr-only">{t("sidebarTextSmaller")}</span></button>
            <button
              disabled={sidebarFontScale >= 1.3}
              onClick={() => setSidebarFontScale(sidebarFontScale + 0.05)}
              title={t("sidebarTextLarger")}
              type="button"
            ><span aria-hidden="true">A+</span><span className="sr-only">{t("sidebarTextLarger")}</span></button>
          </div>
          <button
            className="sidebar-add-button"
            onClick={onCreateRoom}
            title={t("newRoom")}
            type="button"
          >
            <span aria-hidden="true">＋</span>
            <span className="sr-only">{t("newRoom")}</span>
          </button>
        </div>
      </div>

      <nav aria-label={t("roomList")} className="room-list">
        {rooms.map((room) => (
          <button
            aria-current={room.id === activeRoomId ? "page" : undefined}
            className={`room-list-item ${room.id === activeTurnRoomId ? "is-processing" : ""}`}
            key={room.id}
            onClick={() => onSelectRoom(room.id)}
            onContextMenu={(event) => openContextMenu(event, room.id)}
            type="button"
          >
            <span className="room-list-icon" aria-hidden="true">
              {room.name.slice(0, 1)}
            </span>
            <span className="room-list-copy">
              <span className="room-list-title">
                <strong>{room.name}</strong>
                {room.id === activeTurnRoomId ? (
                  <span className="room-processing-spinner" aria-hidden="true" />
                ) : null}
              </span>
              <span>
                {t("aiCount", { count: Math.max(room.participantIds.length - 1, 0) })} · {room.id === activeTurnRoomId ? t("roomAiWorking") : displayUpdatedLabel(room)}
              </span>
            </span>
          </button>
        ))}
      </nav>

      {contextMenu && contextRoom ? (
        <div
          aria-label={t("roomActions", { name: contextRoom.name })}
          className="room-context-menu"
          onContextMenu={(event) => event.preventDefault()}
          ref={contextMenuRef}
          role="menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <strong className="room-context-menu-title">{contextRoom.name}</strong>
          {!contextMenu.deleteArmed ? (
            <>
              <button
                onClick={() => {
                  onSelectRoom(contextRoom.id);
                  setContextMenu(null);
                }}
                role="menuitem"
                type="button"
              >
                {t("openRoom")}
              </button>
              <button
                disabled={contextRoomBusy}
                onClick={() => {
                  onOpenRoomSettings(contextRoom.id);
                  setContextMenu(null);
                }}
                role="menuitem"
                type="button"
              >
                {t("renameRoom")}
              </button>
              <button
                className="is-danger"
                disabled={isBundledRoom(contextRoom.id) || contextRoomBusy}
                onClick={() => setContextMenu((current) => current ? { ...current, deleteArmed: true, deleteFailed: false } : null)}
                role="menuitem"
                type="button"
              >
                {t("delete")}
              </button>
              {isBundledRoom(contextRoom.id) ? (
                <span className="room-context-menu-help">{t("protectedRoom")}</span>
              ) : null}
            </>
          ) : (
            <div className="room-context-delete-confirm">
              <span>{t("deleteRoomConfirm", { name: contextRoom.name })}</span>
              <div>
                <button onClick={() => setContextMenu((current) => current ? { ...current, deleteArmed: false, deleteFailed: false } : null)} type="button">
                  {t("cancel")}
                </button>
                <button className="is-danger" disabled={contextRoomBusy} onClick={() => void confirmDelete()} type="button">
                  {t("reallyDelete")}
                </button>
              </div>
              {contextMenu.deleteFailed ? (
                <span className="room-context-menu-error" role="alert">{t("deleteRoomFailed")}</span>
              ) : null}
            </div>
          )}
        </div>
      ) : null}

      <footer className="sidebar-footer">
        <span className={`core-dot ${coreReady ? "is-ready" : ""}`} />
        <span>{coreLabel}</span>
      </footer>
    </aside>
  );
}
