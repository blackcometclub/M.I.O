# ADR 0046: M.I.O. Windows binary names

- Status: Accepted
- Date: 2026-09-05
- Depends on: ADR 0041 (Windows-safe product and shortcut name), ADR 0044 (per-machine installer)

## Context

The visible product name is `M.I.O`, but the Windows installer and Store package
still installed `moe-desktop.exe` and `moe-command-helper.exe`. Those names expose
an old internal project name, make file searches for `MIO` confusing, and do not
match the product the user installed.

Changing the existing application data identifier at the same time would create
a different Tauri data directory. A Store update could then appear to have lost
the user's Rooms and settings even though the old data still existed.

## Decision

1. Name the product-visible Windows binaries `mio-desktop.exe` and
   `mio-command-helper.exe` in both NSIS and Microsoft Store packages.
2. Keep the internal Cargo package and crate names unchanged. Explicit Cargo bin
   targets provide the product-visible filenames without destabilizing internal
   dependency identities.
3. Keep the stable Tauri application identifier `app.moe.desktop` for V1 data
   continuity. Renaming that identifier requires a separate, tested data migration.
4. Require build, helper lookup, Store packaging, and release-preparation scripts
   to use the new executable filenames and fail closed when an expected file is
   missing.
5. Do not upload the previously generated `1.0.2.0` Store candidate containing
   the old filenames. Generate and verify a replacement package.

## Consequences

- Installed files now match the `M.I.O` product name and are discoverable as
  `mio-desktop.exe` and `mio-command-helper.exe`.
- Existing Rooms and settings continue to use the established data location.
- Internal source names may still contain `moe`; they are implementation
  identifiers and are not installed executable filenames.
- A clean uninstall and reinstall must confirm that no obsolete `moe-*.exe`
  remains in `C:\Program Files\M.I.O`.

## Build verification

On 2026-09-05, the Windows x64 release build produced `mio-desktop.exe` and
`mio-command-helper.exe`. The NSIS installer was rebuilt, and a replacement
unsigned Store package with version `1.0.2.0` was unpacked and verified to contain
both new names. Installation evidence is recorded separately after the clean
install check.

## Installation and submission verification

On the same day, the previous installation was removed and the replacement NSIS
installer was installed on the Windows 11 development machine. The application
launched successfully from `C:\Program Files\M.I.O\mio-desktop.exe`.
`mio-command-helper.exe` was present beside it, while `moe-desktop.exe` and
`moe-command-helper.exe` were absent. A filename search also found the new
`mio-desktop.exe` name.

The replacement Store package
`M.I.O_1.0.2.0_x64_store-unsigned_20260905-170824.msix` was uploaded to
Submission 2 after the owner approved the upload. Partner Center validated the
package as Windows 10/11 Desktop, x64, version `1.0.2.0`. Submission 2 was then
submitted after a separate owner approval. At the last check on 2026-09-05 it
was in update certification preprocessing; this was not treated as certification
completion or publication of the update.
