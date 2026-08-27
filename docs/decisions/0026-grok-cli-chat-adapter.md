# ADR 0026: Grok CLI chat-only adapter

- Status: Accepted
- Date: 2026-08-13

## Context

M.O.E. already has a provider-neutral text-turn contract and an immutable
`grok` participant identity, but Grok was previously an unsupported catalog
entry. The installed Grok Build CLI can return bounded JSON with a session ID
and can resume that session. Its default agent may also discover local tools,
skills, rules, MCP servers, hooks, and project instructions, which is too broad
for an ordinary Talk Room.

The existing multi-recipient dispatch command also returned one command-level
error when a recipient failed. Once a second live adapter exists, that could
hide a reply already saved for an earlier recipient.

## Decision

1. Add Grok as a separate `TextTurnAdapter`; do not add Grok-specific types to
   M.O.E. Core or the renderer Room model.
2. Use the user's installed Grok CLI and its own OAuth storage. M.O.E. does not
   read, copy, serialize, or expose Grok credentials.
3. The first Grok integration is chat-only. Generate a private agent profile
   with `tools: []`, disable subagents, Web search/fetch, cross-session memory,
   and Cursor/Claude compatibility sources for the child process. Run it from
   an M.O.E. app-data directory rather than a selected workspace. Native hooks
   configured by the user in Grok's own home remain Grok CLI behavior; they are
   not model-callable tools and M.O.E. does not edit that user configuration.
4. Send bounded Room context and the current message as untrusted JSON inside
   the prompt. Accept only a completed, bounded JSON response with a valid
   session ID.
5. Resolve outbound participant names from the device-local participant
   profile. Never export the human participant's internal ID; use `room-owner`
   and an anonymous `Room owner` fallback when no profile exists. A prompt
   contract version change starts a fresh AI session so an older fixed name is
   not retained through continuity.
6. Persist one Grok CLI session ID per Room and participant through the existing
   provider-neutral continuity store. Resume only when its chat-only
   environment key still matches.
7. CLI detection alone reports `Installed`, never `Ready`. The current app
   process reports `Ready` only after a valid live response is received.
8. Record failure per recipient. A failed recipient returns no message and a
   bounded error code, while successful recipients remain visible. Failed
   external turns are not automatically retried.
9. Keep Grok workspace access, tools, streaming UI, model selection, login UI,
   and cancellation as separate future decisions.

## Consequences

- A Talk Room can receive a real Grok reply without granting Grok local file or
  command access.
- User-configured Grok lifecycle hooks may still observe the child session. The
  currently installed hooks only report Grok session status to AGI Cockpit;
  removing or isolating native hooks requires a supported Grok configuration
  boundary and is not silently approximated here.
- Grok keeps conversational continuity independently from Codex and every other
  participant.
- Mixed Codex/Grok sends no longer lose a completed reply merely because the
  other adapter failed.
- The user must complete Grok CLI login outside M.O.E. when the CLI reports an
  authentication failure.
- Grok CLI output and flags remain an external compatibility boundary and are
  covered by parser, argument, dispatch, and live release checks.
