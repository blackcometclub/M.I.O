# Changelog

M.I.O. (Malevolent Immortal Overdrive) の主な変更を記録します。

## v0.1.0-alpha.1 — Public release candidate

Status: 履歴を分離した公開repository候補で検査完了。GitHub Prereleaseを再公開準備中。安定版ではありません。

最初のsource-first公開候補です。複数のローカルAIをひとつのTalk Roomへ集め、直接会話または指揮者モードで協働させるWindows向け研究用α版として区切ります。

### Added

- Tauri 2 / React / TypeScriptによるWindows Talk Room UI
- Room作成、名前変更、参加AI管理、永続message履歴
- 参加者の表示名、avatar、AI向けローカル案内、端末内設定
- 全RoomのJSON backupと、二段階確認付きrestore
- Codex、Gemini Antigravity、Claude Fable、GrokのローカルCLI会話adapter
- 明示した宛先へ送るDirect mode
- Codexが1 round・最大3 workerまでを扱うConductor mode
- Codexのchat-only接続と、workspace read / write requestの送信前fail-closed
- token設定時だけloopbackへ起動するlocal MCP read toolsとOwner-proxy write
- M.I.O.ロゴ、外観設定、参加AI一覧の折りたたみUI

### Safety boundaries

- recipient単位のdurable dispatch ledgerとidempotency
- 結果不明のProvider turnを自動再送しないunknown outcome処理
- 未接続AIの偽replyや成功表示を生成しない
- single-instanceによるRoom writerの重複防止
- Windows alpha.1でworkspace read / write UIを無効化し、保存済みrequestもProvider起動前に拒否
- local MCPのloopback限定、token認証、bounded request

### Experimental or unavailable

- Claude Webの正式製品接続
- 公開Remote Relay、複数device、複数account
- ChatGPT Web、OpenAI API、Generic MCP client、Custom adapter
- Google Search / AI Mode Browser Bridgeの正式製品化
- Codex、Fable、Gemini、Grokのworkspace access
- token streaming UI、Provider turn途中のcancel、model選択UI
- background automation、無制限のConductor round、nested delegation
- Windows以外の対応保証

### Distribution

- source-firstの研究用α版
- 配布用installer、コード署名、自動更新、安定版SLAは未提供
- 公開可否は `docs/PUBLIC-ALPHA1-READINESS.md` の検証完了後に判断
