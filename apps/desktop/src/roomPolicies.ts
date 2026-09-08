const bundledRoomIds = new Set(["moe-dev-room", "comparison-room", "mcp-lab"]);

export function isBundledRoom(roomId: string) {
  return bundledRoomIds.has(roomId);
}
