import { type Ref, useEffect, useMemo, useState } from "react";
import { isBundledRoom } from "../roomPolicies";
import { useUiPreferences } from "../uiPreferences";
import { Avatar } from "./Avatar";

import type {
  ParticipantMap,
  Room,
  RoomBackupPreview,
  RoomBackupStatus,
  RoomConductorStatus,
  RoomWorkspaceStatus,
} from "../types";

type RoomSettingsPanelProps = {
  dataMessage: string | null;
  error: string | null;
  isBusy: boolean;
  backupStatus: RoomBackupStatus;
  onBackup: () => Promise<boolean>;
  onChooseBackupDirectory: () => Promise<boolean>;
  onChooseWorkspace: () => Promise<boolean>;
  onClearWorkspace: () => Promise<boolean>;
  onClose: () => void;
  onConfigureConductor: (participantId: string | null) => Promise<boolean>;
  onDelete: () => Promise<boolean>;
  onEditParticipantProfile: (participantId: string) => void;
  onOpenBackupDirectory: () => Promise<boolean>;
  onPreviewLatestBackup: () => Promise<boolean>;
  onRemoveParticipant: (participantId: string) => Promise<boolean>;
  onResetAiContinuity: (participantId: string) => Promise<boolean>;
  onRename: (name: string) => Promise<boolean>;
  onRestorePreviewedBackup: () => Promise<boolean>;
  onUseDefaultBackupDirectory: () => Promise<boolean>;
  participants: ParticipantMap;
  panelRef: Ref<HTMLElement>;
  room: Room;
  restorePreview: RoomBackupPreview | null;
  roomConductor: RoomConductorStatus;
  workspace: RoomWorkspaceStatus;
};

