# 0018: Claude Code / Claude Web connection status separation

- Status: Accepted
- Date: 2026-08-12

## Context

M.O.E.のRoomにはClaude CodeとClaude Webが別participantとして存在するが、ADR 0013以降のDesktop dispatchはCodex以外を一律 `unsupported` としていた。UIも名前とadapter種別だけを表示するため、ローカルCLIが検出済みのClaude Codeと、Remote MCP設定が必要なClaude Webと、未実装providerを区別できなかった。

個人利用の本来の目的は、Codex、Claude Code、Claude Webをそれぞれ独立した接続・session・返信元として同じRoomへ参加させることである。Claudeの契約がない期間にも製品境界とUIを実装できるが、実回答を確認していない接続を「接続済み」と表示してはならない。

## Decision

- Rust Desktopにboundedな `desktop_ai_connection_status` commandを追加し、known participantごとに `ready`、`installed`、`setupRequired`、`unsupported` を返す。
- Codexは製品launcherが解決できる場合だけ `ready` とする。これは「利用可能」であり、各turnの成功保証ではない。
- Claude Codeは `MOE_CLAUDE_BIN` またはWindows native installerの標準位置 `%USERPROFILE%/.local/bin/claude.exe` を検出する。CLI検出だけでは認証・契約・実回答を証明しないため `installed` とする。
- Claude WebはRemote MCP / Relayの常設pairingがまだないため `setupRequired` とする。Claude CodeのCLI状態を流用しない。
- Gemini等の未実装providerは `unsupported` とする。UIは状態ラベルと説明を表示し、ダミー返信を作らない。
- filesystem path、credential、account、plan、email、organization IDはWebViewへ返さない。

## Consequences

- 利用者は送信前にCodex、Claude Code、Claude Webの現在位置を区別できる。
- Claude再契約前でも、CLI検出とWeb接続設定待ちを別々に検証できる。
- Claude Codeのstructured stream実装とClaude WebのRemote MCP / Relay返信経路は後続trancheであり、実回答が通るまで状態を `ready` へ昇格しない。
