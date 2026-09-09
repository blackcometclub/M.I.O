import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import {
  demoParticipants,
  initialRecipientIds,
  initialRooms,
  ownerParticipantId,
} from "../mockData";
import {
  activateDesktopCommandRoomSession,
  deactivateDesktopCommandRoomSession,
} from "../commandConfirmationBridge";
import {
  addDesktopRoomParticipant,
  backupDesktopRooms,
  browserBridgeReplyView,
  cancelDesktopRoomTurn,
  chooseDesktopRoomBackupDirectory,
  chooseDesktopRoomWorkspace,
  clearDesktopRoomWorkspace,
  clearDesktopRoomConductor,
  createDesktopRoom,
  deleteDesktopRoom,
  desktopRoomBackupStatusView,
  dispatchDesktopRoomRecipient,
  orchestrateDesktopRoomMessage,
  openDesktopRoomBackupDirectory,
  previewLatestDesktopRoomBackup,
  readDesktopAiConnectionStatuses,
  readDesktopRoomConductorStatus,
  readDesktopRoomDispatchUnknowns,
  readDesktopRoom,
  readDesktopRoomBackupStatus,
  readDesktopRoomWorkspaceStatus,
  readDesktopRooms,
  resetDesktopRoomAiContinuity,
  removeDesktopRoomParticipant,
  renameDesktopRoom,
  restoreDesktopRoomBackup,
  saveDesktopRoomConductorMode,
  setDesktopRoomConductor,
  useDefaultDesktopRoomBackupDirectory,
  writeDesktopRoomMessage,
} from "../roomBridge";
import { isBundledRoom } from "../roomPolicies";
import {
  readParticipantProfiles,
  saveParticipantProfile as persistParticipantProfile,
} from "../participantProfileBridge";
import type {
  AiConnectionMap,
  ChatMessage,
  CodexTurnProgress,
  CodexTurnProgressPhase,
  ImageGenerationPreferences,
  ParticipantMap,
  ParticipantProfile,
  Room,
  RoomBackupPreview,
  RoomBackupStatus,
  RoomConductorStatus,
  RoomWorkspaceStatus,
} from "../types";
import { useUiPreferences } from "../uiPreferences";
import { readRememberedRoomId, rememberRoomId, resolveSelectedRoom } from "../roomSelection";

export type RoomSourceMode = "loading" | "backend" | "browserDemo" | "error";

const dismissedDispatchUnknownsStorageKey = "moe-dismissed-dispatch-unknowns-v1";
const maximumDismissedDispatchUnknowns = 512;
const codexTurnProgressPhases = new Set<CodexTurnProgressPhase>([
  "preparing",
  "thinking",
  "reconnecting",
  "workspace",
  "tool",
  "generatingImage",
  "writingResponse",
]);

type DispatchUnknown = Awaited<ReturnType<typeof readDesktopRoomDispatchUnknowns>>[number];

function dispatchUnknownKey(roomId: string, unknown: DispatchUnknown) {
  return JSON.stringify([roomId, unknown.sourceMessageId, unknown.recipientId]);
}

function loadDismissedDispatchUnknowns() {
  try {
    const value = JSON.parse(
      localStorage.getItem(dismissedDispatchUnknownsStorageKey) ?? "[]",
    ) as unknown;
    if (!Array.isArray(value)) return [];
    return Array.from(new Set(value.filter(
      (item): item is string => typeof item === "string" && item.length <= 512,
    ))).slice(-maximumDismissedDispatchUnknowns);
  } catch {
    return [];
  }
}

function persistDismissedDispatchUnknowns(keys: string[]) {
  try {
    localStorage.setItem(dismissedDispatchUnknownsStorageKey, JSON.stringify(keys));
  } catch {
    // The warning can still be dismissed for this session when storage is unavailable.
  }
}

