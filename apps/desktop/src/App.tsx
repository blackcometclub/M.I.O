import { useEffect, useRef, useState } from "react";

import mioLogoUrl from "./assets/mio-logo.svg";
import { AppearancePanel } from "./components/AppearancePanel";
import { ArtworkStage } from "./components/ArtworkStage";
import { ConversationPanel } from "./components/ConversationPanel";
import { MessageComposer } from "./components/MessageComposer";
import { ParticipantBar } from "./components/ParticipantBar";
import { ParticipantProfileEditor } from "./components/ParticipantProfileEditor";
import { PreferencesPanel } from "./components/PreferencesPanel";
import { RoomSidebar } from "./components/RoomSidebar";
import { SidebarResizeHandle } from "./components/SidebarResizeHandle";
import { RoomSettingsPanel } from "./components/RoomSettingsPanel";
import { WindowControls } from "./components/WindowControls";
import { WindowResizeHandles } from "./components/WindowResizeHandles";
import { useAppearance } from "./hooks/useAppearance";
import { useBootstrapStatus } from "./hooks/useBootstrapStatus";
import { useRooms } from "./hooks/useRooms";
import { useUiPreferences } from "./uiPreferences";

export function App() {
  const { t } = useUiPreferences();
  const { coreReady } = useBootstrapStatus();
  const {
    activeRoom,
    aiConnections,
    addParticipant,
    availableParticipants,
    backupRooms,
    changeConductorSendMode,
    chooseWorkspace,
    createRoom,
    closeParticipantMenu,
    clearWorkspace,
    clearRoomMutationError,
    configureRoomConductor,
    deleteRoom,
    dismissDispatchSafetyWarning,
    dispatchSafetyWarning,
    isParticipantMenuOpen,
    isAwaitingReply,
    isSending,
    participants,
    participantProfiles,
    recipientIds,
    roomParticipants,
    rooms,
    roomSourceMode,
    roomMutationError,
    roomConductor,
    roomWorkspace,
    roomDataMessage,
    removeParticipant,
    resetAiContinuity,
    renameRoom,
    restoreLatestBackup,
    saveParticipantProfile,
    sendError,
    sendNotice,
    selectedRecipients,
    selectRoom,
    sendMessage,
    toggleParticipantMenu,
    toggleRecipient,
    typingParticipantId,
  } = useRooms();
  const [isRoomSettingsOpen, setRoomSettingsOpen] = useState(false);
  const [isPreferencesOpen, setPreferencesOpen] = useState(false);
  const [editingParticipantId, setEditingParticipantId] = useState<string | null>(null);
  const roomSettingsButtonRef = useRef<HTMLButtonElement>(null);
  const roomSettingsPanelRef = useRef<HTMLElement>(null);
  const appearanceButtonRef = useRef<HTMLButtonElement>(null);
  const appearancePanelRef = useRef<HTMLElement>(null);
  const preferencesButtonRef = useRef<HTMLButtonElement>(null);
  const preferencesPanelRef = useRef<HTMLElement>(null);
  const {
    artwork,
    artworkEditorMessage,
    appearanceSaveStatus,
    backgroundColor,
    backgroundImageUrl,
    chooseArtworkImage,
    chooseBackgroundImage,
    clearArtworkImage,
    clearBackgroundImage,
    closeAppearance,
    editArtwork,
    isAppearanceOpen,
    setBackgroundColor,
    surfaceStyle,
    toggleAppearance,
    workbenchRef,
  } = useAppearance();
  const roomStatusLabel =
    roomSourceMode === "error"
      ? t("coreOffline")
      : roomSourceMode === "loading"
        ? t("coreConnecting")
        : roomSourceMode === "backend"
          ? t("coreReady")
          : t("previewReady");
  const roomStatusReady =
    coreReady && roomSourceMode !== "loading" && roomSourceMode !== "error";
  const targetsBackendRoom = "__TAURI_INTERNALS__" in window;
  const usesBackendWrite = targetsBackendRoom && roomSourceMode === "backend";
  const hasUnconnectedRecipient = selectedRecipients.some(
    (participant) => aiConnections[participant.id]?.state !== "ready",
  );
  useEffect(() => {
    const openPanel = isRoomSettingsOpen
      ? roomSettingsPanelRef.current
      : isAppearanceOpen
        ? appearancePanelRef.current
        : isPreferencesOpen
          ? preferencesPanelRef.current
          : null;
    if (!openPanel) return;
    const frame = window.requestAnimationFrame(() => {
      openPanel.querySelector<HTMLElement>(
        "button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled)",
      )?.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [isAppearanceOpen, isPreferencesOpen, isRoomSettingsOpen]);

  useEffect(() => {
    if (!isRoomSettingsOpen && !isAppearanceOpen && !isPreferencesOpen) return;
    const dismiss = () => {
      clearRoomMutationError();
      setRoomSettingsOpen(false);
      closeAppearance();
      setPreferencesOpen(false);
    };
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      const insideOpenLayer =
        (isRoomSettingsOpen && (roomSettingsPanelRef.current?.contains(target) || roomSettingsButtonRef.current?.contains(target))) ||
        (isAppearanceOpen && (appearancePanelRef.current?.contains(target) || appearanceButtonRef.current?.contains(target))) ||
        (isPreferencesOpen && (preferencesPanelRef.current?.contains(target) || preferencesButtonRef.current?.contains(target)));
      if (!insideOpenLayer) dismiss();
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      const trigger = isRoomSettingsOpen
        ? roomSettingsButtonRef.current
        : isAppearanceOpen
          ? appearanceButtonRef.current
          : preferencesButtonRef.current;
      event.preventDefault();
      dismiss();
      window.requestAnimationFrame(() => trigger?.focus());
    };
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [clearRoomMutationError, closeAppearance, isAppearanceOpen, isPreferencesOpen, isRoomSettingsOpen]);

  return (
    <main className="moe-window" style={surfaceStyle}>
      <WindowResizeHandles />
      <section
        className="moe-workbench"
        aria-label={t("appLabel")}
        ref={workbenchRef}
      >
        <ArtworkStage artwork={artwork} />
        <SidebarResizeHandle />

        <RoomSidebar
          activeRoomId={activeRoom.id}
          coreLabel={roomStatusLabel}
          coreReady={roomStatusReady}
          onCreateRoom={() => {
            setRoomSettingsOpen(false);
            setPreferencesOpen(false);
            closeAppearance();
            closeParticipantMenu();
            void createRoom();
          }}
          onSelectRoom={(roomId) => {
            setRoomSettingsOpen(false);
            setPreferencesOpen(false);
            closeAppearance();
            closeParticipantMenu();
            selectRoom(roomId);
          }}
          rooms={rooms}
        />

        <section
          className={`room-workspace ${backgroundImageUrl ? "has-custom-background" : ""}`}
          aria-labelledby="current-room-title"
        >
          <header className="workspace-header" data-tauri-drag-region="">
            <div className="room-heading" data-tauri-drag-region="">
              <span className="room-kicker" data-tauri-drag-region="">
                TALK ROOM
              </span>
              <h1 data-tauri-drag-region="" id="current-room-title">
                {activeRoom.name}
              </h1>
            </div>

            <div
              aria-label="M.I.O. Malevolent Immortal Overdrive"
              className="brand-lockup"
              data-tauri-drag-region=""
            >
              <img
                alt=""
                aria-hidden="true"
                data-tauri-drag-region=""
                draggable={false}
                src={mioLogoUrl}
              />
              <span data-tauri-drag-region="">Malevolent Immortal Overdrive</span>
            </div>

            <button
              aria-expanded={isRoomSettingsOpen}
              className="icon-button room-settings-button"
              ref={roomSettingsButtonRef}
              onClick={() => {
                closeAppearance();
                setPreferencesOpen(false);
                clearRoomMutationError();
                closeParticipantMenu();
                setRoomSettingsOpen((isOpen) => !isOpen);
              }}
              title={t("roomSettings")}
              type="button"
            >
              <span aria-hidden="true">⚙</span>
              <span className="sr-only">{t("roomSettings")}</span>
            </button>

            <button
              aria-expanded={isAppearanceOpen}
              className="icon-button appearance-button"
              ref={appearanceButtonRef}
              onClick={() => {
                setRoomSettingsOpen(false);
                setPreferencesOpen(false);
                closeParticipantMenu();
                toggleAppearance();
              }}
              title={t("appearance")}
              type="button"
            >
              <span aria-hidden="true">✦</span>
              <span className="sr-only">{t("appearance")}</span>
            </button>

            <button
              aria-expanded={isPreferencesOpen}
              className="icon-button preferences-button"
              onClick={() => {
                setRoomSettingsOpen(false);
                closeAppearance();
                closeParticipantMenu();
                setPreferencesOpen((isOpen) => !isOpen);
              }}
              ref={preferencesButtonRef}
              title={t("preferences")}
              type="button"
            >
              <span aria-hidden="true">Aあ</span>
              <span className="sr-only">{t("preferences")}</span>
            </button>

            <WindowControls />

            {isRoomSettingsOpen ? (
              <RoomSettingsPanel
                error={roomMutationError}
                dataMessage={roomDataMessage}
                isBusy={isSending}
                onClose={() => {
                  clearRoomMutationError();
                  setRoomSettingsOpen(false);
                  window.requestAnimationFrame(() => roomSettingsButtonRef.current?.focus());
                }}
                onConfigureConductor={configureRoomConductor}
                onDelete={deleteRoom}
                onEditParticipantProfile={(participantId) => {
                  setRoomSettingsOpen(false);
                  setEditingParticipantId(participantId);
                }}
                onBackup={backupRooms}
                onChooseWorkspace={chooseWorkspace}
                onClearWorkspace={clearWorkspace}
                onRemoveParticipant={removeParticipant}
                onResetAiContinuity={resetAiContinuity}
                onRename={renameRoom}
                onRestoreLatest={restoreLatestBackup}
                participants={participants}
                panelRef={roomSettingsPanelRef}
                room={activeRoom}
                roomConductor={roomConductor}
                workspace={roomWorkspace}
              />
            ) : null}

            {isAppearanceOpen ? (
              <AppearancePanel
                artworkEditorMessage={artworkEditorMessage}
                appearanceSaveStatus={appearanceSaveStatus}
                backgroundColor={backgroundColor}
                hasArtworkImage={Boolean(artwork)}
                hasBackgroundImage={Boolean(backgroundImageUrl)}
                onArtworkImageChange={chooseArtworkImage}
                onBackgroundColorChange={setBackgroundColor}
                onBackgroundImageChange={chooseBackgroundImage}
                onClearArtworkImage={clearArtworkImage}
                onClearBackgroundImage={clearBackgroundImage}
                onClose={() => {
                  closeAppearance();
                  window.requestAnimationFrame(() => appearanceButtonRef.current?.focus());
                }}
                onEditArtwork={editArtwork}
                panelRef={appearancePanelRef}
              />
            ) : null}

            {isPreferencesOpen ? (
              <PreferencesPanel
                onClose={() => {
                  setPreferencesOpen(false);
                  window.requestAnimationFrame(() => preferencesButtonRef.current?.focus());
                }}
                panelRef={preferencesPanelRef}
              />
            ) : null}
          </header>

          <ParticipantBar
            conductorId={roomConductor.conductorId}
            connections={aiConnections}
            availableParticipants={availableParticipants}
            isMenuOpen={isParticipantMenuOpen}
            onAddParticipant={addParticipant}
            onMenuClose={closeParticipantMenu}
            onMenuToggle={toggleParticipantMenu}
            onToggleRecipient={toggleRecipient}
            participants={roomParticipants.filter((participant) => participant.kind === "ai")}
            recipientSelectionLocked={roomConductor.sendMode === "conductor"}
            selectedRecipientIds={recipientIds}
          />

          <ConversationPanel
            messages={activeRoom.messages}
            participants={participants}
            typingParticipantId={typingParticipantId}
          />

          <MessageComposer
            conductor={
              roomConductor.conductorId
                ? participants[roomConductor.conductorId] ?? null
                : null
            }
            hint={
              usesBackendWrite
                ? isAwaitingReply
                  ? t("awaitingHint")
                  : hasUnconnectedRecipient
                    ? t("unconnectedHint")
                    : t("sendHint")
                : targetsBackendRoom
                  ? t("roomUnavailableHint")
                  : t("demoHint")
            }
            isAvailable={!targetsBackendRoom || roomSourceMode === "backend"}
            isAwaitingReply={isAwaitingReply}
            isSending={isSending}
            onDismissDispatchSafetyWarning={dismissDispatchSafetyWarning}
            onRemoveRecipient={toggleRecipient}
            onSendModeChange={changeConductorSendMode}
            onSend={sendMessage}
            recipients={selectedRecipients}
            sendMode={roomConductor.sendMode}
            dispatchSafetyWarning={dispatchSafetyWarning}
            sendError={sendError}
            sendNotice={sendNotice}
          />
        </section>
      </section>
      {editingParticipantId && participants[editingParticipantId] ? (
        <ParticipantProfileEditor
          key={editingParticipantId}
          onClose={() => setEditingParticipantId(null)}
          onSave={saveParticipantProfile}
          participant={participants[editingParticipantId]}
          profile={participantProfiles[editingParticipantId]}
          roomWorkspace={roomWorkspace}
        />
      ) : null}
    </main>
  );
}
