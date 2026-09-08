# M.I.O. サポート方針

[English](SUPPORT.md) | **日本語**

## 対応範囲

M.I.O. V1は、最新公開済み`1.x`安定版と64-bit版Windows 11の組合せをサポートします。`v1.0.0`公開までは、最新`1.0.0-rc.x`だけをrelease candidate報告の対象とします。古いalpha、開発snapshot、Windows 10、Windows on Arm、32-bit Windows、macOS、Linuxは対応対象ではありません。

再現可能なM.I.O.本体のinstall、起動、Room、Provider送信、backup／restore、Codexの限定workspace操作、update、uninstallの不具合を対象にします。Provider accountの運用、契約・利用枠の取得、Provider側の保存・課金変更、別途導入したProvider CLI自体の修復は含みません。

## 連絡先

- 再現可能な製品不具合: [不具合報告form](https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml)
- 機能提案: [機能提案form](https://github.com/blackcometclub/M.I.O/issues/new?template=feature_request.yml)
- 脆弱性やcredential漏えいの可能性: 公開Issueではなく[SECURITY.ja.md](SECURITY.ja.md)の手順
- Provider障害、account、model利用可否、課金、CLI installerの問題: 各Providerの公式support

回答期限・修正期限のSLAはありません。再現できない、非対応versionである、upstream Providerの問題である、private credentialやdataへのaccessが必要になる場合は、調査終了または適切な窓口への案内となることがあります。

## 報告に含めるもの

- M.I.O. versionとinstaller／sourceの入手元
- Windows edition、version、architecture
- 短い再現手順と、期待した結果・実際の結果
- 関係するProviderとCLI version
- Roomが会話のみ、読取、読取・編集のどの権限だったか
- 匿名化したerror文と、失敗、停止、timeout、配達結果不明のどれだったか

Room内容、非公開file、password、OAuth code、API key、cookie、access token、個人情報、credentialを含む完全なlogは公開しないでください。最小限のtest Roomを使い、添付前にpathと名前を匿名化してください。

## 更新と対応終了

V1はinstallerによる手動updateです。security・製品修正は最新の対応releaseを対象とし、古い安定版やprereleaseへのbackportは保証しません。新しい安定patchがある場合は、そのversionで再現確認をお願いすることがあります。

install、update、data、uninstall、troubleshootingは[V1利用ガイド](docs/USER-GUIDE.ja.md)を参照してください。
