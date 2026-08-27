# ADR 0033: Claude Code Fable chat-only adapter

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0020 (Room-owned AI continuity), ADR 0027 (Durable AI dispatch ledger), ADR 0031 (Device-local AI participant guidance)

## Context

The owner wants Claude Fable to participate directly in M.O.E. through the
existing Claude Code subscription instead of copying Room messages between
applications. Claude Code 2.1.220 is installed as a native Windows executable,
supports non-interactive JSON output, accepts the `claude-fable-5` model id,
and returns a resumable session id.

## Decision

1. Keep the stable participant id `claude-code`, while presenting its current
   canonical identity as Claude Fable and its service as Claude Code Fable 5.
   Claude Web remains a separate participant and connection.
2. Use Claude Code's own Claude.ai authentication storage. M.O.E. never reads,
   copies, exports, or stores Anthropic credentials. An expired login must be
   renewed through Claude Code outside M.O.E.
3. Start one hidden child process only when a Fable turn is sent. Use print
   mode, JSON output, and the explicit model `claude-fable-5`; accept one
   bounded reply and let the process exit immediately. No PowerShell or
   persistent Claude process is required.
4. Run the child from `%LOCALAPPDATA%/M.O.E/ClaudeFableChat`, never from a Room
   workspace. Disable built-in tools, slash commands, Chrome integration,
   project customizations, and interactive permissions. Fable is conversation
   only in this adapter and cannot read or edit local files.
5. Accept only a non-error Claude Code result with a non-empty response of at
   most 800 characters and a valid UUID session id. Apply a 210-second product
   deadline and bounded stdout/stderr readers.
6. Persist one Claude Code session id per Room and the immutable
   `claude-code` participant. Resume only while the Fable adapter and local AI
   guidance environment key match.
7. Report `Installed` after executable detection and `Ready` only after one
   valid live Fable reply.
8. Provider-side availability, usage limits, subscription accounting, and data
   retention remain Anthropic policy. M.O.E. sends the bounded Room context and
   current message to Anthropic only when the owner addresses Fable.
9. Streaming, cancellation, workspace access, model selection UI, automatic
   login, and Claude Web relay behavior remain separate decisions.

## Consequences

- Fable can participate in the same saved Room, local profile, dispatch ledger,
  and continuity flow as Codex, Grok, and Gemini.
- The first request after Claude Code authentication expires fails safely and
  is not retried automatically; the owner must renew the login before sending a
  new Room message.
- One-shot process startup and provider latency still apply to every turn.
- Claude Code flags and JSON remain an external compatibility boundary.
