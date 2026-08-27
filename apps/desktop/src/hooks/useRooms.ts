import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import {
  demoParticipants,
  initialRecipientIds,
  initialRooms,
  ownerParticipantId,
} from "../mockData";
import {
  addDesktopRoomParticipant,
  backupDesktopRooms,
  browserBridgeReplyView,
  chooseDesktopRoomWorkspace,
  clearDesktopRoomWorkspace,
  clearDesktopRoomConductor,
  createDesktopRoom,
  deleteDesktopRoom,
  dispatchDesktopRoomRecipient,
  orchestrateDesktopRoomMessage,
  readDesktopAiConnectionStatuses,
  readDesktopRoomConductorStatus,
  readDesktopRoomDispatchUnknowns,
  readDesktopRoom,
  readDesktopRoomWorkspaceStatus,
  readDesktopRooms,
  resetDesktopRoomAiContinuity,
  removeDesktopRoomParticipant,
  renameDesktopRoom,
  restoreLatestDesktopRoomBackup,
  saveDesktopRoomConductorMode,
  setDesktopRoomConductor,
  writeDesktopRoomMessage,
} from "../roomBridge";
import {
  readParticipantProfiles,
  saveParticipantProfile as persistParticipantProfile,
} from "../participantProfileBridge";
import type {
  AiConnectionMap,
  ChatMessage,
  ParticipantMap,
  ParticipantProfile,
  Room,
  RoomConductorStatus,
  RoomWorkspaceStatus,
} from "../types";
import { useUiPreferences } from "../uiPreferences";

export type RoomSourceMode = "loading" | "backend" | "browserDemo" | "error";

