# ADR 0025: Single-instance desktop ownership

- Status: Accepted
- Date: 2026-08-13

## Context

M.O.E. keeps Rooms, AI continuity, workspace bindings, participant profiles,
and appearance in process memory and persists them to device-local files. The
existing mutexes serialize writes only inside one process. Starting M.O.E.
twice therefore creates two independent writers that can overwrite each
other's newer snapshots and backups.

This is a present Windows V1 risk rather than a future multi-user concern: a
second taskbar click is enough to create it.

## Decision

1. Register the official Tauri single-instance plugin before every other
   plugin on desktop platforms.
2. Keep the first process as the sole owner of M.O.E. device-local persistence.
3. When a second launch is attempted, terminate the second process before app
   setup and persistence ownership begin.
4. Restore, show, and focus the existing `main` window so the second launch
   behaves like reopening the already running app.
5. Ignore the second process's command-line arguments and working directory in
   this tranche. File-open routing and deep links require separate contracts.
6. Do not change any persistence schema or renderer behavior.

## Consequences

- Two normal M.O.E. processes can no longer write the same app-data files.
- A minimized or hidden main window returns when M.O.E. is launched again.
- The official plugin adds one desktop-only Rust dependency.
- This does not replace transactional writes, backup recovery, or future
  cross-process protection for other tools that might write M.O.E. files.
- Automated tests continue to cover persistence behavior; a release executable
  must also be launched twice against isolated app data to verify process-level
  behavior on Windows.
