import { useEffect, useRef, useState } from "react";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

import mioLogoUrl from "./assets/mio-logo.svg";
import { AppearancePanel } from "./components/AppearancePanel";
import { ArtworkStage } from "./components/ArtworkStage";
import { CommandConfirmationDialog } from "./components/CommandConfirmationDialog";
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
import { useCommandConfirmations } from "./hooks/useCommandConfirmations";
import { useRooms } from "./hooks/useRooms";
import { useContextMenuPolicy } from "./useContextMenuPolicy";
import { useUiPreferences } from "./uiPreferences";

const compactWindowWidth = 1080;
const isTauri = "__TAURI_INTERNALS__" in window;

function SidebarIcon() {
  return (
    <svg aria-hidden="true" className="sidebar-toggle-icon" viewBox="0 0 20 20">
      <rect height="14" rx="2.5" width="15" x="2.5" y="3" />
      <path d="M7.5 3v14" />
    </svg>
  );
}

async function currentWindowGeometry() {
  const appWindow = getCurrentWindow();
  if (await appWindow.isMaximized()) {
    await appWindow.unmaximize();
  }
  const scaleFactor = await appWindow.scaleFactor();
  return {
    appWindow,
    size: (await appWindow.outerSize()).toLogical(scaleFactor),
  };
}

