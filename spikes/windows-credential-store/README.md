# Windows Credential Manager spike

M.O.E. Desktopのdevice credentialを平文ファイルや設定JSONへ保存せず、製品候補の `moe-credential-store` crate経由でWindows Credential Managerへ保存・復元できるかを確認する隔離PoCです。

```powershell
cargo run -p moe-windows-credential-store-spike --quiet
```

probeは実行ごとに一意な `probe-...` account IDと、製品候補schema `M.O.E./relay-device/v1/<account-id>` を使い、次を別process間で確認します。

- 新規保存
- 別processからの完全一致読取
- credential更新
- 更新前credentialとの不一致
- 削除
- 削除後のnot found
- child stdout / stderrへsecretが出ていないこと

secretはcommand line引数へ渡さず、probe childの環境変数だけで受け渡します。標準出力には結果だけを出し、credential自体、fingerprint、targetの完全名は出しません。成功・失敗にかかわらず、probe targetだけを削除するcleanup guardを持ちます。

このPoCはWindows専用です。製品結合、Tauri command、複数アカウントUI、credential rotation policy、Relay側recordの永続化はまだ実装しません。
