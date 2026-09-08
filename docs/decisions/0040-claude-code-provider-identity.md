# ADR 0040: Claude Code provider identity

- Status: Accepted
- Date: 2026-08-31
- Depends on: ADR 0018 (separate Claude connections), ADR 0023 (participant identity and local profiles), ADR 0039 (provider model selection)

## Context

M.I.O. connects to the Claude Code CLI, while early UI and adapter wording called
the participant `Claude Fable`. `Fable` is the selected model family rather than
the connection product. Showing both as one identity makes provider availability,
sign-in failures, billing, retention, and model selection harder to explain.

Existing installations may also have a device-local participant display name such
as `Claude Fable`. That owner-controlled nickname must not be overwritten by a
provider-label migration.

## Decision

1. Use `Claude Code` as the locked canonical connection identity in the product UI,
   provider status, dispatch errors, and adapter prompt.
2. Show model information separately from the connection identity. The verified
   explicit adapter model IDs are `claude-fable-5`, `claude-opus-5`, and
   `claude-sonnet-5`; provider-default mode continues to omit `--model`.
3. Keep the stable participant ID `claude-code` and existing persisted Room data.
   Keep any device-local participant display name or avatar chosen by the owner.
4. Advance the Claude continuity environment generation when the canonical adapter
   prompt changes. The next Claude Code turn starts a new provider session and
   reconstructs bounded Room context instead of resuming under changed instructions.
5. Internal Rust type names and legacy storage directory names may retain `Fable`
   where renaming would add migration risk without changing user-visible identity.

## Consequences

- The UI clearly separates the Claude Code connection from its selected model.
- Existing nicknames remain visible as owner customization beside the locked
  `Claude Code` identity.
- One new Claude Code provider session is expected after this change; M.I.O. Room
  history remains intact.
