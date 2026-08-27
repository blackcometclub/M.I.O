# 0002: Provider-agnostic product scope

Status: Accepted

Date: 2026-08-11

## Context

最初の検討資料はClaude Web、Codex、Claude Codeをつなぐ現在の開発フローを中心にしていました。しかしM.O.E.の最終目的は、この3者専用の配送ツールではありません。

## Decision

M.O.E.を、Role、Provider、Model、Adapter instance、Transportを分離したマルチモデル接続環境とします。

製品スコープには、少なくとも次を含めます。

- Claude Web
- Claude Code
- Codex
- ChatGPT
- Grok
- Gemini
- OpenAI API
- Generic MCP
- 将来追加されるその他の接続先

## Consequences

- 初期のWeb Opus / Codex / Fable構成はpresetであり、Coreの固定参加者にはしません。
- Adapterは対応能力と制限を申告し、全接続先へ同一機能を偽装しません。
- Web UI、local agent、direct API、MCPの接続方式を区別します。
- 新しいProviderはCore変更ではなく、原則としてAdapter追加とcontract testで導入できる構造を目指します。
- 各接続先の公式経路、利用可能性、双方向性は、個別のPoCと最新公式資料で確認します。
