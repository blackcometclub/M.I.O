# M.I.O.

**English** | [日本語](README.ja.md)

**M.I.O. (Malevolent Immortal Overdrive)** is a local-first Windows desktop app that brings multiple AIs into one Talk Room, where they can collaborate through direct conversations or Conductor mode.

> [!IMPORTANT]
> **M.I.O. v1.0.5 is publicly available.** Microsoft Store distributes the signed `1.0.5.0`
> package, confirmed public and updated on a physical PC on September 9, 2026. BOOTH and itch.io
> distribute the separate unsigned `1.0.5` Preview released the same day. The current source version
> is `1.0.5`. The published application was built from `3076294e1d87821afd1e18f36420d9162820679e`;
> subsequent release-documentation updates do not change the application code.

- [Microsoft Store](https://apps.microsoft.com/detail/9NS9B7T71XHN) — recommended signed package
- [BOOTH](https://tinmoon.booth.pm/items/8807279) — free ZIP with optional support; unsigned preview
- [itch.io](https://tinmoon-label.itch.io/mio-talk-room) — free/pay-what-you-want unsigned preview installer
- [GitHub v1.0.5](https://github.com/blackcometclub/M.I.O/releases/tag/v1.0.5) — history-free source ZIP and checksums
- [What's new in v1.0.5](https://tinmoon-label-site.pages.dev/en/works/mio/#release-notes) — release notes in English

The published [alpha.2 Release](https://github.com/blackcometclub/M.I.O/releases/tag/v0.1.0-alpha.2)
and earlier releases remain immutable historical artifacts. They are not replaced or repurposed as
the V1 download; use one of the current distribution channels above.

## Screenshots

![English M.I.O. Talk Room with Codex, Claude Web, and Gemini in a capture-only UI demo](docs/assets/screenshots/mio-talk-room.png)

This screenshot shows a capture-only Room with a fictional conversation. The displayed AI responses
are illustrative text for explaining the product and are not actual Provider responses.

<details>
<summary>View Preferences, appearance, and Room settings</summary>

### Preferences

![English M.I.O. Preferences screen](docs/assets/screenshots/mio-preferences.png)

### Appearance

![English M.I.O. appearance settings](docs/assets/screenshots/mio-appearance.png)

### Room settings

![English M.I.O. Room settings](docs/assets/screenshots/mio-room-settings.png)

</details>

## What M.I.O. can do

- Let multiple AI participants and a human talk in the same Talk Room
- Send a message to explicitly selected recipients in **Direct mode**
- Let Codex answer directly or delegate one round to up to three workers in **Conductor mode**
- Create and rename Rooms, manage participating AIs, and persist message history
- Store participant display names, avatars, local AI guidance, and supported access modes on the device
- Back up all Rooms as safe JSON to a user-selected directory and restore a reviewed backup after confirmation
- Give Codex chat-only, selected-folder read, or selected-folder read/write access
- Let Codex run a small fixed set of bounded development operations; package installation and other sensitive operations require an Owner confirmation
- Choose a verified Codex or Claude Code model per participant profile
- Cancel an active Provider turn from the Send button
- Display Codex-generated images, download them, or save them under the selected editing folder after confirming the file name
- Start bounded local MCP tools on loopback only when a token is configured

Conductor mode does not create an automatic or unlimited chain of work. Codex is the first and only
supported conductor. Each Owner request is limited to one round and at most three workers. Direct mode
and Conductor mode can be selected again from the Room screen.

## Connection status

| Provider | V1 scope | Access |
|---|---|---|
| Codex | Supported | Conversation, Conductor, verified model selection, generated images, selected-folder read/write, and bounded commands through the local Codex CLI |
| Gemini Antigravity | Supported | Conversation-only responses through the local CLI |
| Claude Code (display name may be changed) | Supported | Conversation-only responses and verified model selection through the local Claude CLI |
| Grok | Supported | Conversation and, only when selected by the Owner, read-only review of tracked Git changes through the local CLI |
| Claude Web | Not connected | Remote MCP/Relay remains research work and is not a supported product connection |
| Google Search / AI Mode Browser Bridge | Experimental | A recreational proof of concept, disabled in normal builds |
| ChatGPT Web / OpenAI API | Not currently supported | Cannot be selected in the UI |
| Generic MCP client / Custom adapter | Not currently supported | Cannot be selected in the UI |

“Local CLI” means an adapter that M.I.O. detects and launches on the user's Windows device. It does not
mean that model inference happens only on that device. Conversations sent to an AI may be subject to the
Provider's network, terms, subscription, billing, retention, and storage policies. Users are responsible
for reviewing each CLI's installation, authentication, and usage conditions.

M.I.O. is designed to continue starting even when a CLI is missing, unauthenticated, or otherwise
unavailable. Unavailable participants show their status, and M.I.O. does not fabricate replies from an AI
that is not connected.

## Safety boundaries

- Start an external turn only once for each source message and recipient; do not retry automatically when the outcome is unknown
- Persist Room and dispatch state; do not report partial results or unknown outcomes as success
- Run the desktop app as a single instance to avoid multiple writers for the same Room data
- Constrain Codex workspace access to one selected folder through M.I.O.'s path and file-operation brokers
- Offer only fixed command tools; require action-time confirmation where specified and deny arbitrary command strings, delete, rename, and unrestricted network access
- Do not grant workspace read/write to Fable or Gemini. When the Owner selects it in the participant profile, Grok may receive only M.I.O.-prepared Git status and tracked changes for read-only review
- Do not start local MCP without a token, and never bind it outside loopback
- Do not expose arbitrary shell access or credential values to the WebView

See the [V1 readiness checklist](docs/V1-READINESS.md) for the current release boundary and validation evidence.
The historical alpha boundary remains recorded in [ADR 0037](docs/decisions/0037-mio-public-alpha-release-boundary.md).

## Current limitations

- Operating systems other than Windows are not supported
- Fable and Gemini are conversation-only. Grok workspace access is limited to read-only review of tracked Git changes and status
- Codex workspace tools cover bounded UTF-8 text-file and development operations; they are not unrestricted desktop or shell access
- Exact pixel dimensions for generated images are not supported; aspect ratio and quality are best-effort preferences, and imported images are limited to 16 MiB
- Token-by-token streaming is not displayed even though an active Provider turn can be cancelled
- Public Remote Relay, multiple devices, and multiple accounts remain research work
- Background automation, unlimited conductor rounds, and nested delegation are not supported
- Automatic updates are not included. The Microsoft Store package is signed through Microsoft Store; BOOTH and itch.io direct downloads are explicitly unsigned previews

## System requirements

M.I.O. V1 supports 64-bit Windows 11. Windows 10 was used during alpha development but is not guaranteed
for V1 because the final release candidate has not been validated there. M.I.O. requires the Evergreen version of
[Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
This is the shared Runtime used by Windows desktop apps to display their UI, not the Microsoft Edge
browser itself.

For installation, first launch, manual update, backup, data transmission, uninstall, and troubleshooting,
see the [V1 user guide](docs/USER-GUIDE.md).

A source checkout can build an unsigned validation installer that uses Tauri's WebView2 download
bootstrapper. This local artifact is not the Microsoft Store-signed package or an official storefront
download.
If you run only the standalone executable and the Runtime is missing, install the Evergreen Runtime from
Microsoft's official download page.

The Codex, Gemini, Claude, and Grok CLIs are not required to start M.I.O. Install and authenticate only
the CLIs for the AIs you intend to use. Gate 3 startup validation in an isolated Windows environment with
none of these CLIs installed is complete. The evidence is recorded in the
[public readiness checklist](docs/PUBLIC-ALPHA1-READINESS.md).

### Installing and authenticating Provider CLIs

M.I.O. does not install or update CLIs, perform login, or enter credentials on the user's behalf. With
M.I.O. closed, install only the Providers you intend to use and launch each CLI separately to complete
its official login flow. In organizations where install scripts cannot be executed directly, follow the
linked official documentation and the organization's software installation policy.

#### Codex

Follow the [official OpenAI Codex CLI instructions](https://learn.chatgpt.com/docs/codex/cli). On Windows,
run the standalone installer from PowerShell 7. Do not use Windows PowerShell 5.1: during physical-device
validation on August 23, 2026, the installer available at that time stopped because it could not read
`OSArchitecture`.

```powershell
winget install --id Microsoft.PowerShell --source winget
pwsh -NoProfile -Command "irm https://chatgpt.com/codex/install.ps1 | iex"
codex --version
codex
```

On first launch, choose `Sign in with ChatGPT` or the API key method described in the official
instructions. M.I.O. detects `codex` from `PATH` or its standard installation location and launches
`codex app-server`.

For each Codex participant, choose chat only, read selected folder, or read and edit selected folder.
Read/write access is mediated by M.I.O. and does not expose the host workspace path to the AI. Text-file
creation and safe replacement are supported in write mode. A fixed set of Node/npm/Git-status operations
is available only in that mode, with Owner confirmation where required. M.I.O. does not modify Codex's
`config.toml`.

#### Gemini Antigravity

Follow [Google's official Antigravity CLI instructions](https://codelabs.developers.google.com/antigravity-cli-hands-on)
and install it from Windows PowerShell.

```powershell
irm https://antigravity.google/cli/install.ps1 | iex
agy --version
agy
```

Complete Google login on first launch. M.I.O. detects `agy` from `PATH` or
`%LOCALAPPDATA%\agy\bin\agy.exe` and uses it in a non-interactive, conversation-only mode.

#### Claude Fable

Follow [Anthropic's official Claude Code instructions](https://code.claude.com/docs/en/installation) and
install it from Windows PowerShell.

```powershell
winget install Anthropic.ClaudeCode
claude --version
claude
```

Complete browser login on first launch. M.I.O. detects `claude` from `PATH` or
`%USERPROFILE%\.local\bin\claude.exe`, disables tools, and uses a conversation-only mode. The profile can
use the Provider default or one of the verified Claude Code model identifiers shown by M.I.O.; unavailable
models are not reported as successful replies.

#### Grok

Follow [xAI's official Grok CLI instructions](https://docs.x.ai/build/overview) and install it from
Windows PowerShell.

```powershell
irm https://x.ai/cli/install.ps1 | iex
grok --version
grok
```

Complete browser login on first launch. M.I.O. detects `grok` from `PATH` or
`%USERPROFILE%\.grok\bin\grok.exe`, disables web search, memory, subagents, and tools, and requests
`grok-4.6` for a one-turn conversation response. When read-only access is selected in the participant
profile, M.I.O.'s broker supplies only Git status and tracked changes for review. It does not supply
untracked file bodies, the workspace's absolute host path, edits, commands, or web access. M.I.O. does
not report success when that model is unavailable under the user's subscription.

After installation and login, restart M.I.O. so that it reads the new `PATH` and credential state. Do
not paste a password, OAuth code, or API key into M.I.O. A detected CLI is not shown as connected until
its first live reply succeeds. Because sent content is subject to each Provider's network, contract,
billing, and retention policies, live validation records the CLI version, authentication method,
subscription, and retention policy—without credential values—in the
[public readiness checklist](docs/PUBLIC-ALPHA1-READINESS.md).

## Development environment

Windows is the current target. Development and validation from source require:

- Node.js 24 or later
- npm 11.11.x
- Rust 1.96.x (pinned by `rust-toolchain.toml`)
- The “Desktop development with C++” workload from Microsoft C++ Build Tools
- Microsoft Edge WebView2

Install dependencies exactly as locked, then launch the Tauri development app.

```powershell
npm.cmd ci
npm.cmd run dev
```

Build a development binary with:

```powershell
npm.cmd run tauri:build
```

`tauri:build` is a `--no-bundle` build for development validation. It does not create an installer.

Build a Windows x64 release-validation executable with the dedicated script below. The legacy script name
is retained for compatibility. It builds the
frontend, creates a release executable with the Visual C++ Runtime linked statically, and prints its
SHA-256 hash. It does not bundle an installer or the WebView2 Runtime.

```powershell
& .\scripts\build-alpha-windows.ps1
```

The executable is written to `target/x86_64-pc-windows-msvc/release/moe-desktop.exe`.

Build an unsigned Windows x64 NSIS validation installer with:

```powershell
& .\scripts\build-alpha-windows.ps1 -Installer
```

The installer uses a per-machine installation under `%ProgramFiles%\M.I.O` and asks for Administrator
approval through Windows UAC. This makes M.I.O. discoverable in Windows Settings and Programs and
Features for normal maintenance and uninstall. If WebView2 is missing, it uses Tauri's download bootstrapper and therefore
requires an internet connection during installation. The output is written to
`target/x86_64-pc-windows-msvc/release/bundle/nsis/`. This is a local validation artifact; it is unsigned
and is not automatically attached to a GitHub Release. The installer build also compiles and bundles the
fixed-request `moe-command-helper.exe` sidecar. Bundling the helper does not enable Room command tools;
the desktop host embeds its build-time SHA-256 and accepts only the exact, non-linked sibling binary with
the same hash. Command tools are enabled only for an eligible Codex workspace-write profile and remain
subject to their fixed request shape and confirmation rules. The installer shows the project license and
places `THIRD-PARTY-NOTICES.txt` in the installation directory.

Prepare the V1 source snapshot, installer, checksums, secret-scan result, and release plan from a committed state with:

```powershell
& .\scripts\prepare-public-release.ps1 -Commit HEAD
```

The export is rejected if the versions in the root package, desktop package, Tauri configuration, and
Rust workspace do not match. For safety, it is also rejected when tracked working-tree or staged changes
exist.

## Validation commands

```powershell
npm.cmd run typecheck
npm.cmd run build
cargo fmt --all -- --check
cargo test --workspace
```

Automated CI does not perform tests that require Provider login, external publication, or writes to OS
credential storage. The execution requirements for individual proofs of concept are documented in the
READMEs under `spikes/`.

## Repository layout

```text
apps/
  desktop/            M.I.O. Windows desktop UI and Tauri backend
  relay/              Placeholder for a future Remote Relay
crates/
  moe-core/           Provider-neutral Rust core
  moe-protocol/       Neutral Rust data contract
  moe-adapter-sdk/    Rust adapter boundary
  moe-credential-store/ OS credential storage boundary
packages/             TypeScript boundaries
adapters/             Planned location for Provider-specific implementations
spikes/               Short-lived PoCs and sanitized evidence isolated from product code
docs/
  architecture/       Unresolved design proposals and analysis
  decisions/          Explicitly adopted decisions (ADRs)
```

Internal crate names, package names, environment variables, and app data retain the `moe` identifier to
preserve compatibility with existing data and development boundaries. This does not indicate an intent
to return to the former product name.

Historical documents written before ADR 0037 may retain the former product name `M.O.E.` as it existed
when those decisions were made. The current user-facing product name is `M.I.O.`. Adopted decisions,
their rationale, and their consequences are recorded in the [ADRs](docs/decisions/).

## Public project information

- Bug reports: [open the bug report form](https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml)
- Feature proposals: [open the proposal form](https://github.com/blackcometclub/M.I.O/issues/new?template=feature_request.yml)
- Contribution policy: [CONTRIBUTING.md](CONTRIBUTING.md)
- Support policy: [SUPPORT.md](SUPPORT.md)
- Security reports: [SECURITY.md](SECURITY.md)
- Current V1 release notes: [docs/RELEASE-NOTES-1.0.0.md](docs/RELEASE-NOTES-1.0.0.md)
- Documentation guide: [docs/README.md](docs/README.md)

## License

Copyright (c) 2026 blackcometclub.

Except where otherwise noted, the public code is available under the
[GNU Affero General Public License v3.0 only](LICENSE) (`AGPL-3.0-only`). The copyright holder may offer
a separate commercial license for proprietary products and services that cannot comply with the AGPL.
See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md) for details.

Third-party dependencies and assets remain subject to their respective licenses. The software license
does not grant trademark rights in the M.I.O. name or branding.

See [Third-party notices](THIRD-PARTY-NOTICES.md) for the review status of third-party assets, including
Pixelify Sans, and dependency licenses.
