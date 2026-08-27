import { useEffect, useRef } from "react";

import { Avatar } from "./Avatar";
import type { ChatMessage, ParticipantMap } from "../types";
import { useUiPreferences } from "../uiPreferences";

type ConversationPanelProps = {
  messages: ChatMessage[];
  participants: ParticipantMap;
  typingParticipantId: string | null;
};

function recipientLabel(message: ChatMessage, participants: ParticipantMap) {
  const names = message.targetIds
    .map((participantId) => participants[participantId]?.displayName)
    .filter((name) => name !== undefined);

  return names.length > 0 ? `To ${names.join(" + ")}` : "";
}

export function ConversationPanel({
  messages,
  participants,
  typingParticipantId,
}: ConversationPanelProps) {
  const { t } = useUiPreferences();
  const scrollRef = useRef<HTMLDivElement>(null);
  const typingParticipant = typingParticipantId
    ? participants[typingParticipantId]
    : undefined;

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
                  <div className="message-bubble">
                    <p>{message.body}</p>
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
            <span>{t("thinking", { name: typingParticipant.displayName })}</span>
            <i />
            <i />
            <i />
          </div>
        ) : null}
      </div>
    </section>
  );
}
