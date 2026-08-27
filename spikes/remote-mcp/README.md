# Claude Web Remote MCP spike

通常のClaude WebからM.O.E.へ接続するTest 0-Cの最小実験です。Claude Code CLIとは別経路です。

現段階では公開せず、`127.0.0.1`だけでStreamable HTTPの初期化、tool一覧、tool呼び出しを確認します。公開前の安全境界として、次の2ツールだけを持ちます。

- `ping_moe()` — 到達確認
- `moe_status()` — 機密情報を含まない固定状態の読み取り
- `moe_read_room()` — ホストが選んだruntimeスナップショットから、最大30件のRoomメッセージを読み取る

`moe_read_room()`はClaudeからファイルパスを受け取りません。ホストが起動時に指定した `runtime/` 内のスナップショットだけを読み、`roomId`、`afterMessageId`、1〜30の`limit`だけを受け付けます。未知のRoomやcursorは明示エラーです。Artifact実体、shell、Desktop操作、任意URL取得、書き込みtoolは実装していません。すべてのtoolがMCPの`readOnlyHint`、`idempotentHint`を宣言します。

## ローカル検証

リポジトリルートで実行します。

```powershell
npm.cmd test --workspace @moe/remote-mcp-spike
```

Desktop outbound Relayとの全経路をlocalhostで統合検証:

```powershell
npm.cmd run test:relay-integration --workspace @moe/remote-mcp-spike
```

このintegration probeでは `MOE_RELAY_BASE_URL` をloopback HTTP originだけに限定し、Remote MCPの `moe_read_room` と `moe_status` をRelay経由へ切り替えます。完成版の公開Relay URLやproduction認証を受け付ける実装ではありません。

手動起動:

```powershell
npm.cmd start --workspace @moe/remote-mcp-spike
```

既定URLは `http://127.0.0.1:3108/mcp` です。外部公開する一時試験では `MOE_REMOTE_MCP_PATH` に推測困難なパスを渡し、公開URLだけではMCP endpointへ到達できないようにします。これは正式な認証の代替ではなく、固定された非機密toolだけを短時間公開するための追加防御です。

Tunnelが割り当てたHostは、`MOE_REMOTE_MCP_ALLOWED_HOST`へ単一の小文字hostnameとして明示します。localhostとそのHost以外は公式SDKのDNS rebinding防止middlewareが403で拒否します。

一時公開後の公式SDKクライアント検証は、秘密URLを環境変数だけで渡します。URLはログや追跡証跡へ保存しません。

Claude WebでRoom読み取りを試す場合は、追跡対象fixtureから無視対象のruntimeスナップショットを生成します。

2026-08-12の短時間試験では、Claude Web無料プランが `moe_read_room` を認識し、`afterMessageId=welcome-3`、`limit=1`でruntime追加メッセージを読み取るところまでPASSしました。公開hostnameと秘密pathは証跡へ保存せず、試験後にTunnel、server、runtime snapshot、秘密sessionを削除しています。

```powershell
npm.cmd run prepare:public-room --workspace @moe/remote-mcp-spike
```

```powershell
$env:MOE_REMOTE_MCP_URL = "https://temporary.example/<secret>/mcp"
npm.cmd run test:public --workspace @moe/remote-mcp-spike
Remove-Item Env:MOE_REMOTE_MCP_URL
```
