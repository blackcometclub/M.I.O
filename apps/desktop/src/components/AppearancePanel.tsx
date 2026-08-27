import type { ChangeEvent, Ref } from "react";
import { useUiPreferences } from "../uiPreferences";
import type { AppearanceSaveStatus } from "../hooks/useAppearance";

type AppearancePanelProps = {
  artworkEditorMessage: string | null;
  appearanceSaveStatus: AppearanceSaveStatus;
  backgroundColor: string;
  hasArtworkImage: boolean;
  hasBackgroundImage: boolean;
  onArtworkImageChange: (file: File) => void;
  onBackgroundColorChange: (color: string) => void;
  onBackgroundImageChange: (file: File) => void;
  onClearArtworkImage: () => void;
  onClearBackgroundImage: () => void;
  onClose: () => void;
  onEditArtwork: () => void;
  panelRef: Ref<HTMLElement>;
};

const presets = ["#ffc126", "#f08da8", "#80b9cf", "#7d72c6"];

export function AppearancePanel({
  artworkEditorMessage,
  appearanceSaveStatus,
  backgroundColor,
  hasArtworkImage,
  hasBackgroundImage,
  onArtworkImageChange,
  onBackgroundColorChange,
  onBackgroundImageChange,
  onClearArtworkImage,
  onClearBackgroundImage,
  onClose,
  onEditArtwork,
  panelRef,
}: AppearancePanelProps) {
  const { t } = useUiPreferences();
  function chooseBackgroundImage(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) {
      onBackgroundImageChange(file);
    }
    event.target.value = "";
  }

  function chooseArtworkImage(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) {
      onArtworkImageChange(file);
    }
    event.target.value = "";
  }

  return (
    <section aria-label={t("appearance")} className="appearance-panel" ref={panelRef} role="dialog">
      <header className="popover-heading">
        <div>
          <strong>{t("appearanceTitle")}</strong>
          <span>{t("appearanceSubtitle")}</span>
        </div>
        <button onClick={onClose} title={t("close")} type="button">
          <span aria-hidden="true">×</span>
          <span className="sr-only">{t("close")}</span>
        </button>
      </header>

      <div className="appearance-field">
        <label htmlFor="background-color">{t("backgroundColor")}</label>
        <div className="color-control">
          <input
            id="background-color"
            onChange={(event) => onBackgroundColorChange(event.target.value)}
            type="color"
            value={backgroundColor}
          />
          <code>{backgroundColor.toUpperCase()}</code>
        </div>
        <div className="color-presets" aria-label={t("colorPresets")}>
          {presets.map((preset) => (
            <button
              aria-label={`${t("backgroundColor")} ${preset}`}
              className={backgroundColor === preset ? "is-current" : ""}
              key={preset}
              onClick={() => onBackgroundColorChange(preset)}
              style={{ backgroundColor: preset }}
              type="button"
            />
          ))}
        </div>
      </div>

      <div className="appearance-field">
        <span>{t("bodyBackground")}</span>
        <label className="image-picker">
          <input accept="image/png,image/jpeg,image/webp" onChange={chooseBackgroundImage} type="file" />
          <span aria-hidden="true">▧</span>
          {t("chooseBackground")}
        </label>
        {hasBackgroundImage ? (
          <button className="clear-image-button" onClick={onClearBackgroundImage} type="button">
            {t("clearBackground")}
          </button>
        ) : null}
      </div>

      <div className="appearance-field artwork-field">
        <span>{t("artwork")}</span>
        <small>{t("artworkHelp")}</small>
        <label className="image-picker artwork-image-picker">
          <input accept="image/png,image/jpeg,image/webp" onChange={chooseArtworkImage} type="file" />
          <span aria-hidden="true">♡</span>
          {t("chooseArtwork")}
        </label>
        {hasArtworkImage ? (
          <div className="artwork-action-row">
            <button className="edit-artwork-button" onClick={onEditArtwork} type="button">
              {t("editPlacement")}
            </button>
            <button className="clear-image-button" onClick={onClearArtworkImage} type="button">
              {t("clearArtwork")}
            </button>
          </div>
        ) : null}
        {artworkEditorMessage ? (
          <p className="artwork-editor-message" role="status">
            {artworkEditorMessage}
          </p>
        ) : null}
        <p className="transparent-png-note">
          {t("artworkNote")}
        </p>
      </div>
      <p className={`appearance-save-status is-${appearanceSaveStatus}`} role="status">
        {t(
          appearanceSaveStatus === "saving"
            ? "appearanceSaving"
            : appearanceSaveStatus === "error"
              ? "appearanceSaveFailed"
              : appearanceSaveStatus === "loading"
                ? "appearanceLoading"
                : "appearanceSaved",
        )}
      </p>
    </section>
  );
}
