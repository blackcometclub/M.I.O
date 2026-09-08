# ADR 0043: Windows V1 product scope and provider guarantees

- Status: Accepted
- Date: 2026-08-31
- Supersedes: ADR 0037のalpha release scope（V1に限る）
- Depends on: ADR 0035 (Room-scoped conductor orchestration), ADR 0038
  (Room workspace execution permissions), ADR 0039 (provider model
  selection), ADR 0040 (Claude Code provider identity), ADR 0042 (Grok
  read-only Git review)

## Context

M.I.O.のV1は、将来研究中の接続をすべて完成させる版ではない。中心目的は、
利用者が選んだ一つのRoom workspaceを複数AIとの会話へ結び、Codexに安全な
開発操作を任せ、Providerごとのmodelを明示的に選べるWindows desktop製品を
安定して配布することである。

Remote Relay、複数device、複数account、macOS、LinuxまでV1へ含めると、現在
保有していないserviceと実機の境界まで保証する必要がある。未検証のplatformや
Provider権限を完成機能として扱わず、実装済みのWindows経路へ試験を集中する。

## Decision

### Product and platform scope

1. V1はWindows x64向けのlocal-first desktop applicationとする。現在の主な
   検証対象はWindows 11 x64であり、対応buildはrelease candidateの実機証拠に
   基づいて公開文書へ記載する。Windows 10を未検証のまま保証しない。
2. V1の中核を、永続Talk Room、Direct mode、1 round・最大3 workerの
   Conductor mode、Room単位のworkspace、Providerごとのmodel選択とする。
3. Room workspaceの読取り・編集をV1必須にするが、書込みを許可するProviderは
   Windows境界を個別に証明したCodexだけとする。権限はRoomとparticipantへ
   device-localに保存し、chat-onlyを既定とする。
4. model選択はProvider既定を初期値とし、M.I.O.が実際に検証した候補だけを表示
   する。選択不能なmodelへの暗黙fallbackは行わない。Geminiは明示model指定を
   検証するまでProvider既定だけを許可する。

### Provider guarantees

V1で保証できるProvider範囲を次のように固定する。各経路はrelease candidateで
実accountを使った一回以上の成功証拠を持つ場合だけ、公開文書で「対応」とする。

- **Codex**: 会話、Codex conductor、worker、Room workspaceの読取り・編集、
  broker経由の列挙済みfile操作と確認付きcommand。任意shell、任意path、無確認の
  外部送信は許可しない。
- **Claude Code**: 会話とworker。検証済みmodel選択に対応する。workspace、command、
  Web、MCPはV1で許可しない。
- **Gemini Antigravity CLI**: 会話とworker。modelはProvider既定だけとする。
  workspace、command、Web、MCPはV1で許可しない。
- **Grok CLI**: 会話に対応する。ADR 0042のboundedな追跡済みGit差分レビューは、
  実accountのrelease gateを通った場合だけ読取り機能として有効化・説明する。
  workspace file tool、未追跡file本文、編集、command、Web、MCPは許可しない。
- **Claude Webおよび任意Custom Provider**: V1の正式接続には含めない。

CLIの未導入、未認証、契約や利用上限による拒否は、M.I.O.全体の起動失敗へ変換
しない。M.I.O.はProviderが返していないreplyを作らず、結果不明のturnを自動再送
しない。

### Explicit V1 non-goals

次はV1の完成や公開を止めない。

- Remote Relay、複数device、複数account、公開server運用。
- macOS、Linux、Windows on Arm、32-bit Windows。
- Claude Webの正式接続、Generic MCP client、任意Provider plugin。
- Codex以外のworkspace writeと、Grokの限定レビュー以外のworkspace read。
- background automation、無制限Conductor round、nested delegation。
- token streaming UIと自動update。安全な停止と手動update手順は別に維持する。

非対応surfaceをUIへ残す場合は、選択不可にするか、現在未対応であることと利用者へ
与える権限を明記する。将来実装を示す表示だけでV1対応とは扱わない。

## Release gates

1. 対応と説明する各ProviderでDirect replyを保存し、失敗時に偽replyや自動再送が
   ないことを確認する。
2. Codex workspaceのread、create、replace、代表的なGit／Node／npm操作、確認拒否、
   workspace外拒否を製品UI経路で一巡する。
3. Provider modelの既定・明示指定・変更後continuityを製品経路で確認する。
4. Windows release candidateでinstall、update時のdata保持、uninstall、再起動、
   shortcut、Defender scan、署名を確認する。
5. 対象外機能がUI、README、release notesで対応済みに見えないことを確認する。

自動testのPASSだけで実Provider、実画面、install、OS対応を完了扱いにしない。

## Consequences

- V1開発をWindows上のworkspace協働と安全な一般配布へ集中できる。
- Codex以外のAIも会話やレビューへ参加できるが、ProviderごとのOS境界を証明せずに
  file操作を横展開しない。
- Windows 10やGrok reviewなど、release gateを通していない経路は、実装が存在しても
  V1の保証範囲へ自動的には入らない。
- Remote Relayと複数deviceは将来の独立したservice設計・費用判断として残る。
