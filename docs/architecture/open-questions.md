# Architecture open questions

Status: Draft / Not decided

Room catalog、3標準RoomのUI hydration、全Room message write、idempotentなRoom作成と参加AI追加、single-room fileの非破壊補完、Room名変更、未参照AIの取り外し、ユーザー作成Room削除、primary破損自動復旧、全Room backup / 最新復元はADR 0010から0017で決定済み。Room並べ替え、履歴を保持したparticipant tombstone、複数process writer、schema migration、任意backup import / retention、永続dispatch ledgerは未決定のままとする。

初期計画を複数Providerへ拡張する際に、実装前または各PoCで確認する論点です。この一覧自体は技術選定ではありません。

## Vocabulary and identity

- Provider、Model、Adapter、Transport、Endpoint、Role、Agent、Session、Roomをどう区別するか
- 同じProviderの複数アカウントや複数接続をどう識別するか
- 表示名の変更やモデル廃止後も履歴の出所を再現できるか

## Capability negotiation

- Adapterが対応能力と制限値をどの形式・versionで申告するか
- text、image、audio、document、tool、streaming、resume、fork、approval、interrupt、pushをどう表現するか
- 未対応機能に対するfallbackをCore、Adapter、UIのどこが決めるか

## Connector lifecycle

- 接続phase、metadata-only status、managed drop、明示start / stop、bounded retry、Desktop handle ownershipは製品Relay境界で固定済み。health check、更新、無効化、削除の状態遷移は未決定
- 状態機械、実timer、cooperative cancellation、orchestrator自動retry、localhost socket cancelに加え、HTTPS transport、OS証明書・hostname検証、I/O deadline、frame上限、確立後socket cancel、build-time設定からDesktop taskへの自動結合、strictなRoom read router、request ID相関、bounded Tauri read command、全RoomのUI hydration / message write / catalog mutation、Codex App Serverのtext-only subprocess driver、単一processのversioned Room snapshot永続化・破損復旧・明示backup / 最新復元は固定済み。DNS解決の全体deadline、schema migration、任意backup import / retention、複数process writer、履歴を伴うparticipant mutation、Claude Web / Gemini dispatch、byte-budget paging、WebSocket、MCPのproduction driver、health check、backpressureは未決定
- 同じ外部Sessionを複数画面や複数プロセスから操作する際の排他

## Security and data egress

- API key、OAuth token、session credentialをWindows上でどう保管するか。device credentialはWindows Credential Manager、製品候補の保管crate、Rust Relay client、production HTTPS Desktop task、自動再接続までPASS。Relay側credential record、rotation、複数account、非Windows backendは未決定
- Artifactや会話を外部Providerへ送る直前に、送信先と内容をどう提示するか
- project root外へのアクセス、symlink、巨大ファイル、秘密情報をどう制限するか
- Relay、Desktop、Adapter、外部Provider間のtrust boundary

## Delivery semantics

- ユーザーmessage writeのclient-generated message IDとpayloadはRoom snapshotへ永続化済み。Codex dispatchはsource message ID + recipientの別keyをprocess内ledgerへ記録し、同一processの二重外部turnと失敗後の自動再送を拒否する。process restartをまたぐdispatch ledger、provider側idempotency、Claude Web / Gemini、複数同時dispatchは未決定
- timeout、cancel、provider切断後に結果が到着した場合の扱い
- streaming eventの順序、backpressure、部分結果、再開位置
- 同期応答から非同期Jobへ切り替わる条件

## Human control and budgets

- approvalやpermissionを誰が、どの画面で、どの期限まで承認できるか
- 自動連鎖の最大turn、時間、token、金額、並列数
- ユーザー操作待ちを失敗と区別する状態
- Providerごとのrate limitとquota表示

## History and provenance

- 正規化MessageとProvider raw eventの関係
- 返信、引用、派生Artifact、Job、外部turnをたどれる因果関係
- 削除、export、retention、機密情報redaction
- SQLite schemaとAdapter契約のmigration/versioning

## Generic MCP

- MCP serverを「AI参加者」として扱う場合と「参加者が使うTool」として扱う場合の分離
- M.I.O.自身がMCP server、client、または両方になる範囲
- remote serverのtool結果を信頼済みArtifactへ昇格させる条件

## Web products

- Claude Web、ChatGPT、Grok、Geminiについて利用可能な公式接続経路
- 通常Web UIへの自動入力に依存せず実現できる範囲
- ユーザーによるログイン、connector登録、確認操作が必要な境界
- API版とWeb製品版を同一Adapterにまとめるか分離するか

## Testing and compatibility

- Adapterごとのcontract testと、外部課金を伴わないfake Adapter
- CLI、schema、API version更新時の互換性検査
- 実サービスを用いるmanual smoke testの証拠と有効期限
- UI上の成功表示と、実際の配送・観測完了を区別する基準

## Packaging and operations

- Desktop技術、Relay配置先、更新配布、署名方式
- ローカルDB、Artifact Store、ログのbackupと復旧
- crash reportへ会話や秘密情報を含めない仕組み
- 個人利用から共有環境へ拡張する場合の認証・権限モデル
