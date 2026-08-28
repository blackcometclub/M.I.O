# Security Policy

**English** | [日本語](SECURITY.ja.md)

## Supported versions

M.I.O. is currently a research alpha for Windows and does not have a stable release. `v0.1.0-alpha.2` was published as a source-first GitHub Prerelease on August 28, 2026. Security fixes generally target the latest `main` branch, and no remediation deadline or service-level agreement (SLA) is currently provided.

## Reporting a vulnerability

Do not describe vulnerabilities or possible credential exposure in a public Issue. Use the repository's **Security** tab to report a vulnerability privately. If private vulnerability reporting is unavailable, do not publish reproduction details or secrets; ask the repository owner for a private contact method.

Reports are especially welcome for:

- Credentials or tokens exposed to the WebView, logs, or evidence artifacts
- Tauri command or IPC authorization-boundary bypasses
- Relay or MCP authentication, pairing, or request-correlation defects
- Out-of-scope file access or path traversal
- Unintended external transmission, tool execution, or approval bypasses

Include the impact, the minimum steps needed to reproduce the issue, and the commit you tested. Do not attach real API keys, tokens, cookies, or personal information. If a secret may have been exposed, revoke and replace it immediately rather than waiting for the report to be reviewed.

## Current constraints

Content under `spikes/` consists of proofs of concept for evaluating connection methods. It does not guarantee product quality or operation of a public server. Before running a probe that involves external publication or a real account, review its README for prerequisites and data-transmission scope, and use isolated test data only.
