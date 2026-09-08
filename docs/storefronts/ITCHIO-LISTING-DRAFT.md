# M.I.O. itch.io page draft

Status: the public page was updated to `v1.0.4` on September 7, 2026. The English update history is
published as an itch.io Devlog.

## Project fields

- Title: `M.I.O.`
- Project URL candidate: `mio-talk-room`
- Classification: Tool
- Kind of project: Downloadable
- Release status: Released only after the final package passes its release gates
- Pricing: `$0 or Donate`
- Suggested donation: Owner decision before publication
- Platforms: Windows only
- Languages: English and Japanese

itch.io treats `$0 or Donate` as a free download with an optional payment. A person who skips payment can
download the file without obtaining an itch.io ownership key. Keep this distinction out of the marketing
copy unless a support question requires it.

## Short description

Bring Codex, Claude Code, Gemini, and Grok into one persistent Talk Room on Windows 11.

## English description

**M.I.O. (Malevolent Immortal Overdrive)** is a local-first Windows 11 desktop application that brings
several command-line AI providers into one persistent Talk Room. One human Owner decides which AI receives
each message, what local access that participant is allowed to use, and when a sensitive development action
may proceed.

M.I.O. is not an AI model, a Provider subscription, or a hosted proxy service. It is a desktop coordination
layer for Provider CLIs that you install and authenticate separately. M.I.O. can start without any Provider
CLI; install only the AI connections you intend to use.

### Two ways to work

**Direct mode** sends a message only to the participants selected for that turn. A Room can contain several
AIs without automatically sharing every message with all of them.

**Conductor mode** lets Codex answer directly or coordinate one bounded round of work with up to three
selected workers. It is intentionally not an unlimited autonomous loop: each Owner request ends after that
single round, and nested delegation is not supported.

The Send button becomes a cancellation control while a Provider turn is active. If delivery may have
reached a Provider but the final result is unknown, M.I.O. records that uncertainty and does not retry the
message automatically, avoiding an accidental duplicate turn.

### Rooms, participants, and continuity

- Create multiple named Talk Rooms with persistent, device-local conversation history.
- Choose the recipients separately for each message.
- Give every AI a display name, optional behavioral guidance, icon, and supported model selection.
- Keep Provider-native continuity where the adapter supports it, while rebuilding bounded context from the
  Room history when continuity must restart.
- Reset one participant's native continuity without deleting Room messages, profiles, or local workspaces.
- Change the interface language, appearance, wallpaper, compact layout, and Room presentation on the device.

### Provider capabilities in V1

Capabilities are deliberately different for each Provider. M.I.O. does not pretend that every connected AI
has the same tools or security boundary.

- **Codex:** conversation, Conductor and worker roles, verified model selection, generated images,
  selected-folder read or read/write access, and a small fixed set of bounded development operations.
- **Claude Code:** conversation and worker roles with verified model selection. Workspace, command, Web, and
  MCP access are not enabled through M.I.O. V1.
- **Gemini Antigravity CLI:** conversation and worker roles using the Provider-default model. Workspace,
  command, Web, and MCP access are not enabled through M.I.O. V1.
- **Grok CLI:** conversation through the local CLI. General workspace reading, editing, commands, Web access,
  memory, and subagents are not enabled through M.I.O. V1.
- **Claude Web and arbitrary/custom Providers:** not supported as formal V1 connections.

Availability still depends on the installed CLI version, authentication state, account, subscription,
regional availability, quota, and Provider-side model access. Detecting a CLI does not prove that a live
reply will succeed. M.I.O. does not silently substitute a different model when an explicitly selected model
is unavailable.

### Codex workspace access

Codex participants start in **Chat only** mode. For a specific Room, the Owner may select one local folder
and grant either read-only or read/write access. The permission is stored locally for that Room and
participant; selecting a folder does not grant every AI access to the computer.

Workspace access is mediated by M.I.O.'s brokers and uses workspace-relative targets. Eligible Codex
profiles can list folders, read UTF-8 text files, create new files, or replace existing text files within the
selected boundary. M.I.O. does not expose unrestricted desktop control, arbitrary host paths, arbitrary
shell strings, deletion, or renaming as general AI tools.

Read/write Codex profiles may also use a fixed, bounded set of development operations, including Git status
and selected Node/npm build, test, and typecheck tasks. Installing an npm dependency is limited to one exact
lowercase package name and exact semantic version in an eligible project, and requires an Owner confirmation
for that action. A confirmation is scoped to the current Room, session, workspace, operation, and target; it
is not a permanent unrestricted terminal permission.

### Generated images

When a message to Codex explicitly requests an image, M.I.O. can display the returned image inside the Room.
The Owner can download it or save it to the selected editing folder after confirming the file name.
Composition ratio and quality controls are best-effort preferences: an image Provider may return different
pixel dimensions. Imported image artifacts are limited to 16 MiB.

### Backups and local data

Room names and conversation histories are stored on the device. **Back up all Rooms** writes a JSON backup
to a directory chosen by the Owner; M.I.O. does not upload that backup. Changing the backup directory does
not move older files.

Before restoring, M.I.O. shows the selected backup's file name, timestamp, and Room count. Restore replaces
the current Room catalog and conversation history only after review. It does not restore participant
profiles, appearance, Provider credentials, workspace permissions, model choices, or every dispatch record.

Product data is retained under `%APPDATA%\\app.moe.desktop`; WebView2 cache data may exist under
`%LOCALAPPDATA%\\app.moe.desktop`. Normal uninstall removes the installed application and shortcuts but
intentionally leaves this local data so that reinstalling or updating does not erase Rooms without warning.

### What is sent outside the device

