import { type CSSProperties, useEffect, useRef, useState } from "react";

import {
  defaultArtworkPlacement,
  type ArtworkCanvasLayout,
  type ArtworkSource,
} from "../artwork";
import { openArtworkEditor, readFileAsDataUrl } from "../artworkEditorBridge";
import {
  defaultAppearanceSettings,
  readAppearanceSettings,
  saveAppearanceSettings,
  type AppearanceSettings,
} from "../appearanceBridge";
import { useUiPreferences } from "../uiPreferences";

type SurfaceStyle = CSSProperties & {
  "--surface-color": string;
  "--surface-image": string;
};

export type AppearanceSaveStatus = "loading" | "saving" | "saved" | "error";
const maximumImageBytes = 8 * 1024 * 1024;

function supportedImage(file: File) {
  return ["image/png", "image/jpeg", "image/webp"].includes(file.type)
    && file.size <= maximumImageBytes;
}

export function useAppearance() {
  const { locale } = useUiPreferences();
  const text = (japanese: string, english: string) => locale === "ja" ? japanese : english;
  const [isAppearanceOpen, setAppearanceOpen] = useState(false);
  const [appearance, setAppearance] = useState<AppearanceSettings>(defaultAppearanceSettings);
  const [appearanceSaveStatus, setAppearanceSaveStatus] = useState<AppearanceSaveStatus>("loading");
  const [artworkEditorMessage, setArtworkEditorMessage] = useState<string | null>(null);
  const workbenchRef = useRef<HTMLElement>(null);
  const appearanceRef = useRef(appearance);
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const saveSequenceRef = useRef(0);
  const modifiedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    void readAppearanceSettings()
      .then((stored) => {
        if (cancelled || modifiedRef.current) return;
        appearanceRef.current = stored;
        setAppearance(stored);
        setAppearanceSaveStatus("saved");
      })
      .catch(() => {
        if (!cancelled) setAppearanceSaveStatus("error");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function updateAppearance(next: AppearanceSettings) {
    modifiedRef.current = true;
    appearanceRef.current = next;
    setAppearance(next);
    setAppearanceSaveStatus("saving");
    const sequence = ++saveSequenceRef.current;
    const operation = saveQueueRef.current
      .catch(() => undefined)
      .then(() => saveAppearanceSettings(next));
    saveQueueRef.current = operation.then(() => undefined, () => undefined);
    void operation.then(
      () => {
        if (sequence === saveSequenceRef.current) setAppearanceSaveStatus("saved");
      },
      () => {
        if (sequence === saveSequenceRef.current) setAppearanceSaveStatus("error");
      },
    );
  }

  async function chooseBackgroundImage(file: File) {
    if (!supportedImage(file)) {
      setArtworkEditorMessage(text(
        "PNG・JPEG・WebPの8MB以下の画像を選んでください。",
        "Choose a PNG, JPEG, or WebP image up to 8 MB.",
      ));
      return;
    }
    try {
      const dataUrl = await readFileAsDataUrl(file);
      updateAppearance({
        ...appearanceRef.current,
        backgroundImage: { dataUrl, fileName: file.name },
      });
      setArtworkEditorMessage(null);
    } catch {
      setArtworkEditorMessage(text("画像を読み込めませんでした。", "The image could not be loaded."));
    }
  }

  async function configureArtwork(source: ArtworkSource) {
    setArtworkEditorMessage(text("画像調整画面を開いています…", "Opening the artwork editor…"));

    try {
      const workbench = workbenchRef.current;
      const sidebar = workbench?.querySelector<HTMLElement>(".room-sidebar");
      const workspace = workbench?.querySelector<HTMLElement>(".room-workspace");

      if (!workbench || !sidebar || !workspace) {
        throw new Error(text("M.I.O.画面のサイズを取得できませんでした。", "The M.I.O. layout size could not be measured."));
      }

      const workbenchBounds = workbench.getBoundingClientRect();
      const sidebarBounds = sidebar.getBoundingClientRect();
      const workspaceBounds = workspace.getBoundingClientRect();
      const canvasLayout: ArtworkCanvasLayout = {
        width: workbenchBounds.width,
        height: workbenchBounds.height,
        sidebarWidth: sidebarBounds.width,
        gap: Math.max(0, workspaceBounds.left - sidebarBounds.right),
      };

      const placement = await openArtworkEditor({ ...source, canvasLayout, locale });
      if (placement) {
        updateAppearance({
          ...appearanceRef.current,
          artwork: { ...source, placement },
        });
      }
      setArtworkEditorMessage(null);
    } catch (error) {
      setArtworkEditorMessage(
        error instanceof Error ? error.message : text("画像調整画面を開けませんでした。", "The artwork editor could not be opened."),
      );
    }
  }

  async function chooseArtworkImage(file: File) {
    if (!supportedImage(file)) {
      setArtworkEditorMessage(text(
        "PNG・JPEG・WebPの8MB以下の画像を選んでください。",
        "Choose a PNG, JPEG, or WebP image up to 8 MB.",
      ));
      return;
    }

    try {
      const dataUrl = await readFileAsDataUrl(file);
      await configureArtwork({
        dataUrl,
        fileName: file.name,
        placement: defaultArtworkPlacement,
      });
    } catch (error) {
      setArtworkEditorMessage(
        error instanceof Error ? error.message : text("画像を読み込めませんでした。", "The image could not be loaded."),
      );
    }
  }

  function editArtwork() {
    if (appearance.artwork) {
      void configureArtwork(appearance.artwork);
    }
  }

  const surfaceStyle: SurfaceStyle = {
    "--surface-color": appearance.backgroundColor,
    "--surface-image": appearance.backgroundImage ? `url("${appearance.backgroundImage.dataUrl}")` : "none",
  };

  return {
    artwork: appearance.artwork,
    artworkEditorMessage,
    appearanceSaveStatus,
    backgroundColor: appearance.backgroundColor,
    backgroundImageUrl: appearance.backgroundImage?.dataUrl ?? null,
    chooseArtworkImage,
    chooseBackgroundImage,
    clearArtworkImage: () => updateAppearance({ ...appearanceRef.current, artwork: null }),
    clearBackgroundImage: () => updateAppearance({ ...appearanceRef.current, backgroundImage: null }),
    closeAppearance: () => setAppearanceOpen(false),
    editArtwork,
    isAppearanceOpen,
    setBackgroundColor: (backgroundColor: string) => updateAppearance({
      ...appearanceRef.current,
      backgroundColor,
    }),
    surfaceStyle,
    toggleAppearance: () => setAppearanceOpen((isOpen) => !isOpen),
    workbenchRef,
  };
}
