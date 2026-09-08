# ADR 0045: Windows V1 code signing and timestamp policy

- Status: Accepted
- Date: 2026-09-03
- Depends on: ADR 0043 (Windows V1 product scope), ADR 0044
  (per-machine installer)

## Context

M.I.O.のWindows release candidateは、install、update、uninstall、data保持を実機で
検証するため未署名でもbuildできる。一方、未署名artifactにはpublisherの本人性と
改ざん検出を利用者が確認できる署名がなく、正式V1の配布物として扱えない。

新しい証明書で署名してもMicrosoft Defender SmartScreenのreputationは直ちには
確立しない場合がある。SmartScreen警告の完全な不表示と、署名の有効性は別の条件と
して扱う必要がある。

## Decision

1. NSISを使う`1.0.0`以降の正式Windows releaseは、main executableとNSIS installerの
   両方へAuthenticode署名を付ける。未署名artifactはRC、開発、実機試験、または
   **署名なしプレビュー**に限り、正式V1として掲載しない。
2. file digestはSHA-256、timestampはRFC 3161とSHA-256を使う。timestamp URLは
   certificate issuerが案内するHTTPS endpointを採用する。
3. release準備時に、WindowsのDefault Authentication Verification Policyでchainを
   検証し、署名状態、完全一致するcertificate subject、timestamp certificateを確認する。
   hashだけ、自己署名証明書、想定外publisher、timestampなしを合格にしない。
4. certificate、private key、token、passwordはrepository、release plan、logへ保存しない。
   certificate storeまたはissuerが提供するhardware／cloud signing mechanismから使う。
5. certificate issuerとcertificate subjectは購入・本人確認前にrepositoryへ仮定して
   固定しない。Ownerが選んだ後、TauriのWindows signing設定とrelease commandへ設定する。
6. 日常のbuildとRC試験は未署名を許可する。正式versionのrelease準備と公開処理だけは、
   `scripts/test-windows-release-signature.ps1`の検証失敗時に停止する。
7. GitHub Actionsの利用は署名の必須条件にしない。V1はOwner管理のWindows機で同一commit
   からbuild、署名、検証できる。将来CI署名へ移す場合もsecret materialをrepositoryへ
   置かない。

## Release staging decision

2026-09-03にOwnerと次の順序を採用した。

1. commercial dual licensingは維持したまま、未署名NSISを明確に`RC`または`Preview`と
   表示して試用できる状態にする。公開時は未署名、SmartScreen警告の可能性、SHA-256、
   source commitを省略しない。
2. 正式V1の第一候補としてMicrosoft Store用MSIXを別途試作し、Provider CLI、workspace、
   AppContainer、sidecar、WebView2、既存data移行を実機検証してから審査へ提出する。
   審査時は自動公開を選ばず、合格後もOwnerが公開を決めるまで保留する。
3. MSIX審査が不合格でも、報告された技術・policy理由を確認し、修正と再申請が可能なら
   先に対応する。一度の不合格だけでcommercial licensing方針を変更しない。
4. Store署名と自前署名の両方が現実的でない場合に限り、未署名installerを正式V1とするか、
   commercial dual licensingを終了してSignPath Foundationへ申請するかを、Ownerが改めて
   明示的に判断する。SignPathへの採用はlicense変更後も保証されない。

## SignPath Foundation candidate assessment

2026-09-03時点の無料OSS条件は、commercial dual licensingを行っていないことを求めている。
M.I.O.はADR 0004と`COMMERCIAL-LICENSE.md`で別途有償の商用licenseを提供する方針のため、
現在のまま無料枠へ適合すると主張しない。申請は送信せず、詳細を
`docs/SIGNPATH-FOUNDATION-READINESS.md`へ記録する。商用license方針を変更するか、現在の
方針でも対象になるというSignPath Foundationの書面確認を得るまで候補を保留する。

## Consequences

- 証明書を取得するまでは`1.0.0-rc.*`の検証と、未署名であることを明示したPreview公開を
  継続できるが、正式V1とは扱わない。
- timestampにより、署名時点でcertificateが有効だったことを期限後も検証できる。
- 有効な署名だけではSmartScreen警告の不表示を保証しないため、release notesにはhash、
  publisher、警告時の確認方法を記載する。
- certificateを選んだ後に必要なrepository変更は、秘密情報を含まないTauri signing設定と
  release引数の追加だけに限定できる。
- 無料署名制度へ合わせるために、release engineeringの判断だけでcommercial licensing方針を
  削除しない。
