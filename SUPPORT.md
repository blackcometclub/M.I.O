# M.I.O. support policy

**English** | [日本語](SUPPORT.ja.md)

## Supported scope

M.I.O. V1 support covers the latest published `1.x` stable release on 64-bit Windows 11. Until `v1.0.0` is published, only the latest `1.0.0-rc.x` is considered for release-candidate reports. Older alpha releases, development snapshots, Windows 10, Windows on Arm, 32-bit Windows, macOS, and Linux are not supported.

Support includes reproducible M.I.O. defects involving installation, startup, Rooms, Provider dispatch, backup/restore, bounded Codex workspace operations, update, and uninstall. It does not include operating a Provider account, obtaining a subscription or quota, changing Provider retention/billing, or repairing an independently installed Provider CLI.

## Where to ask

- Reproducible product defect: [bug report form](https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml)
- Feature request: [feature proposal form](https://github.com/blackcometclub/M.I.O/issues/new?template=feature_request.yml)
- Security vulnerability or possible credential exposure: follow [SECURITY.md](SECURITY.md), not a public Issue
- Provider outage, account, model availability, billing, or CLI installer defect: use that Provider's official support channel

M.I.O. is maintained without a response-time or resolution-time SLA. A report may be closed or redirected when it cannot be reproduced, concerns an unsupported version, belongs to an upstream Provider, or requires access to private credentials or data.

## Information to include

- M.I.O. version and installer/source origin
- Windows edition, version, and architecture
- A short reproduction sequence and the expected/actual result
- The affected Provider and CLI version, if relevant
- Whether the Room used chat-only, read, or read/write permission
- Redacted error text and whether M.I.O. reported failed, cancelled, timed out, or unknown delivery

Do not post Room content, private files, passwords, OAuth codes, API keys, cookies, access tokens, personal information, or full credential-bearing logs. Use a minimal test Room and replace sensitive paths and names before attaching evidence.

## Updates and end of support

V1 uses manual installer updates. Security and product fixes target the latest supported release; backports to an older stable or prerelease are not guaranteed. Once a newer stable patch is available, users may be asked to reproduce the problem there before further investigation.

See the [V1 user guide](docs/USER-GUIDE.md) for installation, update, data, uninstall, and troubleshooting instructions.
