import { useEffect, useRef, useState } from "react";

import { Avatar } from "./Avatar";
import type { AiAccessMode, Participant, ParticipantProfile, RoomWorkspaceStatus } from "../types";
import { useUiPreferences } from "../uiPreferences";

type ParticipantProfileEditorProps = {
  onClose: () => void;
  onSave: (profile: ParticipantProfile) => Promise<boolean>;
  participant: Participant;
  profile?: ParticipantProfile;
  roomWorkspace: RoomWorkspaceStatus;
};

const maximumAvatarBytes = 5 * 1024 * 1024;

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function imageStyle(scale: number, x: number, y: number) {
  return {
    objectPosition: `${50 + x * 50}% ${50 + y * 50}%`,
    transform: `translate(${-x * (scale - 1) * 50 / scale}%, ${-y * (scale - 1) * 50 / scale}%) scale(${scale})`,
  };
}

function readFile(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => typeof reader.result === "string" ? resolve(reader.result) : reject());
    reader.addEventListener("error", reject);
    reader.readAsDataURL(file);
  });
}

export function ParticipantProfileEditor({
  onClose,
  onSave,
  participant,
  profile,
  roomWorkspace,
}: ParticipantProfileEditorProps) {
  const { t } = useUiPreferences();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const cropRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ pointerId: number; x: number; y: number; clientX: number; clientY: number } | null>(null);
  const [displayName, setDisplayName] = useState(participant.displayName);
  const [dataUrl, setDataUrl] = useState(
    participant.avatarPlacement ? participant.avatarUrl ?? null : null,
  );
  const [scale, setScale] = useState(participant.avatarPlacement?.scale ?? 1);
  const [x, setX] = useState(participant.avatarPlacement?.x ?? 0);
  const [y, setY] = useState(participant.avatarPlacement?.y ?? 0);
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setSaving] = useState(false);
  const [aiInstructions, setAiInstructions] = useState(profile?.aiInstructions ?? "");
  const supportsWorkspaceAccess = false;
  const [aiAccessMode, setAiAccessMode] = useState<AiAccessMode>(() => {
    if (!supportsWorkspaceAccess) return "chatOnly";
    if (profile?.aiAccessMode && profile.aiAccessMode !== "providerDefault") return profile.aiAccessMode;
    return "chatOnly";
  });
  const workspaceSelected = roomWorkspace.mode === "workspace" && roomWorkspace.available;

  const previewParticipant: Participant = {
    ...participant,
    displayName: displayName.trim() || participant.displayName,
    initials: Array.from(displayName.trim() || participant.displayName).slice(0, 2).join(""),
    avatarUrl: dataUrl ?? undefined,
    avatarPlacement: dataUrl ? { scale, x, y } : undefined,
  };

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !isSaving) onClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isSaving, onClose]);

  function applyScale(nextScale: number) {
    const bounded = clamp(nextScale, 1, 6);
    setScale(bounded);
  }

  async function chooseAvatar(file: File | undefined) {
    if (!file) return;
    setError(null);
    if (!["image/png", "image/jpeg", "image/webp"].includes(file.type)) {
      setError(t("invalidAvatarImage"));
      return;
    }
    if (file.size > maximumAvatarBytes) {
      setError(t("avatarImageTooLarge"));
      return;
    }
    try {
      const bitmap = await createImageBitmap(file);
      bitmap.close();
      setDataUrl(await readFile(file));
      setScale(1);
      setX(0);
      setY(0);
    } catch {
      setError(t("invalidAvatarImage"));
    }
  }

  function handlePointerDown(event: React.PointerEvent<HTMLDivElement>) {
    if (!dataUrl || event.button !== 0) return;
    dragRef.current = { pointerId: event.pointerId, x, y, clientX: event.clientX, clientY: event.clientY };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: React.PointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    const bounds = cropRef.current?.getBoundingClientRect();
    if (!drag || drag.pointerId !== event.pointerId || !bounds) return;
    setX(clamp(drag.x - ((event.clientX - drag.clientX) / bounds.width) * 2 / scale, -1, 1));
    setY(clamp(drag.y - ((event.clientY - drag.clientY) / bounds.height) * 2 / scale, -1, 1));
  }

  function stopDrag(event: React.PointerEvent<HTMLDivElement>) {
    if (dragRef.current?.pointerId === event.pointerId) dragRef.current = null;
  }

  async function save() {
    const trimmedName = displayName.trim();
    if (!trimmedName) return;
    setSaving(true);
    setError(null);
    const saved = await onSave({
      participantId: participant.id,
      displayName: trimmedName,
      avatar: dataUrl ? { dataUrl, scale, x, y } : null,
      aiInstructions: participant.kind === "ai" ? aiInstructions.trim() : "",
      aiAccessMode: participant.kind === "ai" ? aiAccessMode : "providerDefault",
    });
    setSaving(false);
    if (saved) onClose();
    else setError(t("profileSaveFailed"));
  }

  return (
    <div
      aria-label={t("profileTitle")}
      aria-modal="true"
      className="profile-editor-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !isSaving) onClose();
      }}
      role="dialog"
    >
      <section className="profile-editor-card">
        <header className="profile-editor-heading">
          <div>
            <strong>{t("profileTitle")}</strong>
            <span>{t("profileSubtitle")}</span>
          </div>
          <button disabled={isSaving} onClick={onClose} title={t("close")} type="button">×</button>
        </header>

        <div className="profile-editor-identity">
          <Avatar participant={previewParticipant} size="large" />
          <span className="profile-editor-canonical">
            <strong>{participant.canonicalName}</strong>
            <small>{participant.identityBadge} · {participant.serviceLabel}</small>
          </span>
          <em>{t("identityLocked")}</em>
        </div>

        <label className="profile-name-field">
          <span>{t("displayName")}</span>
          <input
            autoFocus
            disabled={isSaving}
            maxLength={60}
            onChange={(event) => setDisplayName(event.target.value)}
            value={displayName}
          />
        </label>

        {participant.kind === "ai" ? (
          <>
            <label className="profile-instructions-field">
              <span>{t("aiInstructions")}</span>
              <small>{t("aiInstructionsHelp")}</small>
              <textarea
                disabled={isSaving}
                maxLength={2000}
                onChange={(event) => setAiInstructions(event.target.value)}
                placeholder={t("aiInstructionsPlaceholder")}
                rows={4}
                value={aiInstructions}
              />
              <output>{aiInstructions.length} / 2000</output>
            </label>

            <fieldset className="profile-permissions-field">
              <legend>{t("aiPermissions")}</legend>
              <p>{t("aiPermissionsHelp")}</p>
              {([
                ["chatOnly", "permissionChatOnly", "permissionChatOnlyDetail", true],
                ["workspaceRead", "permissionWorkspaceRead", "permissionWorkspaceReadDetail", supportsWorkspaceAccess],
                ["workspaceWrite", "permissionWorkspaceWrite", "permissionWorkspaceWriteDetail", supportsWorkspaceAccess],
              ] as const).map(([mode, title, detail, supported]) => (
                <label className={!supported ? "is-disabled" : undefined} key={mode}>
                  <input
                    checked={aiAccessMode === mode}
                    disabled={isSaving || !supported}
                    name={`ai-access-${participant.id}`}
                    onChange={() => setAiAccessMode(mode)}
                    type="radio"
                  />
                  <span>
                    <strong>{t(title)}</strong>
                    <small>{t(detail)}</small>
                  </span>
                  {!supported ? <em>{t("permissionNotSupported")}</em> : null}
                </label>
              ))}
              <div className="profile-permission-summary">
                <span>{t("permissionCommands")}: <strong>{aiAccessMode === "chatOnly" ? t("permissionOff") : t("permissionLocalOnly")}</strong></span>
                <span>{t("permissionWeb")}: <strong>{t("permissionOff")}</strong></span>
              </div>
              {aiAccessMode !== "chatOnly" && !workspaceSelected ? (
                <p className="profile-permission-note">{t("permissionNeedsWorkspace")}</p>
              ) : null}
            </fieldset>
          </>
        ) : null}

        <div className="profile-avatar-editor">
          <div>
            <strong>{t("avatarImage")}</strong>
            <span>{t("avatarPositionHelp")}</span>
          </div>
          <div
            className={`profile-avatar-crop ${dataUrl ? "has-image" : ""}`}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={stopDrag}
            onPointerCancel={stopDrag}
            ref={cropRef}
          >
            {dataUrl ? (
              <img
                alt=""
                draggable={false}
                src={dataUrl}
                style={imageStyle(scale, x, y)}
              />
            ) : (
              <span>{previewParticipant.initials}</span>
            )}
            <i aria-hidden="true" />
          </div>

          <input
            accept="image/png,image/jpeg,image/webp"
            className="sr-only"
            onChange={(event) => {
              void chooseAvatar(event.target.files?.[0]);
              event.target.value = "";
            }}
            ref={fileInputRef}
            type="file"
          />
          <div className="profile-avatar-actions">
            <button disabled={isSaving} onClick={() => fileInputRef.current?.click()} type="button">
              {dataUrl ? t("changeAvatar") : t("chooseAvatar")}
            </button>
            {dataUrl ? (
              <button className="is-secondary" disabled={isSaving} onClick={() => {
                setDataUrl(null);
                setScale(1);
                setX(0);
                setY(0);
              }} type="button">{t("removeAvatar")}</button>
            ) : null}
          </div>

          <label className="profile-zoom-field">
            <span>{t("avatarZoom")}</span>
            <input
              disabled={!dataUrl || isSaving}
              max="6"
              min="1"
              onChange={(event) => applyScale(Number(event.target.value))}
              step="0.05"
              type="range"
              value={scale}
            />
            <output>{Math.round(scale * 100)}%</output>
          </label>
        </div>

        <div className="profile-preview-row">
          <span>{t("profilePreview")}</span>
          <Avatar participant={previewParticipant} size="small" />
          <Avatar participant={previewParticipant} size="medium" />
          <strong>{previewParticipant.displayName}</strong>
          <small>{previewParticipant.canonicalName}</small>
        </div>

        {error ? <p className="profile-editor-error" role="alert">{error}</p> : null}
        <footer className="profile-editor-footer">
          <button className="is-secondary" disabled={isSaving} onClick={onClose} type="button">{t("cancel")}</button>
          <button disabled={isSaving || displayName.trim().length === 0} onClick={save} type="button">
            {isSaving ? t("profileSaving") : t("saveProfile")}
          </button>
        </footer>
      </section>
    </div>
  );
}