export function App() {
  useContextMenuPolicy();
  const {
    expandedWindowWidth,
    imageFreeMode,
    setExpandedWindowWidth,
    setImageFreeMode,
    setSidebarCollapsed,
    sidebarCollapsed,
    t,
  } = useUiPreferences();
  const { coreReady, toolchainReady } = useBootstrapStatus();
  const {
    activeRoom,
    activeTurnRoomId,
    aiConnections,
    addParticipant,
    availableParticipants,
    backupRooms,
    changeConductorSendMode,
    chooseBackupDirectory,
    chooseWorkspace,
    codexTurnProgress,
    createRoom,
    closeParticipantMenu,
    clearWorkspace,
    clearRoomMutationError,
    configureRoomConductor,
    deleteRoom,
    dismissDispatchSafetyWarning,
    dismissSendError,
    dispatchSafetyWarning,
    isParticipantMenuOpen,
    isAwaitingReply,
    isAnotherRoomAwaitingReply,
    isCancelling,
    isSending,
    openBackupDirectory,
    participants,
    participantProfiles,
    recipientIds,
    roomParticipants,
    rooms,
    roomSourceMode,
    roomMutationError,
    roomBackupStatus,
    roomConfigurationReady,
    roomConductor,
    roomRestorePreview,
    roomWorkspace,
    roomDataMessage,
    removeParticipant,
    resetAiContinuity,
    renameRoom,
    previewLatestBackup,
    restorePreviewedBackup,
    saveParticipantProfile,
    sendError,
    sendNotice,
    selectedRecipients,
    selectRoom,
    sendMessage,
    cancelActiveTurn,
    toggleParticipantMenu,
    toggleRecipient,
    typingParticipantId,
    useDefaultBackupDirectory,
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
          ? t(toolchainReady ? "coreReady" : "coreReadyToolsChecking")
          : t("previewReady");
  const roomStatusReady =
    coreReady && roomSourceMode !== "loading" && roomSourceMode !== "error";
  const targetsBackendRoom = "__TAURI_INTERNALS__" in window;
  const usesBackendWrite = targetsBackendRoom && roomSourceMode === "backend";
  const commandConfirmations = useCommandConfirmations({
    enabled: usesBackendWrite,
    roomId: activeRoom.id,
    waiting: isAwaitingReply,
  });
  const activeTurnRoomName = activeTurnRoomId
    ? rooms.find((room) => room.id === activeTurnRoomId)?.name ?? activeTurnRoomId
    : null;
  const hasUnconnectedRecipient = selectedRecipients.some(
    (participant) => {
      const state = aiConnections[participant.id]?.state;
      return state === "setupRequired" || state === "unsupported";
    },
  );
  useEffect(() => {
    if (!imageFreeMode || !isTauri) return;
    void currentWindowGeometry()
      .then(async ({ appWindow, size }) => {
        await appWindow.setSize(new LogicalSize(compactWindowWidth, Math.max(640, size.height)));
      })
      .catch((reason: unknown) => {
        console.error("Initial display-mode window resize failed", reason);
      });
    // The saved mode is applied once when this window is created.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function toggleImageFreeMode() {
    const nextMode = !imageFreeMode;
    if (!isTauri) {
      setImageFreeMode(nextMode);
      return;
    }
    try {
      const { appWindow, size } = await currentWindowGeometry();
      let nextExpandedWindowWidth = expandedWindowWidth;
      if (nextMode && size.width > compactWindowWidth) {
        nextExpandedWindowWidth = size.width;
        setExpandedWindowWidth(size.width);
      }
      await appWindow.setSize(new LogicalSize(
        nextMode ? compactWindowWidth : nextExpandedWindowWidth,
        Math.max(640, size.height),
      ));
      setImageFreeMode(nextMode);
    } catch (reason) {
      console.error("Image-free window resize failed", reason);
    }
  }

  async function toggleSidebar() {
    setSidebarCollapsed(!sidebarCollapsed);
  }
  useEffect(() => {
    const workbench = workbenchRef.current;
    if (!commandConfirmations.activeRequest || !workbench) return;
    workbench.inert = true;
    return () => {
      workbench.inert = false;
    };
  }, [commandConfirmations.activeRequest, workbenchRef]);

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
    <main className={`moe-window ${imageFreeMode ? "is-image-free" : ""}`} style={surfaceStyle}>
      <WindowResizeHandles />
      <section
        className={`moe-workbench ${sidebarCollapsed ? "is-sidebar-collapsed" : ""}`}
        aria-label={t("appLabel")}
        ref={workbenchRef}
      >
        <ArtworkStage artwork={imageFreeMode ? null : artwork} />
        {!sidebarCollapsed ? <SidebarResizeHandle /> : null}

        {!sidebarCollapsed ? (
          <RoomSidebar
            activeRoomId={activeRoom.id}
            activeTurnRoomId={activeTurnRoomId}
            coreLabel={roomStatusLabel}
            coreReady={roomStatusReady}
            isBusy={isSending}
            onCreateRoom={() => {
              setRoomSettingsOpen(false);
              setPreferencesOpen(false);
              closeAppearance();
              closeParticipantMenu();
              void createRoom();
            }}
            onDeleteRoom={deleteRoom}
            onOpenRoomSettings={(roomId) => {
              setPreferencesOpen(false);
              closeAppearance();
              closeParticipantMenu();
              clearRoomMutationError();
              selectRoom(roomId);
              setRoomSettingsOpen(true);
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
        ) : null}

        <section
          className={`room-workspace ${!imageFreeMode && backgroundImageUrl ? "has-custom-background" : ""}`}
          aria-labelledby="current-room-title"
        >
          <header className="workspace-header" data-tauri-drag-region="">
            <button
              aria-expanded={!sidebarCollapsed}
              className="icon-button sidebar-toggle-button"
              onClick={() => void toggleSidebar()}
              title={t(sidebarCollapsed ? "showRoomList" : "hideRoomList")}
              type="button"
            >
              <SidebarIcon />
              <span className="sr-only">{t(sidebarCollapsed ? "showRoomList" : "hideRoomList")}</span>
            </button>

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
              aria-pressed={imageFreeMode}
              className={`icon-button image-mode-button ${imageFreeMode ? "is-active" : ""}`}
              onClick={() => void toggleImageFreeMode()}
              title={t(imageFreeMode ? "showDecorativeImages" : "hideDecorativeImages")}
              type="button"
            >
              <span aria-hidden="true">{imageFreeMode ? "□" : "▧"}</span>
              <span className="sr-only">{t(imageFreeMode ? "showDecorativeImages" : "hideDecorativeImages")}</span>
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
                backupStatus={roomBackupStatus}
                error={roomMutationError}
                dataMessage={roomDataMessage}
                isBusy={isSending || activeTurnRoomId !== null || !roomConfigurationReady}
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
                onChooseBackupDirectory={chooseBackupDirectory}
                onChooseWorkspace={chooseWorkspace}
                onClearWorkspace={clearWorkspace}
                onOpenBackupDirectory={openBackupDirectory}
                onPreviewLatestBackup={previewLatestBackup}
                onRemoveParticipant={removeParticipant}
                onResetAiContinuity={resetAiContinuity}
                onRename={renameRoom}
                onRestorePreviewedBackup={restorePreviewedBackup}
                onUseDefaultBackupDirectory={useDefaultBackupDirectory}
                participants={participants}
                panelRef={roomSettingsPanelRef}
                room={activeRoom}
                restorePreview={roomRestorePreview}
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
            isLocked={isAwaitingReply || !roomConfigurationReady}
            onAddParticipant={addParticipant}
            onMenuClose={closeParticipantMenu}
            onMenuToggle={toggleParticipantMenu}
            onToggleRecipient={toggleRecipient}
            participants={roomParticipants.filter((participant) => participant.kind === "ai")}
            recipientSelectionLocked={roomConductor.sendMode === "conductor"}
            selectedRecipientIds={recipientIds}
          />

          <ConversationPanel
            codexTurnProgress={codexTurnProgress}
            messages={activeRoom.messages}
            participants={participants}
            roomId={activeRoom.id}
            typingParticipantId={isAwaitingReply ? typingParticipantId : null}
          />

          <MessageComposer
            conductor={
              roomConductor.conductorId
                ? participants[roomConductor.conductorId] ?? null
                : null
            }
            hint={
              usesBackendWrite
                ? !roomConfigurationReady
                  ? t("roomSettingsLoadingHint")
                  : isAnotherRoomAwaitingReply && activeTurnRoomName
                  ? t("anotherRoomAwaitingHint", { name: activeTurnRoomName })
                  : isAwaitingReply
                  ? commandConfirmations.activeRequest
                    ? t("commandConfirmationWaitingHint")
                    : t("awaitingHint")
                  : hasUnconnectedRecipient
                    ? t("unconnectedHint")
                    : t("sendHint")
                : targetsBackendRoom
                  ? t("roomUnavailableHint")
                  : t("demoHint")
            }
            isBackgroundTurn={isAnotherRoomAwaitingReply}
            isAvailable={
              (!targetsBackendRoom || roomSourceMode === "backend") &&
              roomConfigurationReady &&
              !isAnotherRoomAwaitingReply
            }
            isAwaitingReply={isAwaitingReply}
            isCancelling={isAwaitingReply && isCancelling}
            isSending={isSending}
            onCancel={cancelActiveTurn}
            onDismissDispatchSafetyWarning={dismissDispatchSafetyWarning}
            onDismissSendError={dismissSendError}
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
      {commandConfirmations.activeRequest ? (
        <CommandConfirmationDialog
          error={commandConfirmations.error}
          isResolving={commandConfirmations.isResolving}
          onDecision={commandConfirmations.resolve}
          remainingCount={commandConfirmations.remainingCount}
          request={commandConfirmations.activeRequest}
        />
      ) : null}
    </main>
  );
}
