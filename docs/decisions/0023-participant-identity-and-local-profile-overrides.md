# ADR 0023: Participant identity and local profile overrides

- Status: Accepted
- Date: 2026-08-12

## Context

People want to give every participant, including the room owner and connected
AI, a friendly name and a custom circular avatar. That personalization must not
change dispatch routing or make Claude Web, Claude Code, Codex, DeepSeek, Grok,
or future adapters ambiguous.

## Decision

1. Keep the participant ID and canonical adapter-supplied name immutable.
   Routing, history, continuation, and connection state continue to use the ID.
2. Store a device-local profile override separately from Room history. It may
   contain a display name and one bounded PNG, JPEG, or WebP data URL with a
   normalized crop translation and zoom.
3. Show both identities: the editable display name is primary, while a compact
   immutable canonical-name chip and avatar badge remain visible. The owner
   uses an `Owner` badge.
4. The profile editor uses an SNS-style circular crop preview. Dragging changes
   translation and a bounded 100%-600% slider changes zoom. The original image
   and placement remain available for later editing.
5. Validate participant existence, names, image media type, encoded size, and
   finite placement bounds in the desktop backend before atomic persistence.
   A corrupt primary profile file may recover from its last valid backup and
   must not prevent the app from opening.
6. Profiles are shared across all Rooms on the current device. Room JSON and
   provider/adapter contracts do not carry image payloads. Profile backup and
   cross-device synchronization are separate future capabilities.

## Consequences

- Renaming `Codex` to `コデちゃん` cannot change the `codex` dispatch target.
- New adapters get the same UI automatically from their canonical participant
  metadata; the UI does not need a provider-specific branch.
- Existing Room backups stay small and provider neutral.
- Removing local profile data restores the canonical name and generated avatar.
