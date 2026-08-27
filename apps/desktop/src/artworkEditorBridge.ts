import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import {
  artworkEditorEvents,
  normalizeArtworkPlacement,
  type ArtworkEditorRequest,
  type ArtworkPlacement,
} from "./artwork";

const editorLabel = "artwork-editor";

export function readFileAsDataUrl(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      if (typeof reader.result === "string") {
        resolve(reader.result);
      } else {
        reject(new Error("画像を読み込めませんでした。"));
      }
    });
    reader.addEventListener("error", () => {
      reject(reader.error ?? new Error("画像を読み込めませんでした。"));
    });
    reader.readAsDataURL(file);
  });
}

export async function openArtworkEditor(
  request: ArtworkEditorRequest,
): Promise<ArtworkPlacement | null> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return request.placement;
  }

  const existingEditor = await WebviewWindow.getByLabel(editorLabel);
  if (existingEditor) {
    await existingEditor.close();
  }

  return new Promise<ArtworkPlacement | null>((resolve, reject) => {
    const unlisteners: UnlistenFn[] = [];
    let isSettled = false;
    let requestWasSent = false;

    function cleanup() {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    }

    function finish(result: ArtworkPlacement | null) {
      if (isSettled) {
        return;
      }
      isSettled = true;
      cleanup();
      resolve(result);
    }

    function fail(error: unknown) {
      if (isSettled) {
        return;
      }
      isSettled = true;
      cleanup();
      reject(error instanceof Error ? error : new Error(String(error)));
    }

    void (async () => {
      unlisteners.push(
        await listen(artworkEditorEvents.ready, () => {
          if (requestWasSent) {
            return;
          }
          requestWasSent = true;
          void emitTo(editorLabel, artworkEditorEvents.load, request).catch(fail);
        }),
      );
      unlisteners.push(
        await listen<ArtworkPlacement>(artworkEditorEvents.apply, (event) => {
          finish(normalizeArtworkPlacement(event.payload));
        }),
      );
      unlisteners.push(
        await listen(artworkEditorEvents.cancel, () => {
          finish(null);
        }),
      );

      const editor = new WebviewWindow(editorLabel, {
        center: true,
        decorations: true,
        focus: true,
        height: 780,
        minHeight: 680,
        minWidth: 980,
        resizable: true,
        title: "M.I.O. 飾り絵の配置",
        url: "/artwork-editor.html",
        width: 1180,
      });

      unlisteners.push(
        await editor.once("tauri://error", (event) => {
          fail(new Error(`画像調整画面を開けませんでした: ${String(event.payload)}`));
        }),
      );
    })().catch(fail);
  });
}
