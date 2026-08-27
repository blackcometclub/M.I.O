# ADR 0032: Gemini Antigravity chat-only adapter

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0020 (Room-owned AI continuity), ADR 0027 (Durable AI dispatch ledger), ADR 0031 (Device-local AI participant guidance)

## Context

The Google Search Gemini browser bridge proved a manual browser experiment but
did not provide a normal autonomous participant. The owner has an authenticated
Antigravity CLI installation whose print mode returns bounded JSON and a
resumable conversation id.

## Decision

1. Keep the browser bridge as an opt-in play experiment. When Antigravity CLI
   is installed, the stable `gemini` participant uses the native CLI adapter in
   preference to the browser bridge.
2. Use the CLI's own Google OAuth storage. M.O.E. never reads, copies, or stores
   Google credentials.
3. Start one hidden child process only when a Gemini turn is sent. Use print
   mode with JSON output, accept one bounded response, and let the process exit
   immediately afterward. No PowerShell or terminal window remains open.
4. Run the child from `%LOCALAPPDATA%/M.O.E/GeminiChat`, never from a selected
   Codex workspace. The prompt declares conversation-only operation and
   forbids files, commands, browsing, and tools. The owner's Antigravity
   project permission deny rules remain an external prerequisite.
5. Accept only `SUCCESS` JSON with a non-empty response of at most 800
   characters and a bounded conversation id. Apply a 210-second product
   deadline and bounded stdout/stderr readers.
6. Persist the Antigravity conversation id per Room and the immutable
   `gemini` participant. Resume it only while the adapter and participant
   guidance environment key still matches.
7. Report `Installed` after executable detection and `Ready` only after one
   valid live reply. A native CLI installation takes status priority over the
   browser experiment.
8. Authentication UI, model selection, token accounting, streaming,
   cancellation, workspace access, and automatic creation of Antigravity
   permission projects remain separate decisions.

## Consequences

- Gemini participates in the same saved Room and continuity flow as Codex and
  Grok without a visible terminal.
- Cold-start and provider latency still apply to every one-shot child process.
- Antigravity CLI flags and JSON remain an external compatibility boundary.
- A machine without the required CLI login and deny-configured project must
  complete that setup outside M.O.E.
