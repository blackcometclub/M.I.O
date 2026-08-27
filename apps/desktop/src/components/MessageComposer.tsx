import {
  type FormEvent,
  type KeyboardEvent,
  useEffect,
  useRef,
  useState,
} from "react";

import type { Participant } from "../types";
import type { ConductorSendMode } from "../types";
import { useUiPreferences } from "../uiPreferences";
import { Avatar } from "./Avatar";

type MessageComposerProps = {
  dispatchSafetyWarning: string | null;
  conductor: Participant | null;
  hint: string;
  isAvailable: boolean;
  isAwaitingReply: boolean;
  isSending: boolean;
  onDismissDispatchSafetyWarning: () => void;
  onSendModeChange: (mode: ConductorSendMode) => Promise<boolean>;
  onRemoveRecipient: (participantId: string) => void;
  onSend: (body: string) => Promise<boolean>;
  recipients: Participant[];
  sendMode: ConductorSendMode;
  sendError: string | null;
  sendNotice: string | null;
};

export function MessageComposer({
  dispatchSafetyWarning,
  conductor,
  hint,
  isAvailable,
  isAwaitingReply,
  isSending,
  onDismissDispatchSafetyWarning,
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

  async function submit(event?: FormEvent) {
    event?.preventDefault();
    if (!canSend) {
      return;
    }

    shouldRestoreFocusRef.current = true;
    if (await onSend(draft.trim())) {
      setDraft("");
    } else {
      shouldRestoreFocusRef.current = false;
    }
  }

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
          {sendError ? null : (
            <button onClick={onDismissDispatchSafetyWarning} type="button">
              {t("dismissSafetyWarning")}
            </button>
          )}
        </div>
      ) : null}

      <div className="composer-input-row">
        <textarea
          aria-label={t("message")}
          disabled={isSending || isAwaitingReply}
          maxLength={1000}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t("messagePlaceholder")}
          ref={inputRef}
          rows={2}
          value={draft}
        />
        <button disabled={!canSend} type="submit">
          <span>{isSending ? t("saving") : isAwaitingReply ? t("waiting") : t("send")}</span>
          <span aria-hidden="true">↑</span>
        </button>
      </div>
      <span
        className={`composer-hint ${sendNotice ? "is-notice" : ""}`}
        role="status"
      >
        {statusText}
      </span>
    </form>
  );
}