M.I.O. itself has no public Remote Relay in V1 and does not operate an intermediary cloud service for these
CLI conversations. However, connected AI models are generally remote services.

- The current message and bounded Room context are sent only through the selected recipient or
  Conductor/worker route.
- A participant's display name and local guidance may be included to shape that participant's reply.
- Selected-folder content is available only to an eligible Codex participant with the matching Room
  permission.
- An image prompt and its preferences are sent only when that turn explicitly requests image generation.
- Passwords, OAuth codes, cookies, and API keys must not be entered into a Talk Room. Provider login happens
  in the official CLI outside M.I.O.

Provider inference, networking, retention, training use, subscriptions, quotas, and billing remain governed
by that Provider's current terms and account settings. Review them before sending private, confidential, or
paid-work material.

### System requirements and installation

- 64-bit Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- Administrator approval for the per-machine installation under `%ProgramFiles%\\M.I.O`
- Internet access when downloading WebView2, installing or authenticating a Provider CLI, sending a message
  to a Provider, or installing an explicitly confirmed npm package
- The official CLI and an eligible account only for each AI Provider you choose to use

Windows 10, Windows on Arm, 32-bit Windows, macOS, and Linux are not supported by V1.

M.I.O. does not buy, install, update, or authenticate Provider CLIs. Close M.I.O., follow each Provider's
official installation instructions, launch the CLI separately to complete login, and then restart M.I.O.

The direct itch.io download is an **Unsigned Preview** Windows x64 NSIS installer. It does not
have an Authenticode publisher signature, so Windows Defender SmartScreen may show a warning. Use the free
Microsoft Store edition if you prefer a package signed through Microsoft Store.

- File: `M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`
- Version: `1.0.4`
- Size: 4,516,598 bytes
- SHA-256: `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`
- Source commit: `fbc81b89ee64a670f13128487c06a96bfabe3238`
- Authenticode: `NotSigned` (no publisher or timestamp)

After downloading, verify that the file name and SHA-256 match the values above. Before an update, use Room
settings to back up all Rooms and close M.I.O. The installer requests Windows administrator confirmation
because it installs for all users under `C:\Program Files\M.I.O`. Existing Room data is not automatically
deleted when the installer updates or removes the application. V1 uses manual updates and does not include
an automatic updater.

### Known V1 limitations

- No public Remote Relay, multi-device synchronization, or multiple-account management
- No automatic updates or background automation
- No unlimited Conductor rounds or nested delegation
- No token-by-token streaming display
- No general workspace access for Claude Code, Gemini, or Grok
- No arbitrary shell, arbitrary path, unrestricted network, or general desktop-control tool
- Generated-image aspect ratio and quality are best-effort rather than exact pixel guarantees

### Current release status

M.I.O. v1.0.4 is available on itch.io as a free download with an optional donation. The direct-download
Unsigned Preview installer has passed local build, source comparison, Gitleaks, and Windows Defender checks.
The Microsoft Store edition remains a separately signed and distributed package.

### Release notes

#### v1.0.4 — September 7, 2026

Released on BOOTH and itch.io on September 7. Microsoft Store edition v1.0.4.0 was also confirmed available on September 8.

- Keeps an active AI reply running when you view another Room and shows progress in the Room list.
- Terminates Provider child processes on Stop so the Room can safely send again.
- Updates Codex connection status from live replies and distinguishes first-send waiting from Ready.
- Prevents Room switching, folder selection, profile saves, and Artwork Editor closure from leaving the app locked.
- Prevents late notifications from clearing a newer error or an in-progress state in another Room.
- Prevents helper processes from keeping M.I.O. running after the main window closes.
- Keeps the Japanese message-copy action on one line in the custom context menu.

#### v1.0.3 — September 6, 2026

- Added per-provider progress messages and elapsed-time feedback while replies are in progress.
- Improved status feedback and recovery after failed or cancelled Provider turns.
- Replaced native browser context menus in Talk Rooms and settings with M.I.O.-specific menus.
- Unified the direct download at version `1.0.3` and the Microsoft Store update candidate at `1.0.3.0`.
- Improved first-launch startup and Windows product naming and search visibility.

### Links

- Official page: https://tinmoon-label-site.pages.dev/works/mio/
- v1.0.4 Devlog: https://tinmoon-label.itch.io/mio-talk-room/devlog/1655617/mio-v104-is-now-available
- Source and releases: https://github.com/blackcometclub/M.I.O
- Full V1 user guide: https://github.com/blackcometclub/M.I.O/blob/main/docs/USER-GUIDE.md
- Bug reports: https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml
- Private security reports: https://github.com/blackcometclub/M.I.O/security/advisories/new

The public source is licensed under `AGPL-3.0-only`. Third-party dependencies and assets retain their own
licenses. The software license does not grant trademark rights in the M.I.O. name or branding.

The conversations shown in screenshots are capture-only demo content.

## Visual assets

- Cover image: `docs/assets/storefronts/mio-itch-cover-1260x1000.png` (`315:250` ratio)
- Screenshots: upload the existing five product screenshots
- Screenshot note: conversations shown in screenshots are capture-only demo content

Existing screenshots:

1. `docs/assets/screenshots/mio-talk-room.png`
2. `docs/assets/screenshots/mio-room-settings.png`
3. `docs/assets/screenshots/mio-preferences.png`
4. `docs/assets/screenshots/mio-appearance.png`
5. `docs/assets/screenshots/mio-compact-image-free.png`

## Confirm before publication

- [x] Microsoft Store link and exact candidate download file
- [x] File size and SHA-256
- [x] Signature status and publisher
- [x] Version `1.0.4` and publication date September 7, 2026; Store availability confirmed September 8
- [x] Update instructions
