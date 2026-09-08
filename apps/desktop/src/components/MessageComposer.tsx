import {
  type FormEvent,
  type KeyboardEvent,
  useEffect,
  useRef,
  useState,
} from "react";

import type {
  ConductorSendMode,
  ImageGenerationComposition,
  ImageGenerationPreferences,
  ImageGenerationQuality,
  Participant,
} from "../types";
import { useUiPreferences } from "../uiPreferences";
import { Avatar } from "./Avatar";

type MessageComposerProps = {
  dispatchSafetyWarning: string | null;
  conductor: Participant | null;
  hint: string;
  isBackgroundTurn: boolean;
  isAvailable: boolean;
  isAwaitingReply: boolean;
  isCancelling: boolean;
  isSending: boolean;
  onCancel: () => Promise<boolean>;
  onDismissDispatchSafetyWarning: () => void;
  onDismissSendError: () => void;
  onSendModeChange: (mode: ConductorSendMode) => Promise<boolean>;
  onRemoveRecipient: (participantId: string) => void;
  onSend: (body: string, imageGeneration: ImageGenerationPreferences | null) => Promise<boolean>;
  recipients: Participant[];
  sendMode: ConductorSendMode;
  sendError: string | null;
  sendNotice: string | null;
};

const imageGenerationPreferencesStorageKey = "moe-image-generation-preferences-v2";
const imageCompositions: ImageGenerationComposition[] = ["1:1", "4:3", "3:4", "16:9", "9:16"];
const imageQualities: ImageGenerationQuality[] = ["auto", "low", "medium", "high"];

const legacyImageCompositions: Record<string, ImageGenerationComposition> = {
  auto: "1:1",
  square: "1:1",
  landscape: "4:3",
  portrait: "3:4",
};

function normalizeImageComposition(value: unknown): ImageGenerationComposition {
  if (imageCompositions.includes(value as ImageGenerationComposition)) {
    return value as ImageGenerationComposition;
  }
  return typeof value === "string" ? legacyImageCompositions[value] ?? "1:1" : "1:1";
}

function loadImageGenerationPreferences(): ImageGenerationPreferences {
  try {
    const value = JSON.parse(localStorage.getItem(imageGenerationPreferencesStorageKey) ?? "null") as Partial<ImageGenerationPreferences> | null;
    return {
      composition: normalizeImageComposition(value?.composition),
      quality: imageQualities.includes(value?.quality as ImageGenerationQuality) ? value!.quality! : "auto",
    };
  } catch {
    return { composition: "1:1", quality: "auto" };
  }
}

