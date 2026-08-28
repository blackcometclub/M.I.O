# セキュリティポリシー

[English](SECURITY.md) | **日本語**

## 対応version

M.I.O.は現在、安定版を公開していないWindows向け研究用α版です。`v0.1.0-alpha.2`を2026-08-28にsource-firstのGitHub Prereleaseとして公開しました。セキュリティ修正の対象は原則として最新の`main` branchだけであり、修正期限（SLA）はまだ定めていません。

## 脆弱性の報告

脆弱性やcredential漏えいにつながる問題は、公開Issueへ詳細を書かないでください。GitHubリポジトリの **Security** タブから非公開で報告してください。Private vulnerability reportingが利用できない場合は、再現手順や秘密情報を公開せず、リポジトリ所有者へ非公開の連絡手段を確認してください。

特に次の領域の報告を歓迎します。

- credentialやtokenがWebView、ログ、証跡へ漏れる問題
- Tauri command / IPCの権限境界を越える問題
- Relay / MCPの認証、pairing、request correlationの欠陥
- 許可範囲外のファイル読み書きやpath traversal
- 意図しない外部送信、tool実行、承認の迂回

報告には、影響、再現に必要な最小手順、確認したcommitを含めてください。実在するAPI key、token、cookie、個人情報は添付しないでください。秘密情報が露出した可能性がある場合は、報告を待たずに該当credentialを失効・再発行してください。

## 現在の制約

`spikes/` は接続方式を検証するためのPoCです。製品品質や公開サーバー運用を保証するものではありません。外部公開や実アカウントを伴うprobeは、各READMEの前提とデータ送信範囲を確認したうえで、隔離したテストデータだけを使用してください。
