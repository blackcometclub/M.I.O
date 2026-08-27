# ADR 0020: Room-owned AI continuity and shared context synchronization

- Status: Accepted
- Date: 2026-08-12
- Depends on: ADR 0019 (room-scoped Codex workspace)

## Context

M.O.E. stores a durable Room transcript, but the first Codex adapter created a new ephemeral Codex thread for every human message and passed only that message. The UI therefore looked continuous while the AI participant was stateless. The same mismatch would recur for Claude, Gemini, or another provider if continuity were implemented independently inside each adapter.

M.O.E. needs two separate forms of continuity:

1. Native provider continuity, such as a Codex App Server thread id that can be resumed.
2. Provider-neutral Room continuity, so every addressed AI can receive new statements made by people and other AIs since its previous turn.

## Decision

### The Room owns continuity

M.O.E. stores one opaque native session id per `(roomId, participantId)`. Adapters receive only a provider-neutral instruction to start a persistent session or resume an opaque session id. Provider-specific ids and protocol methods do not enter the Room core or webview.

The device-local continuity record also stores:

- the last Room message synchronized to that participant;
- an environment key derived from chat-only mode or the selected workspace.

The record is stored separately from the portable Room backup. It contains device-specific provider state and must not be exported as conversation content.

### Native continuation first, bounded replay as recovery

For Codex, a first Room turn uses a non-ephemeral `thread/start`. Later turns use `thread/resume` with the recorded thread id. A changed workspace, missing Room cursor, invalid native id, or an excessive unsynchronized gap starts a fresh native thread instead of resuming with ambiguous context.

When a fresh thread is required, M.O.E. supplies a bounded recent Room snapshot. When a thread is resumed, M.O.E. supplies only bounded messages after the saved cursor and before the current human message. Prior Codex replies are excluded from a resumed delta because they already exist in the native thread.

This same contract supports future providers:

- use a native conversation/session id when the provider exposes one;
- otherwise start a stateless turn with the bounded Room snapshot or delta;
- never make the shared Room transcript depend on one provider's private history format.

### Room content remains untrusted

Synced messages are serialized with explicit author, recipient, timestamp, and body fields. They are labelled as untrusted conversational context, not system or developer instructions. A participant may discuss statements present in the supplied Room record but may not infer delivery, connection, or awareness outside that record.

### Commit and recovery boundary

The AI reply is persisted in the Room before advancing the continuity cursor. If continuity-state persistence fails after the reply is saved, the reply remains visible and the next turn may recover by starting a fresh native session from the Room snapshot. Conversation data is preferred over silently losing a reply.

Only one Codex turn is admitted at a time in the desktop process. This prevents two Room messages from racing to resume and advance the same native thread.

## Initial implementation

This decision initially implements the contract for Codex only:

- persistent room-and-participant continuity state in Rust;
- persistent `thread/start` followed by `thread/resume`;
- initial recent snapshot and subsequent Room-delta synchronization;
- automatic reset when the selected workspace changes;
- no network access and the existing room-scoped filesystem boundary.

Claude Code, Claude Web, and Gemini remain separate adapters. They can adopt the same continuity store and Room context packet without changing the Room model.

## Deferred

- Selecting or forking an existing Codex desktop task into a Room.
- Shared reset behavior for adapters other than the native Codex and Grok integrations (the local reset and degradation UI are defined by ADR 0029).
- Shared summaries for Rooms whose relevant history exceeds the bounded replay window.
- Provider-specific continuity for Claude Code, Claude Web, and Gemini.
- Cross-provider simultaneous dispatch snapshots and merge ordering.

## Consequences

- A Codex participant can retain its own prior turns across M.O.E. restarts.
- Messages from other Room participants become visible to Codex as explicit shared context.
- Changing the workspace intentionally creates a new Codex continuity boundary.
- Native sessions may outlive a deleted Room until a future cleanup command is added; they are not reused unless the exact Room id, participant id, environment key, and cursor remain valid.
- A model invocation is still a new inference, but it receives a durable conversational lineage instead of behaving as a first-time participant.