const dismissedDispatchUnknownsStorageKey = "moe-dismissed-dispatch-unknowns-v1";
const maximumDismissedDispatchUnknowns = 512;

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
  const [activeRoomId, setActiveRoomId] = useState(initialRooms[0].id);
  const [recipientIds, setRecipientIds] = useState(initialRecipientIds);
  const [isParticipantMenuOpen, setParticipantMenuOpen] = useState(false);
  const [typingParticipantId, setTypingParticipantId] = useState<string | null>(null);
  const [roomSourceMode, setRoomSourceMode] = useState<RoomSourceMode>("loading");
  const [isSending, setSending] = useState(false);
  const [isAwaitingReply, setAwaitingReply] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  const [sendNotice, setSendNotice] = useState<string | null>(null);
  const [dispatchSafetyWarning, setDispatchSafetyWarning] = useState<string | null>(null);
  const [dispatchSafetyWarningKeys, setDispatchSafetyWarningKeys] = useState<string[]>([]);
  const [dismissedDispatchUnknownKeys, setDismissedDispatchUnknownKeys] = useState(
    loadDismissedDispatchUnknowns,
  );
  const [dispatchSafetyRevision, setDispatchSafetyRevision] = useState(0);
  const [roomMutationError, setRoomMutationError] = useState<string | null>(null);
  const [roomDataMessage, setRoomDataMessage] = useState<string | null>(null);
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
  const isSendingRef = useRef(false);
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
    if (!("__TAURI_INTERNALS__" in window)) {
      setRoomSourceMode("browserDemo");
      return;
    }

    let cancelled = false;
    void readDesktopRooms()
      .then((hydration) => {
        if (cancelled) {
          return;
        }
        setCanonicalParticipants((current) => ({ ...current, ...hydration.participants }));
        setRooms(hydration.rooms);
        setActiveRoomId((current) =>
          hydration.rooms.some((room) => room.id === current)
            ? current
            : hydration.rooms[0]?.id ?? current,
        );
        setRoomSourceMode("backend");
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
        setTypingParticipantId((current) => current === "gemini" ? null : current);
        setSendError(null);
      } catch {
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
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      setRoomWorkspace({ roomId, mode: "chatOnly", folderName: null, available: true });
      return;
    }
    let cancelled = false;
    void readDesktopRoomWorkspaceStatus(roomId)
      .then((status) => {
        if (!cancelled) {
          setRoomWorkspace(status);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRoomWorkspace({ roomId, mode: "chatOnly", folderName: null, available: false });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeRoom.id, roomSourceMode]);

  useEffect(() => {
    const roomId = activeRoom.id;
    if (!("__TAURI_INTERNALS__" in window) || roomSourceMode !== "backend") {
      setRoomConductor({ roomId, conductorId: null, sendMode: "direct" });
      return;
    }
    let cancelled = false;
    void readDesktopRoomConductorStatus(roomId)
      .then((status) => {
        if (cancelled) return;
        setRoomConductor(status);
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
    if (dispatchSafetyWarningKeys.length === 0) return;
    setDismissedDispatchUnknownKeys((current) => {
      const next = Array.from(new Set([...current, ...dispatchSafetyWarningKeys]))
        .slice(-maximumDismissedDispatchUnknowns);
      persistDismissedDispatchUnknowns(next);
      return next;
    });
    setDispatchSafetyWarning(null);
    setDispatchSafetyWarningKeys([]);
  }

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
        .then((status) => {
          if (disposed) {
            return;
          }
          setRoomWorkspace(status);
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

    const firstAi = nextRoom.participantIds.find(
      (participantId) => participants[participantId]?.kind === "ai",
    );

    setActiveRoomId(roomId);
    const nextRecipients = directRecipientIdsRef.current[roomId]?.filter((participantId) =>
      nextRoom.participantIds.includes(participantId)
    ) ?? (firstAi ? [firstAi] : []);
    directRecipientIdsRef.current[roomId] = nextRecipients;
    setRecipientIds(nextRecipients);
    setParticipantMenuOpen(false);
    setTypingParticipantId(null);
    setSendError(null);
    setSendNotice(null);
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
    setActiveRoomId(newRoom.id);
    setRecipientIds(["codex"]);
    setParticipantMenuOpen(false);
  }

  function toggleRecipient(participantId: string) {
    if (roomConductor.sendMode === "conductor") {
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
    if (isSendingRef.current || activeRoom.participantIds.includes(participantId)) {
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
    if (!nextName || isSendingRef.current) {
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
      isSendingRef.current
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

  async function deleteRoom() {
    if (
      ["moe-dev-room", "comparison-room", "mcp-lab"].includes(activeRoom.id) ||
      isSendingRef.current
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
        await deleteDesktopRoom({ roomId: activeRoom.id, name: activeRoom.name });
      }
      const remainingRooms = rooms.filter((room) => room.id !== activeRoom.id);
      const nextRoom = remainingRooms[0];
      setRooms(remainingRooms);
      if (nextRoom) {
        const firstAi = nextRoom.participantIds.find(
          (id) => participants[id]?.kind === "ai",
        );
        setActiveRoomId(nextRoom.id);
        setRecipientIds(firstAi ? [firstAi] : []);
      }
      setParticipantMenuOpen(false);
      return true;
    } catch {
      setRoomMutationError(text("ルームを削除できませんでした。もう一度お試しください。", "The room could not be deleted. Please try again."));
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
    try {
      const result = await backupDesktopRooms();
      setRoomDataMessage(
        text(`${result.roomCount}室をバックアップしました：${result.fileName}`, `Backed up ${result.roomCount} rooms: ${result.fileName}`),
      );
      return true;
    } catch {
      setRoomMutationError(text("バックアップを作成できませんでした。Documentsフォルダーをご確認ください。", "The backup could not be created. Check the Documents folder."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function restoreLatestBackup() {
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
      const result = await restoreLatestDesktopRoomBackup();
      const hydration = await readDesktopRooms();
      const nextRoom = hydration.rooms[0];
      const firstAi = nextRoom?.participantIds.find(
        (id) => hydration.participants[id]?.kind === "ai",
      );
      setCanonicalParticipants((current) => ({ ...current, ...hydration.participants }));
      setRooms(hydration.rooms);
      if (nextRoom) {
        setActiveRoomId(nextRoom.id);
        setRecipientIds(firstAi ? [firstAi] : []);
      }
      setParticipantMenuOpen(false);
      setRoomDataMessage(
        text(`${result.fileName} から${result.roomCount}室を復元しました。`, `Restored ${result.roomCount} rooms from ${result.fileName}.`),
      );
      return true;
    } catch {
      setRoomMutationError(text("復元できませんでした。先にバックアップを作成してください。", "Restore failed. Create a backup first."));
      return false;
    } finally {
      isSendingRef.current = false;
      setSending(false);
    }
  }

  async function chooseWorkspace() {
    if (isSendingRef.current || roomSourceMode !== "backend") {
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
      isSendingRef.current = false;
      setSending(false);
      setRoomMutationError(text("作業フォルダーを設定できませんでした。", "The workspace folder could not be set."));
      return false;
    }
  }

  async function clearWorkspace() {
    if (isSendingRef.current || roomSourceMode !== "backend") {
      return false;
    }
    isSendingRef.current = true;
    setSending(true);
    setRoomMutationError(null);
    setRoomDataMessage(null);
    try {
      const status = await clearDesktopRoomWorkspace(activeRoom.id);
      setRoomWorkspace(status);
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
    if (isSendingRef.current || roomSourceMode !== "backend") {
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
      roomSourceMode !== "backend" ||
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

  async function sendMessage(body: string) {
    if (selectedRecipients.length === 0 || isSendingRef.current) {
      return false;
    }

    const roomId = activeRoom.id;
    const primaryRecipient = selectedRecipients[0];
    const targetIds = selectedRecipients.map((participant) => participant.id);
    const targetsBackendRoom = "__TAURI_INTERNALS__" in window;
    if (targetsBackendRoom && roomSourceMode !== "backend") {
      setSendError(text("Rust Roomが利用できないため送信できません。接続状態を確認してください。", "The message cannot be sent while Rust Room is unavailable."));
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
        if (
          roomConductor.roomId === roomId &&
          roomConductor.sendMode === "conductor" &&
          roomConductor.conductorId
        ) {
          setTypingParticipantId(roomConductor.conductorId);
          setAwaitingReply(true);
          void orchestrateDesktopRoomMessage({
            roomId,
            messageId: write.messageId,
            participants,
          })
            .then((orchestration) => {
              if (orchestration.message) {
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
                setSendError(text(
                  "指揮処理の結果を確認できませんでした。二重実行防止のため自動再送していません。",
                  "The orchestration result is unknown. It was not retried to prevent duplicate work.",
                ));
              } else {
                setSendError(text(
                  "指揮者がこの依頼を完了できませんでした。",
                  "The conductor could not complete this request.",
                ));
              }
            })
            .catch(() => {
              setSendError(text(
                "指揮処理を確認できませんでした。二重実行防止のため自動再送していません。",
                "Room orchestration could not be confirmed and was not retried.",
              ));
            })
            .finally(() => {
              setTypingParticipantId(null);
              setAwaitingReply(false);
              setDispatchSafetyRevision((revision) => revision + 1);
            });
          return true;
        }
        const pendingRecipientIds = new Set(targetIds);
        let singleGeminiQueued = false;
        const nextNativeTypingId = () =>
          [...pendingRecipientIds].find((id) => id === "codex" || id === "grok") ?? null;
        const appendSendError = (message: string) => {
          setSendError((current) => current ? `${current} ${message}` : message);
        };
        const appendSendNotice = (message: string) => {
          setSendNotice((current) => current ? `${current} ${message}` : message);
        };
        setTypingParticipantId(nextNativeTypingId());
        setAwaitingReply(true);
        const dispatches = targetIds.map((participantId) =>
          dispatchDesktopRoomRecipient({
            roomId,
            messageId: write.messageId,
            participantId,
            participants,
          })
          .then((dispatch) => {
            if (dispatch.messages.length > 0) {
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
              const otherFailures = dispatch.failedRecipients.filter(
                ({ code }) => code !== "codexWorkspaceSandboxUnavailable",
              );
              if (workspaceSandboxFailures.length > 0) {
                appendSendError(
                  text(
                    "Codexのworkspaceアクセスは、nested junctionの読取り境界を満たさないため、このWindows alphaでは無効です。会話のみは利用できます。メッセージは保存済みで、自動再送していません。",
                    "Codex workspace access is disabled in this Windows alpha because the nested-junction read boundary is not contained. Chat-only messages remain available. The message was saved and was not retried.",
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
              const names = dispatch.unknownRecipients.map(
                ({ recipientId }) => participants[recipientId]?.displayName ?? recipientId,
              );
              appendSendError(
                text(
                  `${names.join("、")}にはメッセージが届いた可能性があります。二重送信を防ぐため、自動再送していません。`,
                  `The message may have reached ${names.join(", ")}. It was not retried to prevent a duplicate turn.`,
                ),
              );
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
            setTypingParticipantId(nextNativeTypingId() ?? (singleGeminiQueued ? "gemini" : null));
          }),
        );
        void Promise.allSettled(dispatches)
          .finally(() => {
            setAwaitingReply(false);
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
    if (isSendingRef.current || roomSourceMode !== "backend") {
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

  return {
    activeRoom,
    aiConnections,
    addParticipant,
    availableParticipants,
    backupRooms,
    chooseWorkspace,
    changeConductorSendMode,
    clearWorkspace,
    closeParticipantMenu: () => setParticipantMenuOpen(false),
    configureRoomConductor,
    createRoom,
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
    sendError,
    sendNotice,
    selectedRecipients,
    removeParticipant,
    resetAiContinuity,
    renameRoom,
    restoreLatestBackup,
    saveParticipantProfile,
    selectRoom,
    sendMessage,
    toggleParticipantMenu: () => setParticipantMenuOpen((isOpen) => !isOpen),
    clearRoomMutationError: () => setRoomMutationError(null),
    toggleRecipient,
    typingParticipantId,
  };
}
