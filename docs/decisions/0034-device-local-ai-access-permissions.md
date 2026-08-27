# ADR 0034: Device-local AI access permissions

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0019 (Room-scoped Codex workspace), ADR 0020 (Room-owned AI continuity), ADR 0023 (Participant identity and local profile overrides)

## Context

M.O.E. already stores each AI's local name, avatar, and conversational guidance,
but access permissions were split between adapter code, the Codex Room workspace
setting, and provider-owned CLI configuration. The owner wants one honest place
to see and choose each participant's effective M.O.E. access without presenting
unsupported permissions as if they worked.

## Decision

1. Add a device-local access mode to each AI participant profile:
   `chatOnly`, `workspaceRead`, or `workspaceWrite`. A migration-only
   `providerDefault` value preserves existing profiles: Codex retains its
   existing read-write Room workspace behavior and other AIs remain chat-only.
2. Treat the profile mode as the maximum access M.O.E. may grant. A Room must
   still have an explicitly selected, available workspace before read or write
   access becomes effective. Without one, the turn is chat-only.
3. In the Windows alpha.1 capability matrix, Codex, Fable, Gemini, and Grok
   support chat-only mode. Workspace read and write remain visible but disabled
   because the Codex native Windows sandbox did not contain reads through a
   nested junction. Legacy `providerDefault` Codex profiles resolve to chat-only.
4. Keep the `workspaceRead` and `workspaceWrite` values and permission-profile
   implementation for isolated future validation. If an explicit saved Codex
   workspace mode reaches the Windows adapter, fail before provider start. Do
   not reactivate either UI mode until the root boundary passes its live test.
5. Web and network access remain disabled for every stable native participant.
   This tranche does not add a switch that cannot yet be enforced.
6. Include the effective access mode in the continuity environment key. A mode
   change therefore starts a fresh provider-side continuation on the next turn
   while retaining the saved Room history.
7. Store permissions only in M.O.E.'s versioned participant profile file. Do not
   rewrite provider-global settings or credentials. Gemini's existing
   M.O.E.-owned Antigravity project deny rules remain an external enforcement
   prerequisite until a documented granular configuration interface is
   validated.
8. Future adapters may enable additional modes only after their workspace-root,
   command, network, and denial behavior have product-path tests.

## Consequences

- The settings screen reports real capabilities instead of optimistic toggles.
- Existing selected-folder and access-mode data remain intact, but alpha.1 does
  not grant them to Codex on Windows.
- Fable, Gemini, and Grok remain safely usable while their broader permissions
  are designed and tested independently.
- Provider-specific permission editors are not yet replaced by a universal
  arbitrary allow/deny rule editor.
