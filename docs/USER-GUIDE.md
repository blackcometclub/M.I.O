# M.I.O. V1 user guide

**English** | [日本語](USER-GUIDE.ja.md)

This guide describes the public Windows V1 line. The signed Microsoft Store package is the recommended
installation. BOOTH and itch.io provide a separate unsigned `v1.0.5` preview for people who deliberately
choose direct download. A locally built installer is not an official storefront download.

## 1. Requirements

- 64-bit Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- Internet access only when downloading WebView2, installing or authenticating a Provider CLI, sending a message to a Provider, or installing an explicitly confirmed npm package
- A Provider CLI only for each AI you choose to use; M.I.O. itself starts without any Provider CLI

Windows 10, Windows on Arm, 32-bit Windows, macOS, and Linux are not supported by V1.

## 2. Install and first launch

1. Recommended: open the [Microsoft Store page](https://apps.microsoft.com/detail/9NS9B7T71XHN), select
   **Get** or **Install**, and let Microsoft Store verify and install the signed package.
2. For direct download, use only the public [BOOTH](https://tinmoon.booth.pm/items/8807279) ZIP or
   [itch.io](https://tinmoon-label.itch.io/mio-talk-room) installer. These files are explicitly marked
   **Unsigned Preview**. Compare the SHA-256 shown on the same storefront page before running the installer:

   ```powershell
   Get-FileHash .\M.I.O_1.0.5_windows-x64_unsigned-preview_setup.exe -Algorithm SHA256
   ```

3. A Microsoft Store installation is signed through Microsoft Store. The BOOTH/itch.io preview has no
   Authenticode publisher and may trigger SmartScreen; if you do not accept that boundary, use Microsoft Store.
4. For a direct install, close M.I.O., run the installer, and approve Windows UAC. It installs for all
   users under `%ProgramFiles%\M.I.O` and requires Administrator approval.
5. If WebView2 is missing, the direct installer downloads Microsoft's official bootstrapper. An internet
   connection is required for that step.
6. Start **M.I.O.** from the Start menu. Create or select a Room, add only the AIs you want, and leave each
   participant in **Chat only** unless local-folder access is needed.

The direct installer includes the project license and `THIRD-PARTY-NOTICES.txt`.

## 3. Install and sign in to Provider CLIs

M.I.O. does not install a Provider CLI, enter credentials, or complete login for you. Close M.I.O., follow the Provider's official instructions linked from the [README](../README.md#installing-and-authenticating-provider-clis), and launch the CLI by itself to finish its login flow. Then restart M.I.O.

- Never paste a password, OAuth authorization code, cookie, or API key into a Talk Room.
- CLI detection alone does not prove that the account, model, subscription, or quota is usable. The participant becomes ready only after an appropriate live reply succeeds.
- If an explicit model is unavailable, M.I.O. does not silently treat a different model as the requested one.

## 4. Manual update

V1 does not include automatic updates.

1. In **Room settings**, choose a backup directory and select **Back up**.
2. Close M.I.O.
3. Microsoft Store users should update through Microsoft Store. Direct-download users should obtain the
   newer file from the same BOOTH or itch.io page and compare its published SHA-256 before running it.
4. If the existing version is a current-user alpha under `%LOCALAPPDATA%\M.I.O`, uninstall that application
   without deleting its retained data before installing V1. An existing per-machine direct V1 can normally
   be updated in place. Do not switch between Store and direct-install channels as if they were the same
   package; uninstall the current app without deleting retained data before deliberately changing channels.
5. Start M.I.O. and confirm the Room list, participant profiles, appearance, workspace permissions, selected models, and recent conversation history.

Stop if the release notes describe a migration that does not match the installed version, a Store signature
is invalid, or a direct-download hash does not match its storefront page.

## 5. Backups and local data

M.I.O. stores product data under `%APPDATA%\app.moe.desktop`. WebView2 cache data may also exist under `%LOCALAPPDATA%\app.moe.desktop`.

The **Back up all Rooms** operation writes Room names and conversation histories as JSON. The default directory is `Documents\M.O.E Backups`; a different directory can be selected in Room settings. Changing the directory does not move existing backup files.

Before restoring, M.I.O. shows the latest backup file, timestamp, and Room count. Restore replaces the current Room catalog and conversation history with the reviewed backup. It does not restore participant profiles, appearance, Provider credentials, workspace permissions, model selections, or every dispatch record.

Keep important backups on a different drive or in another location you control. M.I.O. does not upload these backup files.

## 6. What is sent to an AI

M.I.O. is local-first, but connected AI models are generally remote services.

| Data | Boundary |
|---|---|
| Current message and bounded Room context | Sent only to the explicitly selected recipients or the selected Conductor/worker route |
| Participant display name and local AI guidance | Sent when needed to shape that participant's reply |
| Selected-folder content | Available only to Codex when that participant has read or read/write permission; access is brokered and limited to the selected folder |
| Fixed development operations | Available only to an eligible Codex read/write profile; sensitive operations request Owner confirmation |
| Image-generation prompt and preferences | Sent to Codex only when the current message explicitly asks for an image |
| Passwords, OAuth codes, API keys, or Provider cookies | Must not be entered in M.I.O.; Provider CLI login happens outside M.I.O. |

Provider inference, subscriptions, quotas, billing, retention, training use, and account rules are controlled by that Provider and CLI, not by M.I.O. Review the Provider's current terms and account settings before sending private or paid-work content. M.I.O. has no public Remote Relay in V1 and does not operate an intermediary cloud service for these CLI conversations.

## 7. Uninstall

1. Back up Rooms if you may need them later.
2. Close M.I.O.
3. Open **Windows Settings → Apps → Installed apps → M.I.O → Uninstall** and complete the confirmation.

The uninstaller removes the installed application, bundled command helper, third-party notice, shortcuts, and uninstall registration. It intentionally leaves device-local product data and WebView2 cache so an update or reinstall does not erase Rooms without warning.

To remove the remaining data permanently, first verify your backup and then delete `%APPDATA%\app.moe.desktop` and `%LOCALAPPDATA%\app.moe.desktop` yourself. This cannot be undone by M.I.O. Provider CLIs and their credentials are separate applications and are not removed.

## 8. Troubleshooting

### A Provider CLI is not detected

Close M.I.O., confirm the CLI works in a new PowerShell window with its `--version` command, then restart M.I.O. A terminal opened before installation may still have the old `PATH`.

### Installed but not ready

Launch the CLI directly and finish its official login. Confirm that the selected model and subscription are available. Do not paste the login result into M.I.O.

### Timeout or unknown result

Check the Provider account and network. When M.I.O. says delivery is unknown, it does not retry automatically because doing so could create a duplicate turn. Review the Provider side before sending a new message.

### SmartScreen warning or signature problem

The Microsoft Store package is signed through Microsoft Store. The separate BOOTH/itch.io installer is an
unsigned preview and can trigger SmartScreen; verify its SHA-256 against the same storefront page. If an
expected Store signature is invalid, or if the direct-download hash differs, do not run the file.

### WebView2 is missing

Reconnect to the internet and rerun the official installer so its Microsoft WebView2 bootstrapper can finish, or install the Evergreen Runtime from [Microsoft's WebView2 page](https://developer.microsoft.com/microsoft-edge/webview2/).

### Restore is unavailable

Confirm that the displayed backup directory still exists and contains an unchanged `moe-room-backup-*.json` file. Use **Review restore** again; do not rename or edit a backup between review and restore.

## 9. Known V1 limits

- No public Remote Relay, multiple-device synchronization, or multiple-account management
- No automatic updates, background automation, unlimited Conductor rounds, or nested delegation
- No workspace read/write for Claude Code or Gemini. Grok is limited to Owner-selected read-only review
  of tracked Git status and changes; Codex has no unrestricted shell or desktop control
- No token-by-token streaming display
- Generated-image aspect ratio and quality are best-effort; exact pixel dimensions are not guaranteed and imported images are limited to 16 MiB
