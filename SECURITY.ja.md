# セキュリティポリシー

[English](SECURITY.md) | **日本語**

## 対応version

M.I.O.は最初のWindows向け安定版を準備中です。セキュリティ修正は最新の対応releaseを対象とし、古いprereleaseや開発snapshotへのbackportは保証しません。

| Version | 対応状況 |
|---|---|
| 公開後の最新`1.x`安定版 | 対応 |
| `v1.0.0`公開までの最新`1.0.0-rc.x` | 対応 |
| `0.1.0-alpha.x`以前のprerelease | 非対応 |
| 未releaseの`main` branch snapshot | best effort。対応releaseではない |

非対応version固有の問題を報告する場合は、先にupgradeしてください。upgradeできない事情がある場合は、その制約を報告へ含めてください。

## 脆弱性の報告

脆弱性やcredential漏えいにつながる問題は、公開Issueへ詳細を書かないでください。[GitHub Private Vulnerability Reporting](https://github.com/blackcometclub/M.I.O/security/advisories/new)から非公開で報告してください。このformが利用できない場合は、公開Issueに`Private security contact requested`という件名だけを書き、リポジトリ所有者へ非公開の連絡手段を確認してください。そのIssueには脆弱性、再現手順、ログ、秘密情報を書かないでください。

特に次の領域の報告を歓迎します。

- credentialやtokenがWebView、ログ、証跡へ漏れる問題
- Tauri command / IPCの権限境界を越える問題
- Relay / MCPの認証、pairing、request correlationの欠陥
- 許可範囲外のファイル読み書きやpath traversal
- 意図しない外部送信、tool実行、承認の迂回

報告には、影響、再現に必要な最小手順、確認したcommitを含めてください。実在するAPI key、token、cookie、個人情報は添付しないでください。秘密情報が露出した可能性がある場合は、報告を待たずに該当credentialを失効・再発行してください。

## 報告後の対応

maintainerは報告を非公開で確認し、再現を試み、M.I.O.本体の問題か、外部Provider、CLI、serviceの問題かを切り分けます。M.I.O.の有効な問題は、可能な範囲で最新の対応versionを修正します。協調開示が可能な場合は、詳細を公開する前にsecurity advisory、回避策、または修正版のrelease noteを準備します。

M.I.O.にはセキュリティ対応期限（SLA）やbug bounty制度はありません。回答・修正に要する時間は、深刻度、再現性、maintainerの対応可能時間、upstream修正に左右されます。privacy侵害、service妨害、永続化、問題を示すために必要な最小範囲を越えるaccessを避けた、善意の報告を歓迎します。

## 現在の制約

`spikes/` は接続方式を検証するためのPoCです。製品品質や公開サーバー運用を保証するものではありません。外部公開や実アカウントを伴うprobeは、各READMEの前提とデータ送信範囲を確認したうえで、隔離したテストデータだけを使用してください。
