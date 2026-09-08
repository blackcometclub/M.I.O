# M.I.O. V1 release notes — current source v1.0.4

**Status: public.** Microsoft Store currently distributes signed package `1.0.3.0`. BOOTH and itch.io
distribute the separate unsigned `v1.0.4` direct-download preview. Microsoft Store update package
`1.0.4.0` is under certification as Submission 4; the published `1.0.3.0` package remains available during
that review.

## Highlights

- Persistent Talk Rooms for one Owner and multiple AI participants
- Explicit Direct recipients and one-round, maximum-three-worker Codex Conductor mode
- Codex chat-only, selected-folder read, and selected-folder read/write profiles
- Bounded Git status, Node, npm build/test/typecheck, and Owner-confirmed exact npm package installation
- Verified model selection for Codex and Claude Code; Provider default for Gemini
- Active-turn cancellation without automatic retry of unknown delivery
- Codex-generated image display, download, editing-folder save, and best-effort aspect/quality preferences
- User-selected Room backup directory and reviewed restore

## Public v1.0.3 direct-download update

- Explains confirmed failure and unknown delivery without silently retrying a possibly duplicated turn
- Shows per-AI progress and reliably returns the Room to an operable state after failure or cancellation
- Improves Codex reconnection guidance and avoids selecting unavailable CLI candidates
- Adds M.I.O.-specific context menus and message copy while suppressing irrelevant browser menus

## Public v1.0.4 direct-download update

- Shows per-AI progress and keeps an active turn running when the user views another Room
- Makes Stop terminate the Provider process tree and reliably returns the Room to an operable state
- Improves Codex runtime detection and reports **Ready** only after a live response succeeds
- Prevents a missed native folder-choice event from leaving the application permanently locked
- Prevents mixed settings after a Room switch, late-notification state loss, and locks after profile-save failure
- Prevents helper processes from keeping M.I.O. alive after the main window closes
- Keeps the Japanese message-copy action on one line in the custom context menu

## Supported environment

- 64-bit Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- Per-machine installation under Program Files; Windows UAC Administrator approval is required

Windows 10 and non-Windows platforms are not guaranteed by V1.

## Provider scope

| Provider | V1 capability |
|---|---|
| Codex | Conversation, Conductor/worker, verified model selection, generated images, selected-folder read/write, bounded commands |
| Claude Code | Conversation/worker and verified model selection; no workspace or tools |
| Gemini Antigravity | Conversation/worker with Provider-default model; no workspace or tools |
| Grok | Conversation and bounded read-only review of tracked Git changes without untracked-file content or host paths |

Claude Web, public Remote Relay, generic/custom Providers, multiple devices, and multiple accounts are not V1 connections.

## Safety boundary

- Unknown Provider delivery is not retried automatically
- Workspace access is constrained to the selected folder through M.I.O. brokers
- Arbitrary shell strings, arbitrary paths, delete/rename, and unrestricted network access are not exposed
- Sensitive fixed operations require a matching action-time Owner confirmation
- Provider credentials are entered through the Provider CLI outside M.I.O.

## Install and update

Back up Rooms and close M.I.O. before changing the installed version. Microsoft Store users update through
Microsoft Store. Direct-download users verify the published SHA-256 of the unsigned installer before
running it. If an older alpha was installed for the current user, uninstall that application without
deleting its retained data before installing V1. Later direct V1 updates can run over the existing
per-machine direct installation unless their release notes explicitly require another procedure.

See the [V1 user guide](USER-GUIDE.md) for detailed steps and data-retention behavior.

## Distribution channels

- [Microsoft Store](https://apps.microsoft.com/detail/9NS9B7T71XHN): recommended signed package,
  currently `1.0.3.0`
- [BOOTH](https://tinmoon.booth.pm/items/8807279): free ZIP with optional support, containing the
  unsigned `v1.0.4` preview installer and checksum instructions
- [itch.io](https://tinmoon-label.itch.io/mio-talk-room): free/pay-what-you-want unsigned `v1.0.4`
  preview installer, with the [English update log](https://tinmoon-label.itch.io/mio-talk-room/devlog/1655617/mio-v104-is-now-available)
- [GitHub repository](https://github.com/blackcometclub/M.I.O): source and development history. The
  older alpha Releases remain immutable historical artifacts rather than the current V1 installer channel

## Known limitations

- No automatic update, public Remote Relay, multi-device sync, or multiple-account management
- No workspace tools for Claude Code, Gemini, or Grok beyond any separately gated bounded Git review
- No token-by-token streaming display
- Generated-image dimensions are not exact; imported artifacts are limited to 16 MiB
- The separate unsigned BOOTH/itch.io preview can trigger SmartScreen; use Microsoft Store when a signed
  package is required

## Current update status

- Microsoft Store `1.0.4.0` is under certification as Submission 4; publishing or cancelling that submission remains a
  separately approved Store operation
- BOOTH and itch.io published the unsigned `v1.0.4` direct-download preview on September 7, 2026
- Direct-download installers are not Authenticode-signed. Users who require a signed package should use
  Microsoft Store
