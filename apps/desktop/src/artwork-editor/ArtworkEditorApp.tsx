import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import {
  artworkEditorEvents,
  defaultArtworkPlacement,
  normalizeArtworkPlacement,
  type ArtworkEditorRequest,
  type ArtworkPlacement,
} from "../artwork";
import { ArtworkCanvas } from "../components/ArtworkCanvas";

type DragState = {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  startPlacement: ArtworkPlacement;
};

type PreviewStyle = CSSProperties & {
  "--editor-sidebar-width"?: string;
  "--editor-workbench-gap"?: string;
};

const minimumScale = 0.05;
const maximumScale = 2;

function scaleToSliderValue(scale: number) {
  return scale < 1
    ? -(Math.log(scale) / Math.log(minimumScale)) * 100
    : (Math.log(scale) / Math.log(maximumScale)) * 100;
}

function sliderValueToScale(value: number) {
  return value < 0
    ? minimumScale ** (-value / 100)
    : maximumScale ** (value / 100);
}

export function ArtworkEditorApp() {
  const [request, setRequest] = useState<ArtworkEditorRequest | null>(null);
  const [placement, setPlacement] = useState(defaultArtworkPlacement);
  const [naturalSize, setNaturalSize] = useState({ width: 0, height: 0 });
  const [previewScale, setPreviewScale] = useState(1);
  const stageRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<DragState | null>(null);
  const isFinishingRef = useRef(false);

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];
    let isDisposed = false;
    const editorWindow = getCurrentWindow();

    void (async () => {
      unlisteners.push(
        await listen<ArtworkEditorRequest>(artworkEditorEvents.load, (event) => {
          setRequest(event.payload);
          setPlacement(normalizeArtworkPlacement(event.payload.placement));
        }),
      );
      unlisteners.push(
        await editorWindow.onCloseRequested(async (event) => {
          if (isFinishingRef.current) {
            return;
          }
          event.preventDefault();
          await finish(artworkEditorEvents.cancel);
        }),
      );

      if (!isDisposed) {
        await emitTo("main", artworkEditorEvents.ready);
      }
    })();

    return () => {
      isDisposed = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  }, []);

  useLayoutEffect(() => {
    const stage = stageRef.current;
    if (!stage || !request) {
      return;
    }

    const updatePreviewScale = () => {
      const bounds = stage.getBoundingClientRect();
      setPreviewScale(
        Math.min(
          bounds.width / request.canvasLayout.width,
          bounds.height / request.canvasLayout.height,
        ),
      );
    };

    updatePreviewScale();
    const observer = new ResizeObserver(updatePreviewScale);
    observer.observe(stage);
    return () => observer.disconnect();
  }, [request]);

  async function finish(
    eventName: typeof artworkEditorEvents.apply | typeof artworkEditorEvents.cancel,
    payload?: ArtworkPlacement,
  ) {
    if (isFinishingRef.current) {
      return;
    }

    isFinishingRef.current = true;
    try {
      await emitTo("main", eventName, payload);
    } finally {
      await getCurrentWindow().destroy();
    }
  }

  function setZoomFromSlider(value: number) {
    setPlacement((current) =>
      normalizeArtworkPlacement({ ...current, scale: sliderValueToScale(value) }),
    );
  }

  function handlePointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (!request || !stageRef.current) {
      return;
    }

    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startPlacement: placement,
    };
  }

  function handlePointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    const stage = stageRef.current;
    if (!drag || drag.pointerId !== event.pointerId || !stage) {
      return;
    }

    const bounds = stage.getBoundingClientRect();
    setPlacement(
      normalizeArtworkPlacement({
        ...drag.startPlacement,
        x: drag.startPlacement.x + (event.clientX - drag.startClientX) / bounds.width,
        y: drag.startPlacement.y + (event.clientY - drag.startClientY) / bounds.height,
      }),
    );
  }

  function handlePointerEnd(event: ReactPointerEvent<HTMLDivElement>) {
    if (dragRef.current?.pointerId === event.pointerId) {
      dragRef.current = null;
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  function handleWheel(event: ReactWheelEvent<HTMLDivElement>) {
    if (!request) {
      return;
    }

    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.08 : 0.92;
    setPlacement((current) =>
      normalizeArtworkPlacement({ ...current, scale: current.scale * factor }),
    );
  }

  const zoomPercent = Math.round(placement.scale * 100);
  const zoomSliderValue = scaleToSliderValue(placement.scale);
  const previewStyle: PreviewStyle | undefined = request
    ? {
        aspectRatio: `${request.canvasLayout.width} / ${request.canvasLayout.height}`,
        "--editor-sidebar-width": `${(request.canvasLayout.sidebarWidth / request.canvasLayout.width) * 100}%`,
        "--editor-workbench-gap": `${(request.canvasLayout.gap / request.canvasLayout.width) * 100}%`,
      }
    : undefined;
  const isEnglish = request?.locale === "en";

  return (
    <main className="artwork-editor-shell">
      <header className="editor-topbar">
        <div>
          <span>ARTWORK POSITIONING LAB</span>
          <h1>{isEnglish ? "Adjust full-shell artwork" : "M.I.O.全体の画像を調整"}</h1>
        </div>
        <p>{isEnglish ? "Drag to move · Use the wheel to zoom" : "画像をつかんで移動 · ホイールでも拡大縮小"}</p>
      </header>

      <section className="editor-content">
        <div className="preview-card">
          <div className="preview-heading">
            <div>
              <strong>{isEnglish ? "M.I.O. preview" : "仮M.I.O.プレビュー"}</strong>
              <span>{isEnglish ? "Preview of the colored shell including the Chat tree" : "左のChatツリーを含む、黄色い部分全体の確認画面です"}</span>
            </div>
            <span className="preview-live-badge">LIVE PREVIEW</span>
          </div>

          <div
            aria-label="M.I.O.全体の画像編集領域"
            className="mock-moe-window"
            onPointerCancel={handlePointerEnd}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerEnd}
            onWheel={handleWheel}
            ref={stageRef}
            style={previewStyle}
          >
            {request ? (
              <ArtworkCanvas
                ariaLabel="調整中のM.I.O.全体画像"
                className="editor-artwork-canvas"
                dataUrl={request.dataUrl}
                imageAlt="調整中のM.I.O.全体画像"
                imageScaleMultiplier={previewScale}
                onImageLoad={(width, height) => setNaturalSize({ width, height })}
                placement={placement}
              />
            ) : (
              <div className="editor-loading">{isEnglish ? "Loading image…" : "画像を読み込んでいます…"}</div>
            )}

            <aside className="mock-room-tree">
              <small>ROOMS</small>
              <strong>{isEnglish ? "Talk rooms" : "トークルーム"}</strong>
              <i />
              <i />
              <i />
            </aside>

            <section className="mock-workspace">
              <header>
                <div>
                  <small>TALK ROOM</small>
                  <strong>{isEnglish ? "M.I.O. Dev Room" : "M.I.O.開発室"}</strong>
                </div>
                <b>M.I.O.</b>
              </header>

              <div className="mock-main-grid">
                <div className="mock-participants">
                  <i />
                  <i />
                  <i />
                </div>
                <div className="mock-conversation">
                  <span />
                  <span />
                  <span />
                </div>
                <div className="mock-composer" />
              </div>
            </section>
          </div>
        </div>

        <aside className="editor-controls">
          <div className="control-heading">
            <span>SELECTED IMAGE</span>
            <strong>{request?.fileName ?? (isEnglish ? "Loading…" : "読み込み中…")}</strong>
            {naturalSize.width > 0 ? (
              <small>
                {naturalSize.width} × {naturalSize.height}px
              </small>
            ) : null}
          </div>

          <div className="zoom-control">
            <label htmlFor="artwork-zoom">
              <span>{isEnglish ? "Scale" : "倍率"}</span>
              <output>{zoomPercent}%</output>
            </label>
            <input
              disabled={!request}
              id="artwork-zoom"
              max="100"
              min="-100"
              onChange={(event) => setZoomFromSlider(Number(event.target.value))}
              step="1"
              type="range"
              value={zoomSliderValue}
            />
            <div className="zoom-scale-labels">
              <span>5%</span>
              <span>{isEnglish ? "Original 100%" : "原寸 100%"}</span>
              <span>200%</span>
            </div>
          </div>

        </aside>
      </section>

      <footer className="editor-footer">
        <span>{isEnglish ? "Drag to position · Wheel or slider to scale" : "ドラッグで位置調整 · ホイールまたはスライダーで倍率調整"}</span>
        <div>
          <button
            className="cancel-button"
            onClick={() => void finish(artworkEditorEvents.cancel)}
            type="button"
          >
            {isEnglish ? "Cancel" : "キャンセル"}
          </button>
          <button
            className="apply-button"
            disabled={!request}
            onClick={() => void finish(artworkEditorEvents.apply, placement)}
            type="button"
          >
            {isEnglish ? "Apply placement" : "この配置で決定"}
          </button>
        </div>
      </footer>
    </main>
  );
}