export function RoomSettingsPanel({
  dataMessage,
  error,
  isBusy,
  backupStatus,
  onBackup,
  onChooseBackupDirectory,
  onChooseWorkspace,
  onClearWorkspace,
  onClose,
  onConfigureConductor,
  onDelete,
  onEditParticipantProfile,
  onOpenBackupDirectory,
  onPreviewLatestBackup,
  onRemoveParticipant,
  onResetAiContinuity,
  onRename,
  onRestorePreviewedBackup,
  onUseDefaultBackupDirectory,
  participants,
  panelRef,
  room,
  restorePreview,
  roomConductor,
  workspace,
}: RoomSettingsPanelProps) {
  const { t } = useUiPreferences();
  const [name, setName] = useState(room.name);
  const [isDeleteArmed, setDeleteArmed] = useState(false);

  useEffect(() => {
    setName(room.name);
    setDeleteArmed(false);
  }, [room.id, room.name]);

  const aiParticipants = useMemo(
    () =>
      room.participantIds
        .map((participantId) => participants[participantId])
        .filter((participant) => participant?.kind === "ai"),
    [participants, room.participantIds],
  );
  const roomParticipants = useMemo(
    () => room.participantIds
      .map((participantId) => participants[participantId])
      .filter((participant) => participant !== undefined),
    [participants, room.participantIds],
  );
  const nativeContinuityParticipants = aiParticipants.filter(
    (participant) => participant.id === "codex" || participant.id === "grok",
  );

  async function saveName() {
    if (await onRename(name)) {
      setName(name.trim());
    }
  }

  async function deleteRoom() {
    if (!isDeleteArmed) {
      setDeleteArmed(true);
      return;
    }
    if (await onDelete()) {
      onClose();
    }
  }

  async function restoreBackup() {
    if (restorePreview) await onRestorePreviewedBackup();
    else await onPreviewLatestBackup();
  }

  return (
    <section aria-label={t("roomSettings")} className="room-settings-panel" ref={panelRef} role="dialog">
      <header className="popover-heading">
        <div>
          <strong>{t("roomSettings")}</strong>
          <span>{t("roomSettingsSubtitle")}</span>
        </div>
        <button onClick={onClose} title={t("close")} type="button">
          <span aria-hidden="true">×</span>
          <span className="sr-only">{t("close")}</span>
        </button>
      </header>

      <div className="room-settings-feedback">
        {error ? (
          <p className="room-settings-error" role="alert">
            {error}
          </p>
        ) : null}
        {dataMessage ? (
          <p className="room-settings-success" role="status">
            {dataMessage}
          </p>
        ) : null}
      </div>

      <div className="room-settings-field">
        <label htmlFor="room-settings-name">{t("roomName")}</label>
        <div className="room-name-control">
          <input
            disabled={isBusy}
            id="room-settings-name"
            maxLength={60}
            onChange={(event) => setName(event.target.value)}
            value={name}
          />
          <button
            disabled={isBusy || name.trim().length === 0 || name.trim() === room.name}
            onClick={saveName}
            type="button"
          >
            {t("save")}
          </button>
        </div>
      </div>

      <div className="room-data-zone">
        <div>
          <strong>{t("backupTitle")}</strong>
          <span>{t("backupHelp")}</span>
        </div>
        <div className="room-backup-location">
          <span>{backupStatus.isCustom ? t("backupCustomLocation") : t("backupDefaultLocation")}</span>
          <code title={backupStatus.directoryPath}>
            {backupStatus.directoryPath || t("backupLocationLoading")}
          </code>
          {!backupStatus.available && backupStatus.directoryPath ? (
            <small>{t("backupLocationMissing")}</small>
          ) : null}
        </div>
        <div className="room-backup-location-actions">
          <button disabled={isBusy} onClick={onChooseBackupDirectory} type="button">
            {t("backupChooseLocation")}
          </button>
          <button
            disabled={isBusy || !backupStatus.available}
            onClick={onOpenBackupDirectory}
            type="button"
          >
            {t("backupOpenLocation")}
          </button>
          {backupStatus.isCustom ? (
            <button disabled={isBusy} onClick={onUseDefaultBackupDirectory} type="button">
              {t("backupUseDefault")}
            </button>
          ) : null}
        </div>
        <div className="room-data-actions">
          <button disabled={isBusy} onClick={onBackup} type="button">
            {t("backup")}
          </button>
          <button
            className={restorePreview ? "is-armed" : ""}
            disabled={isBusy}
            onClick={restoreBackup}
            type="button"
          >
            {restorePreview ? t("reallyRestore") : t("restore")}
          </button>
        </div>
        {restorePreview ? (
          <div className="room-restore-preview" role="status">
            <strong>{t("restorePreviewTitle")}</strong>
            <span>{t("restorePreviewRooms", { count: restorePreview.roomCount })}</span>
            <span>{new Date(restorePreview.createdAtUnixMs).toLocaleString()}</span>
            <code>{restorePreview.fileName}</code>
            <small>{t("restorePreviewWarning")}</small>
          </div>
        ) : null}
      </div>

      <div className="room-workspace-zone">
        <div>
          <strong>{t("codexMode")}</strong>
          <span>
            {workspace.mode === "workspace"
              ? workspace.available
                ? t("workspaceActive", { name: workspace.folderName ?? t("selectedFolder") })
                : t("workspaceMissing", { name: workspace.folderName ?? t("selectedFolder") })
              : t("workspaceHelp")}
          </span>
        </div>
        <div className="room-workspace-actions">
          <button disabled={isBusy} onClick={onChooseWorkspace} type="button">
            {workspace.mode === "workspace" ? t("changeFolder") : t("chooseFolder")}
          </button>
          {workspace.mode === "workspace" ? (
            <button disabled={isBusy} onClick={onClearWorkspace} type="button">
              {t("returnChatOnly")}
            </button>
          ) : null}
        </div>
      </div>

      <div className="room-conductor-zone">
        <div>
          <strong>{t("roomConductor")}</strong>
          <span>{t("roomConductorHelp")}</span>
        </div>
        <label htmlFor="room-conductor-select">{t("roomConductorSelection")}</label>
        <select
          disabled={isBusy}
          id="room-conductor-select"
          onChange={(event) => {
            void onConfigureConductor(event.target.value || null);
          }}
          value={roomConductor.conductorId ?? ""}
        >
          <option value="">{t("noConductor")}</option>
          {aiParticipants.some((participant) => participant.id === "codex") ? (
            <option value="codex">Codex</option>
          ) : null}
        </select>
        {roomConductor.conductorId ? (
          <small>{t("conductorModeChangeHelp")}</small>
        ) : null}
      </div>

      <div className="room-settings-field">
        <div className="room-settings-label-row">
          <strong>{t("participantProfiles")}</strong>
          <span>{t("profileDeviceNote")}</span>
        </div>
        <div className="room-profile-list">
          {roomParticipants.map((participant) => (
            <div className="room-profile-row" key={participant.id}>
              <Avatar participant={participant} size="medium" />
              <span>
                <strong>{participant.displayName}</strong>
                <small>{participant.canonicalName} · {participant.serviceLabel}</small>
              </span>
              <button
                disabled={isBusy}
                onClick={() => onEditParticipantProfile(participant.id)}
                type="button"
              >
                {t("editProfile")}
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="room-settings-field">
        <div className="room-settings-label-row">
          <strong>{t("aiMembership")}</strong>
          <span>{t("keepOneAi")}</span>
        </div>
        <div className="room-member-list">
          {aiParticipants.map((participant) => {
            const isReferenced = room.messages.some(
              (message) =>
                message.authorId === participant.id ||
                message.targetIds.includes(participant.id),
            );
            const cannotRemove = aiParticipants.length <= 1 || isReferenced;
            return (
              <div className="room-member-row" key={participant.id}>
                <span>
                  <strong>{participant.displayName}</strong>
                  <small>
                    {isReferenced ? t("historyKeepsAi") : participant.serviceLabel}
                  </small>
                </span>
                <button
                  disabled={isBusy || cannotRemove}
                  onClick={() => onRemoveParticipant(participant.id)}
                  title={cannotRemove ? t("cannotRemoveAi") : t("removeFromRoom")}
                  type="button"
                >
                  {t("remove")}
                </button>
              </div>
            );
          })}
        </div>
      </div>

      {nativeContinuityParticipants.length > 0 ? (
        <div className="room-settings-field">
          <div className="room-settings-label-row">
            <strong>{t("aiContinuity")}</strong>
            <span>{t("aiContinuityHelp")}</span>
          </div>
          <div className="room-member-list">
            {nativeContinuityParticipants.map((participant) => (
              <div className="room-member-row" key={participant.id}>
                <span>
                  <strong>{participant.displayName}</strong>
                  <small>{t("roomHistoryKept")}</small>
                </span>
                <button
                  disabled={isBusy}
                  onClick={() => onResetAiContinuity(participant.id)}
                  type="button"
                >
                  {t("resetContinuity")}
                </button>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      {!isBundledRoom(room.id) ? (
        <div className="room-danger-zone">
          <div>
            <strong>{t("deleteRoom")}</strong>
            <span>{t("deleteRoomHelp")}</span>
          </div>
          <button
            className={isDeleteArmed ? "is-armed" : ""}
            disabled={isBusy}
            onClick={deleteRoom}
            type="button"
          >
            {isDeleteArmed ? t("reallyDelete") : t("delete")}
          </button>
        </div>
      ) : (
        <p className="protected-room-note">{t("protectedRoom")}</p>
      )}

    </section>
  );
}
