# Spikes

公式CLI、API、MCP、Relay、Artifact転送などの実現性を短時間で確かめる、捨てられるPoCを置きます。

候補:

```text
spikes/
  fixtures/
    seeded-bug-app/
  codex-app-server/
  claude-code-stream/
  remote-mcp/
  relay-roundtrip/
  artifact-image/
  timeout-behavior/
  generic-mcp/
  fake-adapter/
```

PoCのコードをそのまま製品Coreへ昇格させず、観測結果と採用判断を分けて記録します。pingや固定文字列だけでは合格とせず、共通fixtureを使った実変更、Artifact、失敗復旧、重複防止まで検証します。

`fixtures/seeded-bug-app/` はProvider間で再利用する共通の不具合fixtureです。各probeは追跡対象の `baseline/` から、Gitで無視される `runtime/` を作成して実行します。baseline自体をAIの作業対象にしてはいけません。
