# ADR 0024: Device-local appearance persistence

- Status: Accepted
- Date: 2026-08-13

## Context

M.O.E. can change its shell color, shell background image, and full-shell
artwork placement, but those values currently live only in React state and are
lost when the app closes. Images are too large and important for browser
`localStorage`, and Room backups should remain focused on conversations.

## Decision

1. Store one versioned appearance document in the Tauri application-data
   directory, separately from Rooms, participant profiles, and UI text
   preferences.
2. Persist the shell color, optional raster background image, and optional
   raster full-shell artwork with its filename and normalized placement.
3. Accept only bounded PNG, JPEG, and WebP data URLs. Validate media type,
   encoded size, color syntax, finite placement, and total document size in
   Rust before writing.
4. Save changes automatically and restore them during application startup.
   The appearance panel reports saving, saved, and failed states.
5. Write through a temporary file and retain the previous valid document as a
   backup. A corrupt primary may recover from the backup and must not prevent
   M.O.E. from opening.

## Consequences

- Appearance survives restarts without enlarging Room history or backups.
- Large or unsupported images fail closed and leave the prior saved appearance
  intact.
- Appearance remains local to the current Windows user and device. Theme
  export/import and cross-device synchronization remain future features.
