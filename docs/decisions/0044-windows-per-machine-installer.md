# ADR 0044: Windows V1 per-machine installer

- Status: Accepted
- Date: 2026-09-02
- Supersedes: ADR 0041 decision 3 (install mode only)
- Depends on: ADR 0041 (Windows-safe product and shortcut name), ADR 0043
  (Windows V1 product scope)

## Context

The release-candidate NSIS installer originally used Tauri's `currentUser`
mode. It correctly registered `M.I.O` under the interactive user's HKCU
uninstall key, but the target Windows 11 machine did not enumerate that entry
in Settings, Programs and Features, `Get-Package`, or `winget list`.

A temporary, non-product HKLM visibility probe appeared in both Windows
Settings and Programs and Features on the same machine. Dots in the display
name were separately ruled out. The probe was removed after the check.

V1 needs a normal, discoverable uninstall path. Requiring one UAC approval at
install and uninstall is preferable to leaving a correctly installed product
invisible to the maintenance UI used by its user guide.

## Decision

1. Build the V1 Windows NSIS installer in Tauri `perMachine` mode.
2. Install application binaries under `%ProgramFiles%\M.I.O` and register the
   product for machine-wide discovery and uninstall.
3. State clearly that install and uninstall require Windows UAC Administrator
   approval. M.I.O. does not otherwise run elevated.
4. Keep product data per Windows user under the existing
   `%APPDATA%\app.moe.desktop` and `%LOCALAPPDATA%\app.moe.desktop` locations.
   Uninstall preserves that data unless a separately explicit deletion action
   is chosen.
5. Treat migration from a pre-V1 current-user alpha as a boundary: back up
   Rooms, uninstall the old application without deleting retained product
   data, and then install V1. Later per-machine V1 updates may overwrite the
   existing installation when their Release notes permit it.
6. Do not infer success from the registry alone. The final release candidate
   must be visibly confirmed in Windows Settings and Programs and Features,
   then uninstalled through the documented Windows path while Room data remains
   unchanged.

## Consequences

- M.I.O. is expected to appear in the normal Windows application maintenance
  surfaces on the validated Windows 11 target.
- Installation is no longer silent with respect to privilege: Windows displays
  a UAC prompt.
- Application files are shared machine-wide, while each Windows account keeps
  separate M.I.O. product data.
- A pre-V1 current-user installation must not be left alongside V1 because the
  two copies can create ambiguous shortcuts and launch paths.
- The per-machine installer and uninstall behavior remain release blockers
  until the real installer path, shortcuts, registration, visible Settings
  entry, uninstall removal, and retained Room data are verified.
