# Claude Code structured stream spike

Test 0-Bで、Claude Code CLIをProvider固有のsubprocess Adapter候補として検証します。製品CoreへClaude Code固有eventを持ち込みません。

## Handshake preflight

```powershell
npm.cmd run probe:handshake --workspace @moe/spike-claude-code-stream
```

Windows native installerの標準位置 `%USERPROFILE%\.local\bin\claude.exe` を探索します。別の実行ファイルは `MOE_CLAUDE_BIN` で指定できます。

probeは `-p`、Fable、`stream-json`、partial messagesを使います。tool surfaceは空、permission modeは `dontAsk`、customizationは `--safe-mode` で無効化します。`--bare` はOAuth/keychainを読まないため、既存Claude.aiログインを検証するこのprobeでは使用しません。

成功時は固定marker、session ID、stream eventを実行中に確認します。Anthropic側がprogrammatic利用を拒否した場合は `BLOCKED` とし、session ID、契約plan、token、email、organization IDを証拠へ保存しません。sanitized summaryは `evidence/handshake-latest.json` へ保存します。

参照した公式文書:

- https://code.claude.com/docs/en/headless
- https://code.claude.com/docs/en/cli-usage
- https://code.claude.com/docs/en/authentication
- https://code.claude.com/docs/en/permission-modes
