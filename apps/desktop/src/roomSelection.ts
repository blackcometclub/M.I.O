type RoomSelectionSource = "backend" | "browserDemo";

function storageKey(source: RoomSelectionSource) {
  return `moe-active-room-v1-${source}`;
}

export function readRememberedRoomId(source: RoomSelectionSource): string | null {
  try {
    const id = localStorage.getItem(storageKey(source));
    return id && id.length <= 256 && !/[\u0000-\u0020\u007f]/u.test(id) ? id : null;
  } catch {
    // Room navigation remains available when browser storage is unavailable.
    return null;
  }
}

export function rememberRoomId(source: RoomSelectionSource, roomId: string) {
  try {
    localStorage.setItem(storageKey(source), roomId);
  } catch {
    // A failed preference write must not interrupt conversation or navigation.
  }
}

export function resolveSelectedRoom<T extends { id: string }>(
  rooms: readonly T[],
  preferredId: string | null,
): T | undefined {
  return rooms.find((room) => room.id === preferredId) ?? rooms[0];
}
