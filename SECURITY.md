# Security Policy

**English** | [日本語](SECURITY.ja.md)

## Supported versions

M.I.O. is preparing its first stable Windows release. Security fixes target the latest supported release; they are not guaranteed to be backported to older prereleases or development snapshots.

| Version | Supported |
|---|---|
| Latest `1.x` stable release, once published | Yes |
| Latest `1.0.0-rc.x`, until `v1.0.0` is published | Yes |
| `0.1.0-alpha.x` and older prereleases | No |
| Unreleased `main` branch snapshots | Best effort; not a supported release |

Users of an unsupported version should upgrade before reporting a version-specific defect. If upgrading is not possible, include that constraint in the report.

## Reporting a vulnerability

Do not describe vulnerabilities or possible credential exposure in a public Issue. Use [GitHub Private Vulnerability Reporting](https://github.com/blackcometclub/M.I.O/security/advisories/new) to send the report privately. If that form is unavailable, open a public Issue containing only the title `Private security contact requested` and ask the repository owner for a private contact method. Do not include the vulnerability, reproduction details, logs, or secrets in that Issue.

Reports are especially welcome for:

- Credentials or tokens exposed to the WebView, logs, or evidence artifacts
- Tauri command or IPC authorization-boundary bypasses
- Relay or MCP authentication, pairing, or request-correlation defects
- Out-of-scope file access or path traversal
- Unintended external transmission, tool execution, or approval bypasses

Include the impact, the minimum steps needed to reproduce the issue, and the commit you tested. Do not attach real API keys, tokens, cookies, or personal information. If a secret may have been exposed, revoke and replace it immediately rather than waiting for the report to be reviewed.

## What happens after a report

The maintainer will review the report privately, try to reproduce the issue, and determine whether it affects M.I.O. itself or an external Provider, CLI, or service. Valid M.I.O. issues will be fixed on the latest supported version when practical. A security advisory, mitigation, or release note will be prepared before public details are disclosed when coordinated disclosure is possible.

M.I.O. is maintained without a security-response SLA or bug-bounty program. Response and remediation times depend on severity, reproducibility, maintainer availability, and upstream fixes. Reports made in good faith that avoid privacy violations, service disruption, persistence, and access beyond the minimum needed to demonstrate the issue are welcome.

## Current constraints

Content under `spikes/` consists of proofs of concept for evaluating connection methods. It does not guarantee product quality or operation of a public server. Before running a probe that involves external publication or a real account, review its README for prerequisites and data-transmission scope, and use isolated test data only.
