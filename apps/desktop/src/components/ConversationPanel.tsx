import { type FormEvent, type MouseEvent, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { Avatar } from "./Avatar";
import type { ChatMessage, CodexTurnProgress, ParticipantMap } from "../types";
import { useUiPreferences } from "../uiPreferences";
import {
  readDesktopRoomImageArtifact,
  saveDesktopRoomImageArtifactToWorkspace,
  type RoomImageArtifact,
} from "../roomBridge";

type ConversationPanelProps = {
  messages: ChatMessage[];
  participants: ParticipantMap;
  typingParticipantId: string | null;
  roomId: string;
  codexTurnProgress: CodexTurnProgress | null;
};

type MessageContextMenuState = {
  copyFailed: boolean;
  messageId: string;
  x: number;
  y: number;
};

function imageExtension(fileName: string) {
  const separator = fileName.lastIndexOf(".");
  return separator >= 0 ? fileName.slice(separator + 1).toLowerCase() : "png";
}

function suggestedImageFileName(fileName: string) {
  const now = new Date();
  const part = (value: number) => value.toString().padStart(2, "0");
  const date = `${now.getFullYear()}${part(now.getMonth() + 1)}${part(now.getDate())}`;
  const time = `${part(now.getHours())}${part(now.getMinutes())}${part(now.getSeconds())}`;
  return `MIO-image-${date}-${time}.${imageExtension(fileName)}`;
}

function validImageFileName(fileName: string, expectedExtension: string) {
  const value = fileName.trim();
  const extensionSeparator = value.lastIndexOf(".");
  if (
    value.length === 0 ||
    value.length > 128 ||
    extensionSeparator <= 0 ||
    /[<>:"/\\|?*\u0000-\u001f]/u.test(value) ||
    /[ .]$/u.test(value) ||
    imageExtension(value) !== expectedExtension
  ) {
    return false;
  }
  const deviceStem = value.slice(0, value.indexOf(".")).toUpperCase();
  return !/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/u.test(deviceStem);
}

function MessageImage({
  artifactId,
  messageId,
  roomId,
}: {
  artifactId: string;
  messageId: string;
  roomId: string;
}) {
  const { t } = useUiPreferences();
  const dialogTitleId = useId();
  const dialogDescriptionId = useId();
  const [artifact, setArtifact] = useState<RoomImageArtifact | null>(null);
  const [suggestedFileName, setSuggestedFileName] = useState("");
  const [status, setStatus] = useState<"loading" | "ready" | "saving" | "saved" | "loadError">("loading");
  const [isSaveDialogOpen, setIsSaveDialogOpen] = useState(false);
  const [fileName, setFileName] = useState("");
  const [saveError, setSaveError] = useState<"invalid" | "failed" | null>(null);

  useEffect(() => {
    let active = true;
    setStatus("loading");
    void readDesktopRoomImageArtifact({ roomId, messageId, artifactId })
      .then((value) => {
        if (!active) return;
        setArtifact(value);
        setSuggestedFileName(suggestedImageFileName(value.fileName));
        setStatus("ready");
      })
      .catch(() => {
        if (active) setStatus("loadError");
      });
    return () => {
      active = false;
    };
  }, [artifactId, messageId, roomId]);

  function openSaveDialog() {
    if (!artifact) return;
    setFileName(suggestedFileName || suggestedImageFileName(artifact.fileName));
    setSaveError(null);
    setIsSaveDialogOpen(true);
  }

  function closeSaveDialog() {
    if (status === "saving") return;
    setIsSaveDialogOpen(false);
    setSaveError(null);
  }

  async function saveToWorkspace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!artifact || status === "saving") return;
    const relativePath = fileName.trim();
    if (!validImageFileName(relativePath, imageExtension(artifact.fileName))) {
      setSaveError("invalid");
      return;
    }
    setStatus("saving");
    setSaveError(null);
    try {
      await saveDesktopRoomImageArtifactToWorkspace({
        roomId,
        messageId,
        artifactId,
        relativePath,
      });
      setStatus("saved");
      setIsSaveDialogOpen(false);
    } catch {
      setStatus("ready");
      setSaveError("failed");
    }
  }

  if (!artifact) {
    return <p className={`message-image-status is-${status}`}>{status === "loadError" ? t("roomImageLoadFailed") : t("roomImageLoading")}</p>;
  }

  return (
    <figure className="message-image">
      <a href={artifact.dataUrl} rel="noreferrer" target="_blank" title={t("roomImageOpenOriginal")}>
        <img alt={t("roomGeneratedImage")} src={artifact.dataUrl} />
      </a>
      <figcaption>
        <a download={suggestedFileName || artifact.fileName} href={artifact.dataUrl}>{t("roomImageDownload")}</a>
        <button disabled={status === "saving" || status === "saved"} onClick={openSaveDialog} type="button">
          {status === "saving" ? t("roomImageSaving") : status === "saved" ? t("roomImageSaved") : t("roomImageSaveWorkspace")}
        </button>
      </figcaption>
      {isSaveDialogOpen ? createPortal(
        <div
          className="message-image-save-backdrop"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) closeSaveDialog();
          }}
        >
          <form
            aria-describedby={dialogDescriptionId}
            aria-labelledby={dialogTitleId}
            aria-modal="true"
            className="message-image-save-dialog"
            onKeyDown={(event) => {
              if (event.key === "Escape") closeSaveDialog();
            }}
            onSubmit={saveToWorkspace}
            role="dialog"
          >
            <header>
              <h2 id={dialogTitleId}>{t("roomImageSaveTitle")}</h2>
              <button disabled={status === "saving"} onClick={closeSaveDialog} title={t("close")} type="button">×</button>
            </header>
            <p id={dialogDescriptionId}>{t("roomImageSaveDescription")}</p>
            <label>
              <span>{t("roomImageFileName")}</span>
              <input
                autoFocus
                disabled={status === "saving"}
                maxLength={128}
                onChange={(event) => {
                  setFileName(event.target.value);
                  setSaveError(null);
                }}
                value={fileName}
              />
            </label>
            {saveError ? (
              <p className="message-image-save-error" role="status">
                {saveError === "invalid" ? t("roomImageFileNameInvalid") : t("roomImageSaveFailed")}
              </p>
            ) : null}
            <footer>
              <button disabled={status === "saving"} onClick={closeSaveDialog} type="button">{t("cancel")}</button>
              <button className="is-primary" disabled={status === "saving"} type="submit">
                {status === "saving" ? t("roomImageSaving") : t("roomImageSaveConfirm")}
              </button>
            </footer>
          </form>
        </div>,
        document.body,
      ) : null}
    </figure>
  );
}

