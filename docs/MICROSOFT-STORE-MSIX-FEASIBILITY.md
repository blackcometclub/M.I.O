# Microsoft Store MSIX feasibility

Status: Submission 1 is in Microsoft Store certification as of 2026-09-04, with publishing held until the Owner selects `Publish now`.

## Decision boundary

M.I.O. keeps commercial dual licensing. The first signing route under test is a Microsoft Store MSIX package.
The existing NSIS installer remains the RC/Preview route until the Store-signed MSIX passes real install and
Provider/workspace regression tests. Submission 1 was sent for certification with the Owner's explicit
approval on 2026-09-03. This document does not authorize cancellation, package replacement, resubmission,
further publication-setting changes, selecting `Publish now`, or a change to the commercial license. On
2026-09-04, the Owner explicitly changed the active submission from immediate publishing after certification
to manual publishing while allowing certification to continue.

## Why MSIX is viable

- Microsoft Store signs an accepted MSIX/AppX submission. A CA-trusted certificate is not required for the
  uploaded package. The Store does not provide this signing for the existing NSIS EXE.
- M.I.O. can be packaged as a full-trust desktop application (`Windows.FullTrustApplication` plus
  `runFullTrust`). This preserves ordinary medium-integrity Win32 behavior instead of moving M.I.O. into an
  AppContainer.
- The main executable and `moe-command-helper.exe` can be placed together in the package. That matches the
  product helper locator, which requires the helper to be next to the running executable and verifies its
  build-time SHA-256.
- The selected workspace and external Provider CLIs remain outside the read-only package directory. A
  full-trust desktop package can access them with the same user permissions, but this must be proven with an
  installed Store-signed or trusted test package.
- Windows 11 includes the Evergreen WebView2 Runtime. The V1 support boundary is Windows 11 x64, so the NSIS
  bootstrapper is not copied into the MSIX. A clean Windows 11 test must still confirm the runtime boundary.

## Known compatibility risks

1. Store identity values must exactly match Partner Center. The reserved M.I.O. product values are now fixed in
   the package command; changing them would create a different package family.
2. MSIX installs per user under the protected WindowsApps package directory. Package files are read-only and
   the package location changes on update. M.I.O. must never write beside its executable.
3. Package identity can affect AppData paths and virtualization. The existing NSIS Room data must be preserved
   and migrated deliberately; a Store install must not silently create an unrelated empty data set.
4. External process launch must be tested for Codex, Claude Code, Gemini Antigravity CLI, Grok Build, Git,
   Node/npm, and the bundled command helper. Passing an unpackaged RC test is not evidence for MSIX.
5. Store certification may require an explanation for `runFullTrust`, workspace access, external CLI launch,
   user credentials held by Provider CLIs, and network use by those CLIs.
6. The current Tauri 2.11.4 CLI only offers MSI and NSIS Windows bundles. MSIX is therefore assembled with the
   Windows SDK `MakeAppx.exe`, not with an undocumented Tauri target.
7. The submitted package manifest is `1.0.0.0`, while its packaged PE version metadata remains
   `1.0.0-rc.1`. The current checkout was synchronized to source version `1.0.0` after submission; this does not
   alter the package already under certification. The submitted binary-label mismatch must therefore be
   explicitly accepted or replaced through a separate approved submission before Store publication.

## Local package command

