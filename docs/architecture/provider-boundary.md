# Provider boundary（Draft）

Status: Draft / Not decided

## 問題

M.I.O.が接続する対象には、Web UI、ローカルエージェント、公式API、MCP server/clientなど異なる種類があります。全接続先へ同一機能を要求すると、未対応機能の偽装やProvider固有仕様のCore流入が起きます。

## 設計候補

Coreは、Provider名ではなく役割と能力を見て処理を組み立てます。Adapterは概ね次の情報を公開します。

- identity: Adapter種別、Provider、version
- transport: stdio、HTTP、WebSocket、MCP、その他
- input: text、image、file、structured data
- output: streaming、final response、artifact
- session: create、resume、fork、history
- control: interrupt、approval、permission
- tools: tool calling、MCP client、MCP server
- delivery: pull、同期応答、非同期job、inbound push

未対応能力は明示的に未対応とし、Coreがfallbackまたはユーザー操作を提示できるようにします。

## 未決事項

- Adapter processをin-process、subprocess、plugin packageのどれにするか
- Capabilityのversioningと互換性判定
- Web UI専用接続で許容する自動化範囲
- 認証情報の保存方式
- Provider raw eventの保存期間
- Generic MCPをAgent AdapterとTool Adapterのどちらとして扱うか