function recipientLabel(message: ChatMessage, participants: ParticipantMap) {
  const names = message.targetIds
    .map((participantId) => participants[participantId]?.displayName)
    .filter((name) => name !== undefined);

  return names.length > 0 ? `To ${names.join(" + ")}` : "";
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const transfer = document.createElement("textarea");
    try {
      transfer.value = text;
      transfer.setAttribute("readonly", "");
      transfer.style.position = "fixed";
      transfer.style.opacity = "0";
      document.body.append(transfer);
      transfer.select();
      return document.execCommand("copy");
    } catch {
      return false;
    } finally {
      transfer.remove();
    }
  }
}

export function ConversationPanel({
  codexTurnProgress,
  messages,
  participants,
  typingParticipantId,
  roomId,
}: ConversationPanelProps) {
  const { t } = useUiPreferences();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [contextMenu, setContextMenu] = useState<MessageContextMenuState | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const contextMessage = contextMenu
    ? messages.find((message) => message.id === contextMenu.messageId) ?? null
    : null;
  const contextAuthor = contextMessage ? participants[contextMessage.authorId] : null;
  const typingParticipant = typingParticipantId
    ? participants[typingParticipantId]
    : undefined;

  useEffect(() => {
    if (!contextMenu) return;
    const close = (event: PointerEvent) => {
      if (event.target instanceof Node && contextMenuRef.current?.contains(event.target)) return;
      setContextMenu(null);
    };
    const closeFromKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setContextMenu(null);
    };
    const closeFromWindow = () => setContextMenu(null);
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", closeFromKey);
    window.addEventListener("blur", closeFromWindow);
    window.addEventListener("resize", closeFromWindow);
    const frame = window.requestAnimationFrame(() => {
      contextMenuRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    });
    return () => {
      window.cancelAnimationFrame(frame);
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", closeFromKey);
      window.removeEventListener("blur", closeFromWindow);
      window.removeEventListener("resize", closeFromWindow);
    };
  }, [contextMenu]);

  function openMessageContextMenu(event: MouseEvent, messageId: string) {
    event.preventDefault();
    const menuWidth = 224;
    const menuHeight = 96;
    setContextMenu({
      copyFailed: false,
      messageId,
      x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
      y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
    });
  }

  async function copyContextMessage() {
    if (!contextMessage) return;
    if (await copyText(contextMessage.body)) {
      setContextMenu(null);
      return;
    }
    setContextMenu((current) => current ? { ...current, copyFailed: true } : null);
  }

  useEffect(() => {
    if (!codexTurnProgress || codexTurnProgress.roomId !== roomId) {
      setElapsedSeconds(0);
      return;
    }
    const updateElapsed = () => {
      setElapsedSeconds(Math.max(0, Math.floor((Date.now() - codexTurnProgress.startedAt) / 1_000)));
    };
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 1_000);
    return () => window.clearInterval(timer);
  }, [codexTurnProgress, roomId]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      const container = scrollRef.current;
      if (!container) {
        return;
      }
      container.scrollTo({
        top: container.scrollHeight,
        behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
          ? "auto"
          : "smooth",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [messages, typingParticipantId]);

  const codexProgressLabel = codexTurnProgress?.roomId === roomId
    ? (() => {
        switch (codexTurnProgress.phase) {
          case "preparing": return t("codexProgressPreparing");
          case "thinking": return t("codexProgressThinking");
          case "reconnecting": return t("codexProgressReconnecting");
          case "workspace": return t("codexProgressWorkspace");
          case "tool": return t("codexProgressTool");
          case "generatingImage": return t("codexProgressGeneratingImage");
          case "writingResponse": return t("codexProgressWritingResponse");
        }
      })()
    : null;

  return (
    <section aria-label={t("conversation")} className="conversation-panel">
      <div className="conversation-scroll" aria-live="polite" ref={scrollRef}>
        {messages.length === 0 ? (
          <div className="empty-conversation">
            <span aria-hidden="true">☕</span>
            <strong>{t("quietRoom")}</strong>
            <p>{t("firstMessage")}</p>
          </div>
        ) : (
          messages.map((message) => {
            const author = participants[message.authorId];
            if (!author) {
              return null;
            }

            const isUser = author.kind === "human";
            return (
              <article
                className={`message-row ${isUser ? "is-user" : "is-ai"}`}
                key={message.id}
              >
                <Avatar participant={author} size="large" />
                <div className="message-content">
                  <header>
                    <strong>{author.displayName}</strong>
                    <span className="participant-canonical-name">{author.canonicalName}</span>
                    <span>{author.serviceLabel}</span>
                    <time>{message.sentAt === "いま" ? t("now") : message.sentAt}</time>
                  </header>
                  <div
                    className="message-bubble"
                    onContextMenu={(event) => openMessageContextMenu(event, message.id)}
                  >
                    <p>{message.body}</p>
                    {message.artifactIds?.map((artifactId) => (
                      <MessageImage artifactId={artifactId} key={artifactId} messageId={message.id} roomId={roomId} />
                    ))}
                    <footer>
                      <span>{recipientLabel(message, participants)}</span>
                      <span className="message-provenance">
                        {message.provenance === "codexOwnerProxy" ? (
                          <em>{t("viaCodex")}</em>
                        ) : null}
                        {message.isDemo ? <em>UI DEMO</em> : null}
                      </span>
                    </footer>
                  </div>
                </div>
              </article>
            );
          })
        )}

        {typingParticipant ? (
          <div className="typing-row">
            <Avatar participant={typingParticipant} size="small" />
            <span>
              {typingParticipant.id === "codex" && codexProgressLabel
                ? <>
                    {t("codexProgress", {
                      name: typingParticipant.displayName,
                      phase: codexProgressLabel,
                    })}
                    <span aria-hidden="true" className="typing-elapsed">
                      {t("codexProgressElapsed", { seconds: elapsedSeconds })}
                    </span>
                  </>
                : t("thinking", { name: typingParticipant.displayName })}
            </span>
            <i />
            <i />
            <i />
          </div>
        ) : null}
      </div>

      {contextMenu && contextMessage && contextAuthor ? (
        <div
          aria-label={t("messageActions", { name: contextAuthor.displayName })}
          className="room-context-menu message-context-menu"
          onContextMenu={(event) => event.preventDefault()}
          ref={contextMenuRef}
          role="menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <strong className="room-context-menu-title">{contextAuthor.displayName}</strong>
          <button onClick={() => void copyContextMessage()} role="menuitem" type="button">
            {t("copyMessage")}
          </button>
          {contextMenu.copyFailed ? (
            <span className="room-context-menu-error" role="alert">{t("copyMessageFailed")}</span>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