First build the current release executable and helper through the normal release preparation path. Then run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-store-msix.ps1
```

This produces a timestamped, unsigned `.msix` and `.sha256` file under `.tools/store-msix/` using the reserved
M.I.O. Store identity. It validates the
manifest while packing, unpacks the result again, and checks that the main executable, command helper, license,
third-party notices, and tile assets are present. The unsigned package must not be sideloaded or published.

The defaults are the exact values shown by Partner Center on 2026-09-03:

- Product name: `M.I.O`
- Store ID: `9NS9B7T71XHN`
- Package/Identity/Name: `TINMOON.M.I.O`
- Package/Identity/Publisher: `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`
- Package/Properties/PublisherDisplayName: `TINMOON`
- Package Family Name: `TINMOON.M.I.O_kk3nwjsrdvfmr`

Do not store Partner Center credentials or signing keys in the repository.

## Evidence required before choosing MSIX for V1

- Package structure and manifest validation pass on a package built from one fixed source commit.
- A trusted test package or Store flight installs, starts, exits, updates, and uninstalls on clean Windows 11 x64.
- Existing NSIS Room data is backed up and is either preserved or migrated into the packaged data location.
- Direct and Conductor smoke tests pass for every supported Provider.
- Selected workspace read/write, image save, backup/restore, Git status, npm install, and command confirmation pass.
- `moe-command-helper.exe` is present beside the main EXE and its embedded hash check passes.
- Uninstall behavior for Room data is documented and matches the product promise.
- Store certification result and any required restricted-capability justification are recorded before V1 is
  called signed or complete.

## Submission 1 and current local preflight

The Owner reserved `M.I.O` in Partner Center as an MSIX or PWA app. The package command defaults use the exact
reserved identity values. Windows SDK 10.0.26100.0 `MakeAppx.exe` created and unpacked the submitted structural
package successfully.

The submitted package is:

- Path: `.tools/store-msix/M.I.O_1.0.0.0_x64_store-unsigned_20260903-173912.msix`
- Size: 6,505,409 bytes
- SHA-256: `06BDC3DAEE7D8AD7D94983E34806D8B30FC30F1471DA32668586AE12220867B4`
- Identity: `TINMOON.M.I.O`
- Publisher: `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`
- Architecture: x64
- Target: `Windows.Desktop`, minimum build `10.0.22000.0`
- Capability: `runFullTrust` only
- Package signature: unsigned by design before Store processing

Partner Center accepted the upload as `Validated`, and Submission 1 (ID `1152921505701802472`) was sent for
certification on 2026-09-03. On 2026-09-04, preprocessing was complete, certification was in progress, and
publishing had not started. The Owner then selected and applied the Partner Center option that does not publish
the submission until `Publish now` is selected. Partner Center confirmed that publishing will start only after
that action; certification remained in progress, and the submission was not cancelled or replaced.

A read-only local re-audit on 2026-09-04 confirmed:

- the package SHA-256 and size still match the submitted record;
- the manifest contains no elevation, service, startup, broad-file-system, camera, microphone, location,
  package-management, or other extra capability declaration;
- the package contains the main executable, command helper, license, generated third-party notices, four logo
  assets, manifest, block map, and content-types file;
- the packaged main executable, command helper, license, and third-party notices are byte-identical to the
  recorded local build inputs;
- the Windows App Certification Kit is installed locally, but it was not run against this unsigned submission
  because a package test can open and exercise the app and must not be confused with Store-signed installation
  evidence.

The main remaining local finding is the submitted release-label mismatch: the package identity version is
`1.0.0.0`, but its packaged `moe-desktop.exe` metadata is `1.0.0-rc.1`. On 2026-09-04, the current checkout's
root package, desktop package, Tauri configuration, Cargo workspace, lockfiles, and current release documents
were synchronized to `1.0.0`; historical RC evidence remains unchanged. This local preparation does not alter
the active submission. The Owner must decide whether to accept the submitted binary label or approve a
synchronized rebuild and separate submission.

Until that decision and the certification result are known, do not sideload the unsigned package, alter the
active submission, select `Publish now`, or call the Store route complete. Real installation testing waits for
the Store-signed package or another trusted package so Windows signature checks are not bypassed.

## Official references

- Microsoft Store publishing and Store-provided MSIX signing:
  <https://learn.microsoft.com/windows/apps/publish/get-started>
- MSIX preparation and compatibility boundaries:
  <https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-prepare>
- Packaged desktop app filesystem and runtime behavior:
  <https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-behind-the-scenes>
- Full-trust package capability:
  <https://learn.microsoft.com/windows/apps/package-and-deploy/app-capability-declarations>
