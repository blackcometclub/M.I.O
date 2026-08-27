import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUiPreferences } from "../uiPreferences";

type WindowAction = "close" | "minimize" | "toggleMaximize";

const isTauri = "__TAURI_INTERNALS__" in window;

function runWindowAction(action: WindowAction) {
  if (!isTauri) {
    return;
  }

  const appWindow = getCurrentWindow();
  void appWindow[action]().catch((reason: unknown) => {
    console.error(`Window action ${action} failed`, reason);
  });
}

export function WindowControls() {
  const { t } = useUiPreferences();
  if (!isTauri) {
    return null;
  }

  return (
    <div aria-label="Window controls" className="window-controls" role="group">
      <button
        aria-label={t("minimize")}
        onClick={() => runWindowAction("minimize")}
        title={t("minimize")}
        type="button"
      >
        <span aria-hidden="true">−</span>
      </button>
      <button
        aria-label={t("maximize")}
        onClick={() => runWindowAction("toggleMaximize")}
        title={t("maximize")}
        type="button"
      >
        <span aria-hidden="true">□</span>
      </button>
      <button
        aria-label={t("close")}
        className="window-close-button"
        onClick={() => runWindowAction("close")}
        title={t("close")}
        type="button"
      >
        <span aria-hidden="true">×</span>
      </button>
    </div>
  );
}
