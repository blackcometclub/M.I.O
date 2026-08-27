export type ArtworkPlacement = {
  scale: number;
  x: number;
  y: number;
};

export type ArtworkSource = {
  dataUrl: string;
  fileName: string;
  placement: ArtworkPlacement;
};

export type ArtworkCanvasLayout = {
  width: number;
  height: number;
  sidebarWidth: number;
  gap: number;
};

export type ArtworkEditorRequest = ArtworkSource & {
  canvasLayout: ArtworkCanvasLayout;
  locale: "ja" | "en";
};

export const defaultArtworkPlacement: ArtworkPlacement = {
  scale: 1,
  x: 0.5,
  y: 0.5,
};

export const artworkEditorEvents = {
  apply: "artwork-editor-apply",
  cancel: "artwork-editor-cancel",
  load: "artwork-editor-load",
  ready: "artwork-editor-ready",
} as const;

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

export function normalizeArtworkPlacement(
  placement: ArtworkPlacement,
): ArtworkPlacement {
  return {
    scale: clamp(Number.isFinite(placement.scale) ? placement.scale : 1, 0.05, 2),
    x: clamp(Number.isFinite(placement.x) ? placement.x : 0.5, -0.5, 1.5),
    y: clamp(Number.isFinite(placement.y) ? placement.y : 0.5, -0.5, 1.5),
  };
}