function createId(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

function profileParticipants(
  canonicalParticipants: ParticipantMap,
  profiles: Record<string, ParticipantProfile>,
) {
  return Object.fromEntries(
    Object.entries(canonicalParticipants).map(([participantId, participant]) => {
      const profile = profiles[participantId];
      if (!profile) return [participantId, participant];
      return [participantId, {
        ...participant,
        displayName: profile.displayName,
        initials: Array.from(profile.displayName).slice(0, 2).join(""),
        avatarUrl: profile.avatar?.dataUrl ?? participant.avatarUrl,
        avatarPlacement: profile.avatar
          ? { scale: profile.avatar.scale, x: profile.avatar.x, y: profile.avatar.y }
          : participant.avatarPlacement,
      }];
    }),
  ) as ParticipantMap;
}

export function useRooms() {
  const { locale } = useUiPreferences();
  const text = (japanese: string, english: string) => locale === "ja" ? japanese : english;
  const [rooms, setRooms] = useState<Room[]>(initialRooms);
  const [canonicalParticipants, setCanonicalParticipants] = useState<ParticipantMap>(demoParticipants);
  const [participantProfiles, setParticipantProfiles] = useState<Record<string, ParticipantProfile>>({});
  const [aiConnections, setAiConnections] = useState<AiConnectionMap>({});
  const [activeRoomId, setActiveRoomId] = useState(() =>
    readRememberedRoomId("__TAURI_INTERNALS__" in window ? "backend" : "browserDemo")
      ?? initialRooms[0].id,
  );
  const [recipientIds, setRecipientIds] = useState(initialRecipientIds);
  const [isParticipantMenuOpen, setParticipantMenuOpen] = useState(false);
  const [typingParticipantId, setTypingParticipantId] = useState<string | null>(null);
  const [codexTurnProgress, setCodexTurnProgress] = useState<CodexTurnProgress | null>(null);
  const [roomSourceMode, setRoomSourceMode] = useState<RoomSourceMode>("loading");
  const [isSending, setSending] = useState(false);
  const [activeTurnRoomId, setActiveTurnRoomId] = useState<string | null>(null);
  const [isCancelling, setCancelling] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  const [sendNotice, setSendNotice] = useState<string | null>(null);
  const [sendFeedbackRoomId, setSendFeedbackRoomId] = useState(initialRooms[0].id);
  const [dispatchSafetyWarning, setDispatchSafetyWarning] = useState<string | null>(null);
  const [dispatchSafetyWarningKeys, setDispatchSafetyWarningKeys] = useState<string[]>([]);
  const [dismissedDispatchUnknownKeys, setDismissedDispatchUnknownKeys] = useState(
    loadDismissedDispatchUnknowns,
  );
  const [dispatchSafetyRevision, setDispatchSafetyRevision] = useState(0);
  const [roomMutationError, setRoomMutationError] = useState<string | null>(null);
  const [roomDataMessage, setRoomDataMessage] = useState<string | null>(null);
  const [roomBackupStatus, setRoomBackupStatus] = useState<RoomBackupStatus>({
    directoryPath: "",
    isCustom: false,
    available: false,
  });
  const [roomRestorePreview, setRoomRestorePreview] = useState<RoomBackupPreview | null>(null);
  const [roomWorkspace, setRoomWorkspace] = useState<RoomWorkspaceStatus>({
    roomId: initialRooms[0].id,
    mode: "chatOnly",
    folderName: null,
    available: true,
  });
  const [roomConductor, setRoomConductor] = useState<RoomConductorStatus>({
    roomId: initialRooms[0].id,
    conductorId: null,
    sendMode: "direct",
  });
  const [workspaceStatusKey, setWorkspaceStatusKey] = useState<string | null>(null);
  const [conductorStatusKey, setConductorStatusKey] = useState<string | null>(null);
  const isSendingRef = useRef(false);
  const isCancellingRef = useRef(false);
  const stopRequestedRef = useRef(false);
  const activeTurnRef = useRef<{ messageId: string; roomId: string } | null>(null);
  const activeRoomIdRef = useRef(activeRoomId);
  const directRecipientIdsRef = useRef<Record<string, string[]>>({
    [initialRooms[0].id]: initialRecipientIds,
  });
  const pendingWrite = useRef<{
    body: string;
    messageId: string;
    recipientIds: string[];
    roomId: string;
  } | null>(null);
  const replyTimers = useRef<Set<number>>(new Set());

  const participants = useMemo(
    () => profileParticipants(canonicalParticipants, participantProfiles),
    [canonicalParticipants, participantProfiles],
  );

  const activeRoom = useMemo(
    () => rooms.find((room) => room.id === activeRoomId) ?? rooms[0],
    [activeRoomId, rooms],
  );

  const isAwaitingReply = activeTurnRoomId === activeRoom.id;
  const isAnotherRoomAwaitingReply =
    activeTurnRoomId !== null && activeTurnRoomId !== activeRoom.id;
  const activeRoomConfigurationKey = `${roomSourceMode}:${activeRoom.id}`;
  const workspaceStatusReady = workspaceStatusKey === activeRoomConfigurationKey;
  const conductorStatusReady = conductorStatusKey === activeRoomConfigurationKey;
  const roomConfigurationReady = workspaceStatusReady && conductorStatusReady;

  useEffect(() => {
    activeRoomIdRef.current = activeRoomId;
  }, [activeRoomId]);

  useEffect(() => {
    // Loading/error screens may show a bundled placeholder Room. Never let that
    // placeholder overwrite the selection before the saved catalog is available.
    if (roomSourceMode === "backend" || roomSourceMode === "browserDemo") {
      rememberRoomId(roomSourceMode, activeRoom.id);
    }
  }, [activeRoom.id, roomSourceMode]);

  const roomParticipants = useMemo(
    () =>
      activeRoom.participantIds
        .map((participantId) => participants[participantId])
        .filter((participant) => participant !== undefined),
    [activeRoom, participants],
  );

  const selectedRecipients = roomParticipants.filter(
    (participant) =>
      participant.kind === "ai" && recipientIds.includes(participant.id),
  );

  const availableParticipants = Object.values(participants).filter(
    (participant) =>
      participant.kind === "ai" && !activeRoom.participantIds.includes(participant.id),
  );

  useEffect(() => {
    function applyInitialRooms(
      hydration: { rooms: Room[]; participants: ParticipantMap },
      mode: "backend" | "browserDemo",
    ) {
      const nextRoom = resolveSelectedRoom(hydration.rooms, activeRoomIdRef.current);
      setCanonicalParticipants((current) => ({ ...current, ...hydration.participants }));
      setRooms(hydration.rooms);
      if (nextRoom) {
        const firstAi = nextRoom.participantIds.find(
          (id) => hydration.participants[id]?.kind === "ai",
        );
        const nextRecipients = directRecipientIdsRef.current[nextRoom.id]?.filter(
          (id) => nextRoom.participantIds.includes(id),
        ) ?? (firstAi ? [firstAi] : []);
        directRecipientIdsRef.current[nextRoom.id] = nextRecipients;
        activeRoomIdRef.current = nextRoom.id;
        setActiveRoomId(nextRoom.id);
        setRecipientIds(nextRecipients);
      }
      setRoomSourceMode(mode);
    }

    if (!("__TAURI_INTERNALS__" in window)) {
      applyInitialRooms({ rooms: initialRooms, participants: demoParticipants }, "browserDemo");
      return;
    }

    let cancelled = false;
    void readDesktopRooms()
      .then((hydration) => {
        if (cancelled) {
          return;
        }
        applyInitialRooms(hydration, "backend");
      })
      .catch(() => {
        if (!cancelled) {
          setRoomSourceMode("error");
        }
      });
    void readDesktopAiConnectionStatuses()
      .then((connections) => {
        if (!cancelled) {
          setAiConnections(connections);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setAiConnections({});
        }
      });
    void readParticipantProfiles()
      .then((profiles) => {
        if (!cancelled) {
          setParticipantProfiles(Object.fromEntries(
            profiles.map((profile) => [profile.participantId, profile]),
          ));
        }
      })
      .catch(() => {
        if (!cancelled) setParticipantProfiles({});
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    const refreshConnections = () => {
      void readDesktopAiConnectionStatuses()
        .then((connections) => {
          if (!disposed) setAiConnections(connections);
        })
        .catch(() => {
          if (!disposed) setAiConnections({});
        });
    };
    refreshConnections();
    const timer = window.setInterval(refreshConnections, 4_000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [roomSourceMode]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<{
      roomId?: unknown;
      dispatchId?: unknown;
      phase?: unknown;
    }>("mio-codex-turn-progress", (event) => {
      if (disposed) return;
      const { roomId, dispatchId, phase } = event.payload ?? {};
      if (
        typeof roomId !== "string" ||
        typeof dispatchId !== "string" ||
        typeof phase !== "string" ||
        !codexTurnProgressPhases.has(phase as CodexTurnProgressPhase) ||
        activeTurnRef.current?.roomId !== roomId
      ) {
        return;
      }
      setCodexTurnProgress((current) => ({
        roomId,
        dispatchId,
        phase: phase as CodexTurnProgressPhase,
        startedAt: current?.roomId === roomId ? current.startedAt : Date.now(),
      }));
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      })
      .catch(() => {
        // Progress is optional; message delivery continues if the listener is unavailable.
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [roomSourceMode]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<unknown>("moe-browser-bridge-reply", (event) => {
      if (disposed) return;
      try {
        const reply = browserBridgeReplyView(event.payload, participants);
        setRooms((currentRooms) => currentRooms.map((room) =>
          room.id === reply.roomId
            ? {
                ...room,
                messages: room.messages.some((message) => message.id === reply.message.id)
                  ? room.messages
                  : [...room.messages, reply.message],
                updatedLabel: "いま",
              }
            : room,
        ));
        setTypingParticipantId((current) => {
          if (current !== "gemini") return current;
          return activeTurnRef.current ? current : null;
        });
      } catch {
        const roomId = activeRoomIdRef.current;
        setSendFeedbackRoomId(roomId);
        setSendError(text(
          "Gemini Searchの返答を安全に確認できませんでした。Roomには追加していません。",
          "The Gemini Search reply could not be validated and was not added to the Room.",
        ));
      }
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      })
      .catch(() => {
        if (!disposed) {
          const roomId = activeRoomIdRef.current;
          setSendFeedbackRoomId(roomId);
          setSendError(text(
            "Gemini Searchの返答待受を開始できませんでした。",
            "The Gemini Search reply listener could not be started.",
          ));
        }
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [locale, participants, roomSourceMode]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<{ roomId: string }>("mio-room-message-saved", (event) => {
      const roomId = event.payload?.roomId;
      if (disposed || typeof roomId !== "string") {
        return;
      }
      void readDesktopRoom(roomId)
        .then((hydration) => {
          if (disposed || hydration.room.id !== roomId) {
            return;
          }
          setCanonicalParticipants((current) => ({
            ...current,
            ...hydration.participants,
          }));
          setRooms((currentRooms) => currentRooms.map((room) =>
            room.id === roomId ? hydration.room : room,
          ));
        })
        .catch(() => {
          if (!disposed) {
            setSendFeedbackRoomId(roomId);
            setSendError(text(
              "Codexから保存されたメッセージを再読込できませんでした。",
              "The message saved via Codex could not be reloaded.",
            ));
          }
        });
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      })
      .catch(() => {
        if (!disposed) {
          const roomId = activeRoomIdRef.current;
          setSendFeedbackRoomId(roomId);
          setSendError(text(
            "Codexからのメッセージ通知を待受できませんでした。",
            "The via-Codex message listener could not be started.",
          ));
        }
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [locale, roomSourceMode]);

  useEffect(() => {
    const roomId = activeRoom.id;
    const statusKey = `${roomSourceMode}:${roomId}`;
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      setRoomWorkspace({ roomId, mode: "chatOnly", folderName: null, available: true });
      setWorkspaceStatusKey(statusKey);
      return;
    }
    let cancelled = false;
    setWorkspaceStatusKey(null);
    void readDesktopRoomWorkspaceStatus(roomId)
      .then((status) => {
        if (!cancelled) {
          setRoomWorkspace(status);
          setWorkspaceStatusKey(statusKey);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRoomWorkspace({ roomId, mode: "chatOnly", folderName: null, available: false });
          setWorkspaceStatusKey(statusKey);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeRoom.id, roomSourceMode]);

  useEffect(() => {
    const roomId = activeRoom.id;
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    void activateDesktopCommandRoomSession(roomId).catch(() => {
      // A missing or changed workspace leaves command execution fail closed.
    });
  }, [activeRoom.id, roomSourceMode]);

  useEffect(() => {
    const roomId = activeRoom.id;
    const statusKey = `${roomSourceMode}:${roomId}`;
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      setRoomConductor({ roomId, conductorId: null, sendMode: "direct" });
      setConductorStatusKey(statusKey);
      return;
    }
    let cancelled = false;
    setConductorStatusKey(null);
    void readDesktopRoomConductorStatus(roomId)
      .then((status) => {
        if (cancelled) return;
        setRoomConductor(status);
        setConductorStatusKey(statusKey);
        if (status.sendMode === "conductor" && status.conductorId) {
          setRecipientIds((current) => {
            if (!directRecipientIdsRef.current[roomId]) {
              directRecipientIdsRef.current[roomId] = current;
            }
            return [status.conductorId!];
          });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRoomConductor({ roomId, conductorId: null, sendMode: "direct" });
          setConductorStatusKey(statusKey);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeRoom.id, roomSourceMode]);

  useEffect(() => {
    const roomId = activeRoom.id;
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let cancelled = false;
    void readDesktopRoomDispatchUnknowns(roomId)
      .then((unknowns) => {
        if (cancelled) {
          return;
        }
        const dismissed = new Set(dismissedDispatchUnknownKeys);
        const visibleUnknowns = unknowns.filter(
          (unknown) => !dismissed.has(dispatchUnknownKey(roomId, unknown)),
        );
        if (visibleUnknowns.length === 0) {
          setDispatchSafetyWarning(null);
          setDispatchSafetyWarningKeys([]);
          return;
        }
        const names = Array.from(
          new Set(
            visibleUnknowns.map(
              ({ recipientId }) => participants[recipientId]?.displayName ?? recipientId,
            ),
          ),
        );
        setDispatchSafetyWarningKeys(
          visibleUnknowns.map((unknown) => dispatchUnknownKey(roomId, unknown)),
        );
        setDispatchSafetyWarning(
          text(
            `${names.join("、")}への送信に、結果を確認できていないものが${visibleUnknowns.length}件あります。二重送信を防ぐため、自動再送していません。`,
            `There are ${visibleUnknowns.length} messages to ${names.join(", ")} whose results are unknown. They were not retried to prevent duplicate turns.`,
          ),
        );
      })
      .catch(() => {
        if (!cancelled) {
          setDispatchSafetyWarningKeys([]);
          setDispatchSafetyWarning(
            text(
              "AI送信の安全記録を確認できませんでした。自動再送はしていません。",
              "The AI delivery safety record could not be checked. Nothing was retried automatically.",
            ),
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [
    activeRoom.id,
    dismissedDispatchUnknownKeys,
    dispatchSafetyRevision,
    locale,
    participants,
    roomSourceMode,
  ]);

  function dismissDispatchSafetyWarning() {
    if (dispatchSafetyWarningKeys.length > 0) {
      setDismissedDispatchUnknownKeys((current) => {
        const next = Array.from(new Set([...current, ...dispatchSafetyWarningKeys]))
          .slice(-maximumDismissedDispatchUnknowns);
        persistDismissedDispatchUnknowns(next);
        return next;
      });
    }
    setDispatchSafetyWarning(null);
    setDispatchSafetyWarningKeys([]);
  }

  function dismissSendError() {
    setSendError(null);
  }

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let cancelled = false;
    void readDesktopRoomBackupStatus()
      .then((status) => {
        if (!cancelled) setRoomBackupStatus(status);
      })
      .catch(() => {
        if (!cancelled) {
          setRoomMutationError(text(
            "バックアップ保存先を確認できませんでした。",
            "The backup directory could not be checked.",
          ));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [locale, roomSourceMode]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<{
      changed: boolean;
      status: unknown;
      errorCode: string | null;
    }>("moe-room-backup-directory-choice", (event) => {
      if (disposed) return;
      isSendingRef.current = false;
      setSending(false);
      setRoomRestorePreview(null);
      if (event.payload.errorCode) {
        setRoomMutationError(text(
          "バックアップ保存先を設定できませんでした。選択したフォルダーをご確認ください。",
          "The backup directory could not be set. Check the selected folder.",
        ));
        return;
      }
      try {
        const status = desktopRoomBackupStatusView(event.payload.status);
        setRoomBackupStatus(status);
        setRoomDataMessage(event.payload.changed
          ? text("バックアップ保存先を変更しました。既存ファイルは移動していません。", "The backup directory was changed. Existing files were not moved.")
          : text("フォルダー選択をキャンセルしました。", "Folder selection was canceled."));
      } catch {
        setRoomMutationError(text(
          "バックアップ保存先の応答を確認できませんでした。",
          "The backup directory response could not be validated.",
        ));
      }
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      })
      .catch(() => {
        if (!disposed) {
          setRoomMutationError(text(
            "バックアップ保存先の応答を待受できませんでした。",
            "The backup directory response could not be received.",
          ));
        }
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [locale, roomSourceMode]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      return;
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<{
      roomId: string;
      changed: boolean;
      errorCode: string | null;
    }>("moe-room-workspace-choice", (event) => {
      if (disposed || event.payload.roomId !== activeRoom.id) {
        return;
      }
      isSendingRef.current = false;
      setSending(false);
      if (event.payload.errorCode) {
        setRoomMutationError(
          event.payload.errorCode === "roomWorkspaceUnsafeLink"
            ? text(
                "ジャンクションやシンボリックリンクのフォルダーは、安全のため作業フォルダーに設定できません。",
                "Junctions and symbolic-link folders cannot be used as a workspace for safety.",
              )
            : text("作業フォルダーを設定できませんでした。", "The workspace folder could not be set."),
        );
        return;
      }
      void readDesktopRoomWorkspaceStatus(activeRoom.id)
        .then(async (status) => {
          if (disposed) {
            return;
          }
          setRoomWorkspace(status);
          if (event.payload.changed) {
            const session = await activateDesktopCommandRoomSession(activeRoom.id);
            if (disposed) {
              return;
            }
            if (!session.active) {
              setRoomMutationError(text(
                "作業フォルダーは設定されましたが、コマンド用の安全なRoomセッションを開始できませんでした。",
                "The workspace was selected, but its safe Room command session could not be started.",
              ));
              return;
            }
          }
          setRoomDataMessage(
            event.payload.changed
              ? text(`${status.folderName ?? "選択フォルダー"}をCodex作業フォルダーに設定しました。`, `${status.folderName ?? "Selected folder"} is now the Codex workspace.`)
              : text("フォルダー選択をキャンセルしました。", "Folder selection was canceled."),
          );
        })
        .catch(() => {
          if (!disposed) {
            setRoomMutationError(text("作業フォルダーの状態を確認できませんでした。", "The workspace status could not be checked."));
          }
        });
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          stopListening = unlisten;
        }
      })
      .catch(() => {
        if (!disposed) {
          setRoomMutationError(text("作業フォルダーの応答を受け取れませんでした。", "No workspace response was received."));
        }
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [activeRoom.id, locale, roomSourceMode]);

  useEffect(() => {
    return () => {
      for (const timer of replyTimers.current) {
        window.clearTimeout(timer);
      }
    };
  }, []);

  function selectRoom(roomId: string) {
    if (isSendingRef.current) {
      return;
    }
    const nextRoom = rooms.find((room) => room.id === roomId);
    if (!nextRoom) {
      return;
    }

    if (activeRoom.id !== roomId) {
      setRoomMutationError(null);
      setRoomDataMessage(null);
    }

    if (
      roomSourceMode === "backend" &&
      activeRoom.id !== roomId &&
      activeTurnRef.current?.roomId !== activeRoom.id
    ) {
      void deactivateDesktopCommandRoomSession(activeRoom.id).catch(() => {
        // Command execution remains fail closed if the old session cannot be ended.
      });
    }

    const firstAi = nextRoom.participantIds.find(
      (participantId) => participants[participantId]?.kind === "ai",
    );

    activeRoomIdRef.current = roomId;
    setActiveRoomId(roomId);
    const nextRecipients = directRecipientIdsRef.current[roomId]?.filter((participantId) =>
      nextRoom.participantIds.includes(participantId)
    ) ?? (firstAi ? [firstAi] : []);
    directRecipientIdsRef.current[roomId] = nextRecipients;
    setRecipientIds(nextRecipients);
    setParticipantMenuOpen(false);
    if (!activeTurnRef.current) {
      setTypingParticipantId(null);
    }
  }

  async function createRoom() {
    if (isSendingRef.current) {
      return;
    }
    const roomNumber = rooms.length + 1;
    const draftRoom: Room = {
      id: createId("room"),
      name: text(`新しいルーム ${roomNumber}`, `New room ${roomNumber}`),
      participantIds: [ownerParticipantId, "codex"],
      messages: [],
      updatedLabel: "まだ会話なし",
    };
    let newRoom = draftRoom;
    if ("__TAURI_INTERNALS__" in window) {
      if (roomSourceMode !== "backend") {
        setSendError(text("Rust Roomに接続できないため、ルームを作成できません。", "A room cannot be created while Rust Room is unavailable."));
        return;
      }
      isSendingRef.current = true;
      setSending(true);
      setSendError(null);
      setSendNotice(null);
      try {
        newRoom = await createDesktopRoom({
          roomId: draftRoom.id,
          name: draftRoom.name,
        });
      } catch {
        setSendError(text("ルームを保存できませんでした。もう一度お試しください。", "The room could not be saved. Please try again."));
        return;
      } finally {
        isSendingRef.current = false;
        setSending(false);
      }
    }
    setRooms((currentRooms) =>
      currentRooms.some((room) => room.id === newRoom.id)
        ? currentRooms
        : [...currentRooms, newRoom],
    );
    activeRoomIdRef.current = newRoom.id;
    setActiveRoomId(newRoom.id);
    setRecipientIds(["codex"]);
    setParticipantMenuOpen(false);
  }

  function toggleRecipient(participantId: string) {
    if (
      roomConductor.sendMode === "conductor" ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return;
    }
    setRecipientIds((currentIds) =>
      {
        const nextIds = currentIds.includes(participantId)
        ? currentIds.filter((id) => id !== participantId)
          : [...currentIds, participantId];
        directRecipientIdsRef.current[activeRoom.id] = nextIds;
        return nextIds;
      },
    );
  }

  async function addParticipant(participantId: string) {
    if (
      isSendingRef.current ||
      activeTurnRef.current?.roomId === activeRoom.id ||
      activeRoom.participantIds.includes(participantId)
    ) {
      return;
    }
    let nextParticipantIds = [...activeRoom.participantIds, participantId];
    if ("__TAURI_INTERNALS__" in window) {
      if (roomSourceMode !== "backend") {
        setSendError(text("Rust Roomに接続できないため、参加AIを追加できません。", "AI cannot be added while Rust Room is unavailable."));
        return;
      }
      isSendingRef.current = true;
      setSending(true);
      setSendError(null);
      setSendNotice(null);
      try {
        nextParticipantIds = await addDesktopRoomParticipant({
          roomId: activeRoom.id,
          participantId,
          currentParticipantIds: activeRoom.participantIds,
        });
      } catch {
        setSendError(text("参加AIを保存できませんでした。もう一度お試しください。", "The participating AI could not be saved. Please try again."));
        return;
      } finally {
        isSendingRef.current = false;
        setSending(false);
      }
    }
    setRooms((currentRooms) =>
      currentRooms.map((room) =>
        room.id === activeRoom.id
          ? {
              ...room,
              participantIds: nextParticipantIds,
            }
          : room,
      ),
    );
    setRecipientIds((currentIds) => [...currentIds, participantId]);
    setParticipantMenuOpen(false);
  }

  async function renameRoom(name: string) {
    const nextName = name.trim();
    if (
      !nextName ||
      isSendingRef.current ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    if (nextName === activeRoom.name) {
      return true;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    try {
      if ("__TAURI_INTERNALS__" in window) {
        if (roomSourceMode !== "backend") {
          throw new Error("Room backend unavailable");
        }
        await renameDesktopRoom({
          roomId: activeRoom.id,
          name: nextName,
          currentParticipantIds: activeRoom.participantIds,
        });
      }
      setRooms((currentRooms) =>
        currentRooms.map((room) =>
          room.id === activeRoom.id ? { ...room, name: nextName } : room,
        ),
      );
      return true;
    } catch {
      setRoomMutationError(text("ルーム名を保存できませんでした。もう一度お試しください。", "The room name could not be saved. Please try again."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function removeParticipant(participantId: string) {
    const aiIds = activeRoom.participantIds.filter(
      (id) => participants[id]?.kind === "ai",
    );
    const isReferenced = activeRoom.messages.some(
      (message) =>
        message.authorId === participantId || message.targetIds.includes(participantId),
    );
    if (
      participants[participantId]?.kind === "human" ||
      !activeRoom.participantIds.includes(participantId) ||
      aiIds.length <= 1 ||
      isReferenced ||
      isSendingRef.current ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    try {
      let nextParticipantIds = activeRoom.participantIds.filter(
        (id) => id !== participantId,
      );
      if ("__TAURI_INTERNALS__" in window) {
        if (roomSourceMode !== "backend") {
          throw new Error("Room backend unavailable");
        }
        nextParticipantIds = await removeDesktopRoomParticipant({
          roomId: activeRoom.id,
          participantId,
          currentParticipantIds: activeRoom.participantIds,
        });
      }
      setRooms((currentRooms) =>
        currentRooms.map((room) =>
          room.id === activeRoom.id
            ? { ...room, participantIds: nextParticipantIds }
            : room,
        ),
      );
      setRecipientIds((currentIds) =>
        currentIds.filter((id) => id !== participantId),
      );
      return true;
    } catch {
      setRoomMutationError(text("参加AIを外せませんでした。履歴と接続状態をご確認ください。", "The AI could not be removed. Check the history and connection status."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function deleteRoom(roomId = activeRoom.id) {
    const roomToDelete = rooms.find((room) => room.id === roomId);
    if (
      !roomToDelete ||
      isBundledRoom(roomToDelete.id) ||
      isSendingRef.current ||
      activeTurnRef.current?.roomId === roomToDelete.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    try {
      if ("__TAURI_INTERNALS__" in window) {
        if (roomSourceMode !== "backend") {
          throw new Error("Room backend unavailable");
        }
        await deactivateDesktopCommandRoomSession(roomToDelete.id);
        await deleteDesktopRoom({ roomId: roomToDelete.id, name: roomToDelete.name });
      }
      const remainingRooms = rooms.filter((room) => room.id !== roomToDelete.id);
      setRooms(remainingRooms);
      if (activeRoom.id === roomToDelete.id) {
        const nextRoom = remainingRooms[0];
        if (nextRoom) {
          const firstAi = nextRoom.participantIds.find(
            (id) => participants[id]?.kind === "ai",
          );
          activeRoomIdRef.current = nextRoom.id;
          setActiveRoomId(nextRoom.id);
          setRecipientIds(firstAi ? [firstAi] : []);
        }
      }
      setParticipantMenuOpen(false);
      return true;
    } catch {
      if (roomSourceMode === "backend" && activeRoom.id === roomToDelete.id) {
        void activateDesktopCommandRoomSession(roomToDelete.id).catch(() => {
          // A failed reactivation leaves command execution unavailable rather than widening access.
        });
      }
      setRoomMutationError(text("ルームを削除できませんでした。もう一度お試しください。", "The room could not be deleted. Please try again."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function chooseBackupDirectory() {
    if (isSendingRef.current || roomSourceMode !== "backend") return false;
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    setRoomRestorePreview(null);
    try {
      const status = await chooseDesktopRoomBackupDirectory();
      setRoomBackupStatus(status);
      setRoomDataMessage(text(
        "Windowsの選択画面でバックアップ保存先を選んでください。",
        "Choose the backup directory in the Windows dialog.",
      ));
      return true;
    } catch {
      setRoomMutationError(text(
        "バックアップ保存先を選択できませんでした。",
        "The backup directory could not be selected.",
      ));
      return false;
    } finally {
      // The backend returns as soon as the native modal dialog opens. Do not
      // keep the whole app locked while waiting for its later result event.
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function useDefaultBackupDirectory() {
    if (isSendingRef.current || roomSourceMode !== "backend") return false;
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    setRoomRestorePreview(null);
    try {
      const status = await useDefaultDesktopRoomBackupDirectory();
      setRoomBackupStatus(status);
      setRoomDataMessage(text(
        "既定のバックアップ保存先へ戻しました。既存ファイルは移動していません。",
        "The default backup directory is selected. Existing files were not moved.",
      ));
      return true;
    } catch {
      setRoomMutationError(text(
        "既定のバックアップ保存先へ戻せませんでした。",
        "The default backup directory could not be selected.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function openBackupDirectory() {
    if (isSendingRef.current || roomSourceMode !== "backend") return false;
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    try {
      const status = await openDesktopRoomBackupDirectory();
      setRoomBackupStatus(status);
      return true;
    } catch {
      setRoomMutationError(text(
        "バックアップ保存先を開けませんでした。フォルダーが存在するかご確認ください。",
        "The backup directory could not be opened. Check that the folder exists.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function backupRooms() {
    if (
      !("__TAURI_INTERNALS__" in window) ||
      roomSourceMode !== "backend" ||
      isSendingRef.current
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    setRoomRestorePreview(null);
    try {
      const result = await backupDesktopRooms();
      const status = await readDesktopRoomBackupStatus();
      setRoomBackupStatus(status);
      setRoomDataMessage(
        text(`${result.roomCount}室をバックアップしました：${result.fileName}`, `Backed up ${result.roomCount} rooms: ${result.fileName}`),
      );
      return true;
    } catch {
      setRoomMutationError(text(
        "バックアップを作成できませんでした。表示中の保存先をご確認ください。別の場所へは保存していません。",
        "The backup could not be created. Check the displayed directory. Nothing was saved elsewhere.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function previewLatestBackup() {
    if (
      !("__TAURI_INTERNALS__" in window) ||
      roomSourceMode !== "backend" ||
      isSendingRef.current
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      const preview = await previewLatestDesktopRoomBackup();
      setRoomRestorePreview(preview);
      return true;
    } catch {
      setRoomRestorePreview(null);
      setRoomMutationError(text(
        "復元できる正常なバックアップを確認できませんでした。Roomデータは変更していません。",
        "A valid backup could not be verified. Room data was not changed.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function restorePreviewedBackup() {
    if (
      !("__TAURI_INTERNALS__" in window) ||
      roomSourceMode !== "backend" ||
      isSendingRef.current ||
      activeTurnRef.current !== null ||
      !roomRestorePreview
    ) {
      return false;
    }
    const confirmedPreview = roomRestorePreview;
    const previousRoomId = activeRoom.id;
    let restoreCompleted = false;
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      await Promise.all(rooms.map((room) => deactivateDesktopCommandRoomSession(room.id)));
      const result = await restoreDesktopRoomBackup(confirmedPreview.fileName);
      restoreCompleted = true;
      const hydration = await readDesktopRooms();
      const nextRoom = resolveSelectedRoom(hydration.rooms, previousRoomId);
      const firstAi = nextRoom?.participantIds.find(
        (id) => hydration.participants[id]?.kind === "ai",
      );
      setCanonicalParticipants((current) => ({ ...current, ...hydration.participants }));
      setRooms(hydration.rooms);
      if (nextRoom) {
        activeRoomIdRef.current = nextRoom.id;
        setActiveRoomId(nextRoom.id);
        setRecipientIds(firstAi ? [firstAi] : []);
        try {
          await activateDesktopCommandRoomSession(nextRoom.id);
        } catch {
          setRoomMutationError(text(
            "復元は完了しましたが、Codex作業モードを再開できませんでした。作業フォルダーを選び直してください。",
            "The restore completed, but Codex workspace mode could not be restarted. Choose the workspace folder again.",
          ));
        }
      }
      setParticipantMenuOpen(false);
      setRoomRestorePreview(null);
      setRoomDataMessage(
        text(`${result.fileName} から${result.roomCount}室を復元しました。`, `Restored ${result.roomCount} rooms from ${result.fileName}.`),
      );
      return true;
    } catch {
      setRoomRestorePreview(null);
      if (!restoreCompleted) {
        try {
          await activateDesktopCommandRoomSession(previousRoomId);
        } catch {
          // Restoration did not begin, but command execution remains fail closed
          // if the previous Room session cannot be recreated safely.
        }
      }
      setRoomMutationError(
        restoreCompleted
          ? text(
              "バックアップの復元は完了しましたが、画面へ再読み込みできませんでした。M.I.O.を再起動してください。",
              "The backup was restored, but the screen could not be refreshed. Restart M.I.O.",
            )
          : text(
              "確認したバックアップを復元できませんでした。新しいバックアップが増えた場合は、もう一度内容をご確認ください。",
              "The confirmed backup could not be restored. If a newer backup appeared, review the backup again.",
            ),
      );
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function chooseWorkspace() {
    if (
      isSendingRef.current ||
      roomSourceMode !== "backend" ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      const result = await chooseDesktopRoomWorkspace(activeRoom.id);
      setRoomWorkspace(result.status);
      setRoomDataMessage(text("Windowsの選択画面で作業フォルダーを選んでください。", "Choose a workspace folder in the Windows dialog."));
      return true;
    } catch {
      setRoomMutationError(text("作業フォルダーを設定できませんでした。", "The workspace folder could not be set."));
      return false;
    } finally {
      // The native dialog is modal. Its result arrives through the listener,
      // so the launch guard must never depend on that event to be released.
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function clearWorkspace() {
    if (
      isSendingRef.current ||
      roomSourceMode !== "backend" ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      const status = await clearDesktopRoomWorkspace(activeRoom.id);
      setRoomWorkspace(status);
      await activateDesktopCommandRoomSession(activeRoom.id);
      setRoomDataMessage(text("Codexを会話のみに戻しました。", "Codex has returned to chat-only mode."));
      return true;
    } catch {
      setRoomMutationError(text("Codex作業モードを解除できませんでした。", "Codex workspace mode could not be disabled."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function configureRoomConductor(conductorId: string | null) {
    if (
      isSendingRef.current ||
      roomSourceMode !== "backend" ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    try {
      const status = conductorId
        ? await setDesktopRoomConductor(activeRoom.id, conductorId)
        : await clearDesktopRoomConductor(activeRoom.id);
      if (status.sendMode === "conductor" && status.conductorId) {
        directRecipientIdsRef.current[activeRoom.id] = recipientIds;
        setRecipientIds([status.conductorId]);
      } else {
        const fallback = activeRoom.participantIds.find(
          (participantId) => participants[participantId]?.kind === "ai",
        );
        const directRecipients = directRecipientIdsRef.current[activeRoom.id]
          ?.filter((participantId) => activeRoom.participantIds.includes(participantId));
        setRecipientIds(directRecipients?.length ? directRecipients : fallback ? [fallback] : []);
      }
      setRoomConductor(status);
      return true;
    } catch {
      setRoomMutationError(text(
        "指揮者設定を保存できませんでした。",
        "The conductor setting could not be saved.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function changeConductorSendMode(sendMode: "direct" | "conductor") {
    if (
      isSendingRef.current ||
      activeTurnRef.current?.roomId === activeRoom.id ||
      roomSourceMode !== "backend" ||
      !roomConfigurationReady ||
      !roomConductor.conductorId ||
      roomConductor.sendMode === sendMode
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setSendError(null);
    try {
      const status = await saveDesktopRoomConductorMode(activeRoom.id, sendMode);
      if (sendMode === "conductor") {
        directRecipientIdsRef.current[activeRoom.id] = recipientIds;
        setRecipientIds([status.conductorId!]);
      } else {
        const fallback = activeRoom.participantIds.find(
          (participantId) => participants[participantId]?.kind === "ai",
        );
        const directRecipients = directRecipientIdsRef.current[activeRoom.id]
          ?.filter((participantId) => activeRoom.participantIds.includes(participantId));
        setRecipientIds(directRecipients?.length ? directRecipients : fallback ? [fallback] : []);
      }
      setRoomConductor(status);
      return true;
    } catch {
      setSendError(text(
        "送信モードを変更できませんでした。",
        "The send mode could not be changed.",
      ));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function sendMessage(body: string, imageGeneration: ImageGenerationPreferences | null) {
    if (
      selectedRecipients.length === 0 ||
      isSendingRef.current ||
      activeTurnRef.current !== null
    ) {
      return false;
    }

    const roomId = activeRoom.id;
    setSendFeedbackRoomId(roomId);
    const primaryRecipient = selectedRecipients[0];
    const targetIds = selectedRecipients.map((participant) => participant.id);
    const targetsBackendRoom = "__TAURI_INTERNALS__" in window;
    if (targetsBackendRoom && roomSourceMode !== "backend") {
      setSendError(text("Rust Roomが利用できないため送信できません。接続状態を確認してください。", "The message cannot be sent while Rust Room is unavailable."));
      return false;
    }
    if (targetsBackendRoom && !roomConfigurationReady) {
      setSendError(text(
        "ルーム設定を確認中です。確認が終わってから送信してください。",
        "Room settings are still being checked. Send after the check finishes.",
      ));
      return false;
    }
    if (targetsBackendRoom) {
      const pending = pendingWrite.current;
      const samePendingWrite =
        pending?.roomId === roomId &&
        pending.body === body &&
        pending.recipientIds.length === targetIds.length &&
        pending.recipientIds.every((id, index) => id === targetIds[index]);
      const write = samePendingWrite
        ? pending
        : {
            roomId,
            messageId: createId("message"),
            recipientIds: targetIds,
            body,
          };
      pendingWrite.current = write;
      stopRequestedRef.current = false;
      isCancellingRef.current = false;
      setCancelling(false);
      isSendingRef.current = true;
      setSending(true);
      setSendError(null);
      setSendNotice(null);
      try {
        const savedMessage = await writeDesktopRoomMessage({
          ...write,
          participants,
        });
        setRooms((currentRooms) =>
          currentRooms.map((room) =>
            room.id === roomId
              ? {
                  ...room,
                  messages: room.messages.some((message) => message.id === savedMessage.id)
                    ? room.messages
                    : [...room.messages, savedMessage],
                  updatedLabel: "いま",
                }
              : room,
          ),
        );
        pendingWrite.current = null;
        activeTurnRef.current = { roomId, messageId: write.messageId };
        setActiveTurnRoomId(roomId);
        const codexWillRun =
          roomConductor.roomId === roomId &&
          roomConductor.sendMode === "conductor" &&
          roomConductor.conductorId
            ? roomConductor.conductorId === "codex"
            : targetIds.includes("codex");
        setCodexTurnProgress(codexWillRun ? {
          roomId,
          dispatchId: "",
          phase: "preparing",
          startedAt: Date.now(),
        } : null);
        if (
          roomConductor.roomId === roomId &&
          roomConductor.sendMode === "conductor" &&
          roomConductor.conductorId
        ) {
          setTypingParticipantId(roomConductor.conductorId);
          void orchestrateDesktopRoomMessage({
            roomId,
            messageId: write.messageId,
            participants,
          })
            .then((orchestration) => {
              if (orchestration.message) {
                void readDesktopAiConnectionStatuses()
                  .then(setAiConnections)
                  .catch(() => {
                    // Keep the last known state when a post-response refresh fails.
                  });
                setRooms((currentRooms) =>
                  currentRooms.map((room) =>
                    room.id === roomId
                      ? {
                          ...room,
                          messages: room.messages.some(
                            (message) => message.id === orchestration.message!.id,
                          )
                            ? room.messages
                            : [...room.messages, orchestration.message!],
                          updatedLabel: "いま",
                        }
                      : room,
                  ),
                );
              } else if (orchestration.status === "unknown") {
                if (stopRequestedRef.current) {
                  setSendNotice(text(
                    "AI処理を停止しました。依頼はすでにAIへ届いている可能性があります。自動再送していません。",
                    "The AI turn was stopped. The request may already have reached the AI and was not retried.",
                  ));
                } else {
                  setSendError(text(
                    "指揮処理の結果を確認できませんでした。二重実行防止のため自動再送していません。",
                    "The orchestration result is unknown. It was not retried to prevent duplicate work.",
                  ));
                }
              } else {
                setSendError(text(
                  "指揮者がこの依頼を完了できませんでした。",
                  "The conductor could not complete this request.",
                ));
              }
            })
            .catch(() => {
              if (stopRequestedRef.current) {
                setSendNotice(text(
                  "AI処理を停止しました。依頼はすでにAIへ届いている可能性があります。自動再送していません。",
                  "The AI turn was stopped. The request may already have reached the AI and was not retried.",
                ));
              } else {
                setSendError(text(
                  "指揮処理を確認できませんでした。二重実行防止のため自動再送していません。",
                  "Room orchestration could not be confirmed and was not retried.",
                ));
              }
            })
            .finally(() => {
              activeTurnRef.current = null;
              setActiveTurnRoomId(null);
              if (activeRoomIdRef.current !== roomId) {
                void deactivateDesktopCommandRoomSession(roomId).catch(() => {
                  // A failed cleanup leaves later command execution fail closed.
                });
              }
              setCodexTurnProgress(null);
              isCancellingRef.current = false;
              setCancelling(false);
              setTypingParticipantId(null);
              setDispatchSafetyRevision((revision) => revision + 1);
            });
          return true;
        }
        const pendingRecipientIds = new Set(targetIds);
        let singleGeminiQueued = false;
        const nextNativeTypingId = () =>
          [...pendingRecipientIds].find((id) =>
            id === "codex" || id === "grok" || id === "claude-code" || id === "gemini"
          ) ?? null;
        const appendSendError = (message: string) => {
          setSendError((current) => current ? `${current} ${message}` : message);
        };
        const appendSendNotice = (message: string) => {
          setSendNotice((current) => current ? `${current} ${message}` : message);
        };
        setTypingParticipantId(nextNativeTypingId());
        const dispatches = targetIds.map((participantId) =>
          dispatchDesktopRoomRecipient({
            imageGeneration: participantId === "codex" ? imageGeneration : null,
            roomId,
            messageId: write.messageId,
            participantId,
            participants,
          })
          .then((dispatch) => {
            if (dispatch.messages.length > 0) {
              void readDesktopAiConnectionStatuses()
                .then(setAiConnections)
                .catch(() => {
                  // Keep the last known state when a post-response refresh fails.
                });
              setRooms((currentRooms) =>
                currentRooms.map((room) =>
                  room.id === roomId
                    ? {
                        ...room,
                        messages: [
                          ...room.messages,
                          ...dispatch.messages.filter(
                            (message) =>
                              !room.messages.some((existing) => existing.id === message.id),
                          ),
                        ],
                        updatedLabel: "いま",
                      }
                    : room,
                ),
              );
            }
            if (dispatch.unsupportedRecipientIds.length > 0) {
              const names = dispatch.unsupportedRecipientIds.map(
                (id) => participants[id]?.displayName ?? id,
              );
              appendSendError(text(`${names.join("、")}はまだ実接続されていません。保存は完了しています。`, `${names.join(", ")} are not connected yet. The message was saved.`));
            }
            if (targetIds.length === 1 && dispatch.queuedRecipientIds.includes("gemini")) {
              singleGeminiQueued = true;
            }
            if (dispatch.failedRecipients.length > 0) {
              const workspaceSandboxFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "codexWorkspaceSandboxUnavailable",
              );
              const workspaceUnavailableFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "roomWorkspaceUnavailable",
              );
              const workspaceUnsafeLinkFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "roomWorkspaceUnsafeLink",
              );
              const codexUpdateRequiredFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "codexClientUpdateRequired",
              );
              const codexConfirmedFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "codexTurnFailed",
              );
              const codexPreflightFailures = dispatch.failedRecipients.filter(
                ({ code }) => code === "codexPreflightFailed",
              );
              const otherFailures = dispatch.failedRecipients.filter(
                ({ code }) =>
                  code !== "codexWorkspaceSandboxUnavailable" &&
                  code !== "roomWorkspaceUnavailable" &&
                  code !== "roomWorkspaceUnsafeLink" &&
                  code !== "codexClientUpdateRequired" &&
                  code !== "codexPreflightFailed" &&
                  code !== "codexTurnFailed",
              );
              if (workspaceSandboxFailures.length > 0) {
                appendSendError(
                  text(
                    "CodexのWindows保護機能を安全な状態で開始できませんでした。会話のみに戻すか、Codexの設定を確認してください。メッセージは保存済みで、自動再送していません。",
                    "Codex Windows protection could not start in the required safe mode. Return to chat-only mode or check the Codex configuration. The message was saved and was not retried.",
                  ),
                );
              }
              if (workspaceUnavailableFailures.length > 0) {
                appendSendError(
                  text(
                    "選択したCodex作業フォルダーが見つからないか、開くことができません。フォルダーを選び直すか、会話のみに戻してください。メッセージは保存済みで、Codexは起動せず、自動再送していません。",
                    "The selected Codex workspace is missing or cannot be opened. Choose the folder again or return to chat-only mode. The message was saved, Codex was not started, and nothing was retried.",
                  ),
                );
              }
              if (workspaceUnsafeLinkFailures.length > 0) {
                appendSendError(
                  text(
                    "選択したCodex作業フォルダーがジャンクションまたはシンボリックリンクに変わったため、安全のため開きませんでした。フォルダーを選び直すか、会話のみに戻してください。メッセージは保存済みで、Codexは起動せず、自動再送していません。",
                    "The selected Codex workspace became a junction or symbolic link, so it was not opened. Choose the folder again or return to chat-only mode. The message was saved, Codex was not started, and nothing was retried.",
                  ),
                );
              }
              if (codexUpdateRequiredFailures.length > 0) {
                appendSendError(
                  text(
                    "選択したモデルは、現在のCodexでは利用できません。Codexを最新版へ更新するか、参加者プロフィールで別のモデルを選んでください。今回のAI処理は失敗が確認されており、自動再送していません。",
                    "The selected model is not available with the current Codex version. Update Codex or choose another model in the participant profile. This AI turn is confirmed failed and was not retried.",
                  ),
                );
              }
              if (codexConfirmedFailures.length > 0) {
                appendSendError(
                  text(
                    "Codexが今回の処理を失敗終了したことを確認しました。Codexのログイン状態・モデル選択・設定を確認してから、新しいメッセージとして送信してください。自動再送はしていません。",
                    "Codex confirmed that this turn failed. Check the Codex login, selected model, and settings before sending a new message. It was not retried automatically.",
                  ),
                );
              }
              if (codexPreflightFailures.length > 0) {
                appendSendError(
                  text(
                    "Codexを開始できなかったため、メッセージはCodexへ届いていません。Codexの起動・ログイン状態・モデル選択・設定を確認し、同じ内容を新しいメッセージとして再送できます。自動再送はしていません。",
                    "Codex could not start, so the message was not delivered to Codex. Check the Codex installation, login, selected model, and settings, then send the same content as a new message. It was not retried automatically.",
                  ),
                );
              }
              const names = otherFailures.map(
                ({ recipientId }) => participants[recipientId]?.displayName ?? recipientId,
              );
              if (names.length > 0) {
                appendSendError(
                  text(
                    `${names.join("、")}の応答を取得できませんでした。メッセージは保存済みで、自動再送していません。`,
                    `${names.join(", ")} did not return a response. The message was saved and was not retried.`,
                  ),
                );
              }
            }
            if (dispatch.unknownRecipients.length > 0) {
              const cancelled = dispatch.unknownRecipients.filter(
                ({ code }) => code === "aiDispatchCancelled",
              );
              if (cancelled.length > 0) {
                appendSendNotice(text(
                  "AI処理を停止しました。依頼はすでにAIへ届いている可能性があります。自動再送していません。",
                  "The AI turn was stopped. The request may already have reached the AI and was not retried.",
                ));
              }
              const names = dispatch.unknownRecipients
                .filter(({ code }) => code !== "aiDispatchCancelled")
                .map(
                ({ recipientId }) => participants[recipientId]?.displayName ?? recipientId,
              );
              if (names.length > 0) {
                appendSendError(
                  text(
                    `${names.join("、")}にはメッセージが届いた可能性があります。二重送信を防ぐため、自動再送していません。`,
                    `The message may have reached ${names.join(", ")}. It was not retried to prevent a duplicate turn.`,
                  ),
                );
              }
            }
            const contextNotices = dispatch.contextReports.flatMap((report) => {
              const name = participants[report.participantId]?.displayName ?? report.participantId;
              const details: string[] = [];
              if (report.mode === "reconstructed") {
                details.push(
                  text(
                    "Room履歴から会話を再構築しました",
                    "reconstructed the conversation from Room history",
                  ),
                );
              }
              if (report.omittedMessages > 0 || report.truncatedMessages > 0) {
                details.push(
                  text(
                    `古い履歴${report.omittedMessages}件・長文${report.truncatedMessages}件（計${report.omittedCharacters}文字）を省略しました`,
                    `omitted ${report.omittedMessages} older messages and shortened ${report.truncatedMessages} long messages by ${report.omittedCharacters} characters`,
                  ),
                );
              }
              if (!report.continuitySaved) {
                details.push(
                  text(
                    "返信は保存済みですが継続状態を保存できず、次回はRoom履歴から再開します",
                    "saved the reply but could not save continuity; the next turn will resume from Room history",
                  ),
                );
              }
              return details.length === 0 ? [] : [`${name}: ${details.join("。")}。`];
            });
            if (contextNotices.length > 0) {
              appendSendNotice(contextNotices.join(" "));
            }
          })
          .catch(() => {
            const name = participants[participantId]?.displayName ?? participantId;
            appendSendError(text(`${name}へのメッセージは保存済みですが、AI応答の処理に失敗しました。二重turn防止のため自動再送していません。`, `The message to ${name} was saved, but AI response processing failed. It was not retried to prevent a duplicate turn.`));
          })
          .finally(() => {
            pendingRecipientIds.delete(participantId);
            if (participantId === "codex") {
              setCodexTurnProgress(null);
            }
            setTypingParticipantId(nextNativeTypingId() ?? (singleGeminiQueued ? "gemini" : null));
          }),
        );
        void Promise.allSettled(dispatches)
          .finally(() => {
            activeTurnRef.current = null;
            setActiveTurnRoomId(null);
            if (activeRoomIdRef.current !== roomId) {
              void deactivateDesktopCommandRoomSession(roomId).catch(() => {
                // A failed cleanup leaves later command execution fail closed.
              });
            }
            setCodexTurnProgress(null);
            isCancellingRef.current = false;
            setCancelling(false);
            setDispatchSafetyRevision((revision) => revision + 1);
          });
        return true;
      } catch {
        setSendError(text("Rust Roomに保存できませんでした。本文を残したので、もう一度送信できます。", "The message could not be saved to Rust Room. The draft was kept so you can retry."));
        return false;
      } finally {
        isSendingRef.current = false;
        setSending(false);
      }
    }

    const userMessage: ChatMessage = {
      id: createId("message"),
      authorId: ownerParticipantId,
      body,
      targetIds,
      sentAt: "いま",
    };

    setRooms((currentRooms) =>
      currentRooms.map((room) =>
        room.id === roomId
          ? {
              ...room,
              messages: [...room.messages, userMessage],
              updatedLabel: "いま",
            }
          : room,
      ),
    );
    setTypingParticipantId(primaryRecipient.id);

    const timer = window.setTimeout(() => {
      const demoReply: ChatMessage = {
        id: createId("message"),
        authorId: primaryRecipient.id,
        body: text(`了解です。これは ${primaryRecipient.displayName} の接続前ダミー応答です。送信から返答までのUIの流れを確認できました。`, `Understood. This is a pre-connection demo reply from ${primaryRecipient.displayName}. The send-and-reply UI flow is working.`),
        targetIds: [ownerParticipantId],
        sentAt: "いま",
        isDemo: true,
      };

      setRooms((currentRooms) =>
        currentRooms.map((room) =>
          room.id === roomId
            ? { ...room, messages: [...room.messages, demoReply] }
            : room,
        ),
      );
      setTypingParticipantId((currentId) =>
        currentId === primaryRecipient.id ? null : currentId,
      );
      replyTimers.current.delete(timer);
    }, 650);

    replyTimers.current.add(timer);
    return true;
  }

  async function cancelActiveTurn() {
    const activeTurn = activeTurnRef.current;
    if (
      !activeTurn ||
      activeTurn.roomId !== activeRoom.id ||
      isCancellingRef.current
    ) {
      return false;
    }
    isCancellingRef.current = true;
    stopRequestedRef.current = true;
    setCancelling(true);
    setSendError(null);
    try {
      const accepted = await cancelDesktopRoomTurn(activeTurn.roomId, activeTurn.messageId);
      if (!accepted) {
        isCancellingRef.current = false;
        stopRequestedRef.current = false;
        setCancelling(false);
        setSendError(text(
          "停止対象を確認できませんでした。処理結果を待ち、自動再送はしません。",
          "The active turn could not be found. Wait for its result; it will not be retried automatically.",
        ));
        return false;
      }
      setSendNotice(text(
        "停止しています…",
        "Stopping…",
      ));
      return true;
    } catch {
      isCancellingRef.current = false;
      stopRequestedRef.current = false;
      setCancelling(false);
      setSendError(text(
        "停止要求を送れませんでした。処理結果を待ち、自動再送はしません。",
        "The stop request could not be sent. Wait for the result; it will not be retried automatically.",
      ));
      return false;
    }
  }

  async function saveParticipantProfile(profile: ParticipantProfile) {
    setRoomMutationError(null);
    try {
      const saved = await persistParticipantProfile(profile);
      setParticipantProfiles((current) => ({
        ...current,
        [saved.participantId]: saved,
      }));
      return true;
    } catch {
      setRoomMutationError(text(
        "参加者プロフィールを保存できませんでした。画像サイズと表示名をご確認ください。",
        "The participant profile could not be saved. Check the image size and display name.",
      ));
      return false;
    }
  }

  async function resetAiContinuity(participantId: string) {
    if (
      isSendingRef.current ||
      roomSourceMode !== "backend" ||
      activeTurnRef.current?.roomId === activeRoom.id
    ) {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      const changed = await resetDesktopRoomAiContinuity(activeRoom.id, participantId);
      const name = participants[participantId]?.displayName ?? participantId;
      setRoomDataMessage(
        changed
          ? text(
              `${name}の継続状態をリセットしました。会話履歴は残り、次回はRoom履歴から新しく再開します。`,
              `Reset ${name}'s continuity. Room history remains, and the next turn will start again from Room history.`,
            )
          : text(
              `${name}には保存済みの継続状態がありません。会話履歴は変更していません。`,
              `${name} had no saved continuity. Room history was not changed.`,
            ),
      );
      return true;
    } catch {
      setRoomMutationError(
        text(
          "AIの継続状態をリセットできませんでした。会話履歴は変更していません。",
          "AI continuity could not be reset. Room history was not changed.",
        ),
      );
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  const visibleRoomWorkspace = workspaceStatusReady
    ? roomWorkspace
    : { roomId: activeRoom.id, mode: "chatOnly" as const, folderName: null, available: false };
  const visibleRoomConductor = conductorStatusReady
    ? roomConductor
    : { roomId: activeRoom.id, conductorId: null, sendMode: "direct" as const };

  return {
    activeRoom,
    activeTurnRoomId,
    aiConnections,
    codexTurnProgress,
    addParticipant,
    availableParticipants,
    backupRooms,
    chooseBackupDirectory,
    chooseWorkspace,
    changeConductorSendMode,
    clearWorkspace,
    closeParticipantMenu: () => setParticipantMenuOpen(false),
    configureRoomConductor,
    createRoom,
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
    roomConductor: visibleRoomConductor,
    roomRestorePreview,
    roomWorkspace: visibleRoomWorkspace,
    roomDataMessage,
    sendError: sendFeedbackRoomId === activeRoom.id ? sendError : null,
    sendNotice: sendFeedbackRoomId === activeRoom.id ? sendNotice : null,
    selectedRecipients,
    removeParticipant,
    resetAiContinuity,
    renameRoom,
    previewLatestBackup,
    restorePreviewedBackup,
    saveParticipantProfile,
    selectRoom,
    sendMessage,
    cancelActiveTurn,
    toggleParticipantMenu: () => setParticipantMenuOpen((isOpen) => !isOpen),
    clearRoomMutationError: () => setRoomMutationError(null),
    toggleRecipient,
    typingParticipantId,
    useDefaultBackupDirectory,
  };
}
