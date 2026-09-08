# ADR 0041: Windows-safe product and shortcut name

- Status: Accepted
- Date: 2026-08-31
- Depends on: ADR 0038 (public release preparation)
- Superseded in part by: ADR 0044 (install mode only), ADR 0046 (installed binary names)

## Context

The visible brand is `M.I.O.`. A trailing period is valid in the Tauri window
title, but Windows strips or normalizes trailing periods in filesystem names.
Passing the same value to the NSIS bundler produced an installer named
`M.I.O._<version>_x64-setup.exe` and a shortcut candidate named `M.I.O..lnk`.

Changing the application window title or the internal Rust binary name is not
required to correct those Windows surfaces.

## Decision

1. Keep `M.I.O.` in the base Tauri configuration and in the visible window title.
2. Override only the installer build flavor with the Windows-safe product name
   `M.I.O`, without a trailing period.
3. Keep the stable application identifier `app.moe.desktop`, Rust binary name
   `moe-desktop`, install mode `currentUser`, and bundled helper identity.
4. Derive expected installer paths in build and release scripts from the
   installer configuration instead of duplicating the product name in scripts.
5. Treat the expected Windows names as `M.I.O_<version>_x64-setup.exe` for the
   installer and `M.I.O.lnk` for Start menu or optional desktop shortcuts.
6. Do not install a verification build automatically. Verify the generated
   installer and NSIS script first; installation and uninstall registration are
   separate user-approved release checks.

## Consequences

- The in-app brand remains `M.I.O.`.
- Windows files, install directories, and shortcuts no longer contain a doubled
  period or a trailing-period normalization edge case.
- Release scripts fail closed when the installer product name is missing or ends
  in a period.
- Pre-V1 alpha installations created with the old product name may require an
  explicit uninstall before a clean V1 install. This must be checked on any host
  that still has an alpha installation registered.

## Verification

On 2026-08-31, the Windows x64 NSIS build produced:

- `M.I.O_0.1.0-alpha.2_x64-setup.exe`
- SHA-256 `532F87BC52507D6FFD0A875611ABCFD320E11416594EFD875EF2624AB4F150B6`
- Authenticode status `NotSigned`, as expected for the verification build

The generated `installer.nsi` defines `PRODUCTNAME` as `M.I.O`, installs under
`%LOCALAPPDATA%\M.I.O` in current-user mode, writes the uninstall display name as
`M.I.O`, and creates Start menu and optional desktop shortcuts through
`${PRODUCTNAME}.lnk`. No installer was executed during this verification.
