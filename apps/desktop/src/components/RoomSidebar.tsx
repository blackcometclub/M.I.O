import type { Room } from "../types";
import { useUiPreferences } from "../uiPreferences";

type RoomSidebarProps = {
  activeRoomId: string;
  coreLabel: string;
  coreReady: boolean;
  onCreateRoom: () => void;
  onSelectRoom: (roomId: string) => void;
  rooms: Room[];
};

export function RoomSidebar({
  activeRoomId,
  coreLabel,
  coreReady,
  onCreateRoom,
  onSelectRoom,
  rooms,
}: RoomSidebarProps) {
  const { setSidebarFontScale, sidebarFontScale, t } = useUiPreferences();
  const displayUpdatedLabel = (room: Room) => room.messages.length === 0
    ? t("noConversation")
    : room.updatedLabel === "いま" ? t("now") : room.updatedLabel;
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
            className="room-list-item"
            key={room.id}
            onClick={() => onSelectRoom(room.id)}
            type="button"
          >
            <span className="room-list-icon" aria-hidden="true">
              {room.name.slice(0, 1)}
            </span>
            <span className="room-list-copy">
              <strong>{room.name}</strong>
              <span>
                {t("aiCount", { count: Math.max(room.participantIds.length - 1, 0) })} · {displayUpdatedLabel(room)}
              </span>
            </span>
          </button>
        ))}
      </nav>

      <footer className="sidebar-footer">
        <span className={`core-dot ${coreReady ? "is-ready" : ""}`} />
        <span>{coreLabel}</span>
      </footer>
    </aside>
  );
}
