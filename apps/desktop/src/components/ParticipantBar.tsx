import { useEffect, useId, useRef } from "react";
import { Avatar } from "./Avatar";
import type { AiConnectionMap, Participant } from "../types";
import { useUiPreferences } from "../uiPreferences";

type ParticipantBarProps = {
  availableParticipants: Participant[];
  conductorId: string | null;
  connections: AiConnectionMap;
  isMenuOpen: boolean;
  onAddParticipant: (participantId: string) => void;
  onMenuClose: () => void;
  onMenuToggle: () => void;
  onToggleRecipient: (participantId: string) => void;
  participants: Participant[];
  recipientSelectionLocked: boolean;
  selectedRecipientIds: string[];
};

export function ParticipantBar({
  availableParticipants,
  conductorId,
  connections,
  isMenuOpen,
  onAddParticipant,
  onMenuClose,
  onMenuToggle,
  onToggleRecipient,
  participants,
  recipientSelectionLocked,
  selectedRecipientIds,
}: ParticipantBarProps) {
  const {
    locale,
    participantListCollapsed,
    setParticipantListCollapsed,
    t,
  } = useUiPreferences();
  const participantListId = useId();
  const addButtonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isMenuOpen) return;
    const focusFrame = window.requestAnimationFrame(() => {
      const firstAvailableItem = menuRef.current?.querySelector<HTMLButtonElement>(
        'button[role="menuitem"]:not(:disabled)',
      );
      (firstAvailableItem ?? menuRef.current)?.focus();
    });
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      onMenuClose();
      window.requestAnimationFrame(() => addButtonRef.current?.focus());
    };
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (menuRef.current?.contains(target) || addButtonRef.current?.contains(target)) return;
      onMenuClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("pointerdown", handlePointerDown);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [isMenuOpen, onMenuClose]);

  return (
    <section
      className={`participant-bar ${participantListCollapsed ? "is-collapsed" : ""}`}
      aria-label={t("participants")}
    >
      <div className="participant-heading">
        <div className="participant-copy">
          <strong>{t("participants")}</strong>
          <span>{t("chooseRecipients")}</span>
        </div>

        <div className="participant-heading-actions">
          <div className="participant-add-wrap">
            <button
              aria-expanded={isMenuOpen}
              className="participant-add-button"
              onClick={onMenuToggle}
              ref={addButtonRef}
              type="button"
            >
              <span aria-hidden="true">＋</span>
              {t("addAi")}
            </button>

            {isMenuOpen ? (
              <div className="participant-menu" ref={menuRef} role="menu" tabIndex={-1}>
                <div className="popover-heading">
                  <strong>{t("addToRoom")}</strong>
                  <span>{t("addStatusNote")}</span>
                </div>
                {availableParticipants.length > 0 ? (
                  availableParticipants.map((participant) => {
                    const connection = connections[participant.id];
                    const isUnsupported = connection?.state === "unsupported";
                    return (
                      <button
                        disabled={isUnsupported}
                        key={participant.id}
                        onClick={() => onAddParticipant(participant.id)}
                        role="menuitem"
                        title={connection?.detail ?? participant.serviceLabel}
                        type="button"
                      >
                        <Avatar participant={participant} size="small" />
                        <span>
                          <strong>{participant.displayName}</strong>
                          <em>{participant.canonicalName}</em>
                          <small>
                            {isUnsupported
                              ? t("currentlyUnsupported")
                              : connection?.label ?? participant.serviceLabel}
                          </small>
                        </span>
                      </button>
                    );
                  })
                ) : (
                  <p className="menu-empty">{t("allAiAdded")}</p>
                )}
              </div>
            ) : null}
          </div>

          <button
            aria-controls={participantListId}
            aria-expanded={!participantListCollapsed}
            aria-label={t(participantListCollapsed ? "expandParticipants" : "collapseParticipants")}
            className="participant-toggle-button"
            onClick={() => setParticipantListCollapsed(!participantListCollapsed)}
            title={t(participantListCollapsed ? "expandParticipants" : "collapseParticipants")}
            type="button"
          >
            <svg aria-hidden="true" viewBox="0 0 16 16">
              <path d="m4 10 4-4 4 4" />
            </svg>
          </button>
        </div>
      </div>

      <div className="participant-list" hidden={participantListCollapsed} id={participantListId}>
        {participants.map((participant) => {
          const isSelected = selectedRecipientIds.includes(participant.id);
          const isConductor = participant.id === conductorId;
          const connection = connections[participant.id];
          const connectionLabel = locale === "en"
            ? ({ ready: "Ready", installed: "Installed", setupRequired: "Setup required", unsupported: "Unsupported" } as const)[connection?.state ?? "setupRequired"]
            : connection?.label ?? t("checking");
          const connectionState = connection?.state ?? "setupRequired";
          return (
            <button
              aria-pressed={isSelected}
              className={`participant-button ${isSelected ? "is-selected" : ""}`}
              disabled={recipientSelectionLocked}
              key={participant.id}
              onClick={() => onToggleRecipient(participant.id)}
              title={`${participant.displayName} · ${locale === "en" ? connectionLabel : connection?.detail ?? t("checkingDetail")}`}
              type="button"
            >
              <Avatar participant={participant} size="medium" />
              {isConductor ? (
                <span
                  aria-label={t("conductorBadge")}
                  className="participant-conductor-badge"
                  title={t("conductorBadge")}
                >
                  C
                </span>
              ) : null}
              <span className="participant-identity">
                <span className="participant-name-line">
                  <strong>{participant.displayName}</strong>
                  <em>{participant.canonicalName}</em>
                </span>
                <small className={`connection-state is-${connectionState}`}>
                  <i aria-hidden="true" />
                  {connectionLabel}
                </small>
              </span>
              <span className="participant-check" aria-hidden="true">
                ✓
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
}