export function MessageComposer({
  dispatchSafetyWarning,
  conductor,
  hint,
  isBackgroundTurn,
  isAvailable,
  isAwaitingReply,
  isCancelling,
  isSending,
  onCancel,
  onDismissDispatchSafetyWarning,
  onDismissSendError,
  onSendModeChange,
  onRemoveRecipient,
  onSend,
  recipients,
  sendMode,
  sendError,
  sendNotice,
}: MessageComposerProps) {
  const { t } = useUiPreferences();
  const [draft, setDraft] = useState("");
  const [imageGeneration, setImageGeneration] = useState(loadImageGenerationPreferences);
  const composerRef = useRef<HTMLFormElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const shouldRestoreFocusRef = useRef(false);
  const canSend =
    draft.trim().length > 0 &&
    recipients.length > 0 &&
    isAvailable &&
    !isSending &&
    !isAwaitingReply;
  const visibleSafetyWarning = sendError ?? dispatchSafetyWarning;
  const statusText = sendNotice ?? hint;
  const showBackgroundTurnHint = isBackgroundTurn && !sendNotice;
  const canConfigureImageGeneration =
    sendMode === "direct" && recipients.some((participant) => participant.id === "codex");

  async function submit(event?: FormEvent) {
    event?.preventDefault();
    if (!canSend) {
      return;
    }

    shouldRestoreFocusRef.current = true;
    if (await onSend(draft.trim(), canConfigureImageGeneration ? imageGeneration : null)) {
      setDraft("");
    } else {
      shouldRestoreFocusRef.current = false;
    }
  }

  useEffect(() => {
    try {
      localStorage.setItem(imageGenerationPreferencesStorageKey, JSON.stringify(imageGeneration));
    } catch {
      // The current selection still works for this session when storage is unavailable.
    }
  }, [imageGeneration]);

  useEffect(() => {
    if (isSending || isAwaitingReply || !shouldRestoreFocusRef.current) {
      return;
    }
    const frame = window.requestAnimationFrame(() => {
      const activeElement = document.activeElement;
      const userMovedElsewhere =
        activeElement !== null &&
        activeElement !== document.body &&
        !composerRef.current?.contains(activeElement);
      shouldRestoreFocusRef.current = false;
      if (!userMovedElsewhere && !inputRef.current?.disabled) {
        inputRef.current?.focus();
      }
    });
    return () => window.cancelAnimationFrame(frame);
  }, [isAwaitingReply, isSending]);

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (
      event.key !== "Enter" ||
      event.shiftKey ||
      event.nativeEvent.isComposing
    ) {
      return;
    }

    event.preventDefault();
    void submit();
  }

  return (
    <form className="message-composer" onSubmit={submit} ref={composerRef}>
      <div className="recipient-row">
        <span className="recipient-label">To</span>
        {recipients.length > 0 ? (
          recipients.map((participant) => (
            <button
              className="recipient-chip"
              key={participant.id}
              onClick={() => onRemoveRecipient(participant.id)}
              title={t("removeRecipient", { name: participant.displayName })}
              type="button"
            >
              <Avatar participant={participant} size="small" />
              <span>{participant.displayName}</span>
              <span aria-hidden="true">×</span>
            </button>
          ))
        ) : (
          <span className="recipient-empty">{t("chooseRecipientFirst")}</span>
        )}
        {conductor ? (
          <div aria-label={t("sendMode")} className="composer-mode-switch" role="group">
            <button
              aria-pressed={sendMode === "direct"}
              className={sendMode === "direct" ? "is-active" : ""}
              disabled={isSending || isAwaitingReply}
              onClick={() => void onSendModeChange("direct")}
              type="button"
            >
              Direct
            </button>
            <button
              aria-pressed={sendMode === "conductor"}
              className={sendMode === "conductor" ? "is-active" : ""}
              disabled={isSending || isAwaitingReply}
              onClick={() => void onSendModeChange("conductor")}
              type="button"
            >
              Conductor
            </button>
          </div>
        ) : null}
      </div>

      {visibleSafetyWarning ? (
        <div
          className="composer-safety-warning"
          role={sendError ? "alert" : "status"}
        >
          <span>{visibleSafetyWarning}</span>
          <button
            onClick={sendError ? onDismissSendError : onDismissDispatchSafetyWarning}
            type="button"
          >
            {sendError ? t("close") : t("dismissSafetyWarning")}
          </button>
        </div>
      ) : null}

      {canConfigureImageGeneration ? (
        <details className="composer-image-settings">
          <summary>{t("imageGenerationSettings")}</summary>
          <div>
            <fieldset className="composer-aspect-ratio-fieldset">
              <legend>{t("imageGenerationComposition")}</legend>
              <div
                aria-label={t("imageGenerationComposition")}
                className="composer-aspect-ratio-options"
                role="radiogroup"
              >
                {imageCompositions.map((composition) => {
                  const label = composition === "1:1"
                    ? t("imageAspectSquare")
                    : t("imageAspectRatio", { ratio: composition });
                  return (
                    <button
                      aria-checked={imageGeneration.composition === composition}
                      aria-label={label}
                      className={imageGeneration.composition === composition ? "is-active" : ""}
                      data-ratio={composition}
                      disabled={isSending || isAwaitingReply}
                      key={composition}
                      onClick={() => setImageGeneration((current) => ({ ...current, composition }))}
                      role="radio"
                      title={label}
                      type="button"
                    >
                      <span aria-hidden="true" className="composer-aspect-ratio-glyph" />
                      <span>{composition}</span>
                    </button>
                  );
                })}
              </div>
            </fieldset>
            <label>
              <span>{t("imageGenerationQuality")}</span>
              <select
                disabled={isSending || isAwaitingReply}
                onChange={(event) => setImageGeneration((current) => ({
                  ...current,
                  quality: event.target.value as ImageGenerationQuality,
                }))}
                value={imageGeneration.quality}
              >
                <option value="auto">{t("imageQualityAuto")}</option>
                <option value="low">{t("imageQualityLow")}</option>
                <option value="medium">{t("imageQualityMedium")}</option>
                <option value="high">{t("imageQualityHigh")}</option>
              </select>
            </label>
            <p>{t("imageGenerationSettingsHelp")}</p>
          </div>
        </details>
      ) : null}

      <div className="composer-input-row">
        <textarea
          aria-label={t("message")}
          disabled={isSending}
          maxLength={1000}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t("messagePlaceholder")}
          ref={inputRef}
          rows={2}
          value={draft}
        />
        <button
          aria-label={isAwaitingReply ? t("stop") : t("send")}
          className={isAwaitingReply ? "is-stop" : undefined}
          disabled={isAwaitingReply ? isCancelling : !canSend}
          onClick={isAwaitingReply ? () => void onCancel() : undefined}
          type={isAwaitingReply ? "button" : "submit"}
        >
          <span>{isSending ? t("saving") : isCancelling ? t("stopping") : isAwaitingReply ? t("stop") : t("send")}</span>
          <span aria-hidden="true">{isAwaitingReply ? "■" : "↑"}</span>
        </button>
      </div>
      <span
        className={`composer-hint ${sendNotice ? "is-notice" : ""} ${showBackgroundTurnHint ? "is-background-turn" : ""}`}
        role="status"
      >
        {statusText}
      </span>
    </form>
  );
}
