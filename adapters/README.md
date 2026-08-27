# Adapters

Provider別の交換可能な接続実装を置きます。将来の例:

```text
adapters/
  codex/
  claude-code/
  claude-web/
  chatgpt/
  openai-api/
  grok/
  gemini/
  generic-mcp/
```

実証前に空のProvider実装を量産せず、PoCで接続経路が確認できたものから追加します。
