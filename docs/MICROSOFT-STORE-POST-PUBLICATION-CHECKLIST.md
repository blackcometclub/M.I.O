# M.I.O. Microsoft Store公開後チェックリスト

この文書は、Microsoft Storeの初回公開および更新公開の後に、
公開ページ、Store版の取得、Windows 11実機、既存Room data、レーベルサイトを
順番に確認するための手順である。

## 固定情報

- 製品名: `M.I.O`
- Store ID: `9NS9B7T71XHN`
- Package Identity Name: `TINMOON.M.I.O`
- Publisher: `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`
- 現在の公開package version: `1.0.4.0`（2026-09-08確認）
- 公開済みSubmission 4 ID: `1152921505701829315`
- Store Web page: `https://apps.microsoft.com/detail/9NS9B7T71XHN`
- Microsoft Store app URI: `ms-windows-store://pdp/?ProductId=9NS9B7T71XHN`
- Partner Center overview:
  `https://partner.microsoft.com/ja-jp/dashboard/products/9NS9B7T71XHN/overview`

## 現在の公開境界

2026-09-08、Partner Center実画面でSubmission 4の公開と「お客様の最新の製品が
Microsoft Storeで入手可能になりました」の案内を確認した。現在の公開版は`1.0.4.0`である。
これは公開状態を確認した日であり、配信開始時刻そのものは未取得である。

2026-09-05、Windows実行file名を`mio-desktop.exe`／`mio-command-helper.exe`へ統一した更新package
`M.I.O_1.0.2.0_x64_store-unsigned_20260905-170824.msix`を、Ownerのupload承認後にSubmission 2へ
登録した。local fileは6,520,238 bytes、SHA-256
`1605B2712FB1421D70C46450E70C9E7045E08F886433F83F0980E5CF331BC1CB`で、Partner Centerのpackage
検査は`Validated`、Windows 10/11 Desktop、x64、version `1.0.2.0`を示した。

Submission 2はその後認定に合格し、`1.0.2.0`として公開された。

2026-09-07、現在のsourceからStore更新package
`M.I.O_1.0.3.0_x64_store-unsigned_20260907-080838.msix`を生成した。local fileは6,515,756 bytes、
SHA-256 `B6B607C5802523E1C4DDCC97285E033DC0FC3EF1D98E2703F429D250C6DF58DD`。再展開でIdentity、
Publisher、x64、version `1.0.3.0`、Windows.Desktop minimum `10.0.22000.0`、`en-us`／`ja-jp`、
`runFullTrust`、本体とhelper、license類を確認した。Partner Center Submission 3へuploadして保存し、
package検査`Validated`を確認した。Ownerの明示承認後に認定へ提出し、最終確認時点は
`更新プログラムの認定中`／`前処理中`だった。その後、2026-09-07にPartner Centerの実画面で
Submission 3が公開中になったことを確認した。読み取り専用の公開package詳細は
`M.I.O_1.0.3.0_x64_store-unsigned_20260907-080838.msix`、version `1.0.3.0`、x64、
Windows 10/11 Desktop、minimum `10.0.22000.0`を示していた。

同日、次期版commit `fbc81b89ee64a670f13128487c06a96bfabe3238`から
`M.I.O_1.0.4.0_x64_store-unsigned_20260907-220224.msix`を生成した。local fileは6,527,560 bytes、
SHA-256 `EFF886B23E261AAEBE758A27BD684CA08776C4F702EE5AC44D633C4E563F0192`。package scriptへ版数を
明示せず、通常版manifest `1.0.4`からStore版`1.0.4.0`が自動採用された。MakeAppxの生成・再展開と
必須9 fileの存在確認、Defender custom scanの検出0件まで完了した。このpackageは未署名の
Store提出用であり、サイドロードは行っていない。Ownerの明示承認後にSubmission 4
（ID `1152921505701829315`）へuploadして保存し、Partner Centerのpackage検査`Validated`、
Windows 10/11 Desktop、x64、version `1.0.4.0`、minimum `10.0.22000.0`、`en-us`／`ja-jp`、
`runFullTrust`を確認した。さらにOwnerの認定提出承認後にMicrosoftへ送信し、最終確認時点は
`更新プログラムの認定中`／`前処理中`だった。この提出時点ではSubmission 3／`1.0.3.0`が公開中だった。

2026-09-04にOwnerの明示承認を受け、旧`1.0.0.0` packageの提出を取り消して削除し、
正式V1候補のsource commit `8e3eb4e22c67a52007ca1890cb912b477314368a`から作成した
`1.0.1.0` packageへ差し替えて再提出した。以後は、提出を取り消したり、packageを再差し替え
したりしていなかった。Submission 2は、その後の実行file名修正を届ける別の更新である。

次回更新時は、公開中versionと更新候補versionを再確認する。公開操作や認定取消が必要な画面で
あれば、Ownerの新しい明示承認を得てから1回だけ実行する。

Store IDを使った`ms-windows-store://pdp/`は、Microsoftが特定製品の詳細ページを
開く方法として案内している。Package Family NameやApp IDを推測してリンクを作らない。

## ヒツジさんからの最短の頼み方

通常は次の順に依頼すればよく、PowerShellを自分で操作する必要はない。

1. 「Storeの更新状態を確認して」
2. 認定中のSubmissionが合格したら「公開版と更新版を照合して」
3. 手動の公開操作が必要なら、最終確認後に「Store更新の公開を承認します」
4. 更新配信が確認できたら「Store版の更新試験を準備して」
5. backup結果を確認して「Store版の更新を承認します」
6. 更新後の起動とRoom data確認を終えたら「公開後記録を更新して」
7. preview確認後、commit、push、deployをそれぞれ明示承認する

## 承認境界

- Partner Center、公開Web page、Store pageの読取り確認は、そのまま実施できる。
- Storeからのinstall、update、uninstallは実機を変更するため、直前にOwnerへ確認する。
- 認定の取消、再提出、公開時刻変更、package差替えは、Ownerの明示承認なしに行わない。
- `今すぐ公開`は一般公開を開始する操作なので、Ownerの明示承認なしに選ばない。
- レーベルサイトの編集、commit、push、Cloudflare Pages deployは、それぞれ対象を示して
  Ownerの承認を得る。
- GitHub tag／ReleaseはStore公開と別作業とし、自動的に作成・公開しない。

## 0. 認定結果を確定する

Partner Centerで次を記録する。

- Submission番号と最終更新日時
- `認定中`、`認定合格・公開保留`、`公開処理中`、`公開済み`のどこまで進んだか
- 認定失敗の場合はerror code、理由、対象fieldまたはpackage
- 公開済みの場合は公開日時とStore pageへのリンク

`認定合格・公開保留`は公開済みではなく、`公開処理中`も公開完了ではない。認定失敗時は
推測で修正、再upload、再提出せず、表示内容をそのまま記録して停止する。

## 1. 公開ページを匿名で確認する

最初に通常のWeb pageを、Partner Centerへloginしていないbrowser contextでも開く。

```text
https://apps.microsoft.com/detail/9NS9B7T71XHN
```

次を目視確認する。

- M.I.O.のページがMicrosoft Store homeへredirectされず表示される
- 製品名、発行元`TINMOON`、価格`無料`が正しい
- 英語と日本語の説明、4枚のscreenshot、iconが表示される
- privacy policy、website、support linkが正しい
- Windows 11 desktop向けとして取得操作が表示される

表示が地域や言語で未反映の場合は、公開処理中の可能性を記録して待つ。
説明や画像をその場で編集しない。

Microsoft Store appの製品ページは、PowerShellから次で開ける。

```powershell
Start-Process 'ms-windows-store://pdp/?ProductId=9NS9B7T71XHN'
```

## 2. 実機dataを保護する

install前にM.I.O.を通常終了し、既存のNSIS版とStore版を混同しないようにする。
Store版ではMSIXのAppData仮想化により、同じ論理pathでも通常のPowerShellから読むfileと
Store processから読むfileが異なる場合がある。更新対象の版の全Room backupを優先し、
通常の`%APPDATA%`を外からコピーしただけでStore側のbackupも完了したと扱わない。

1. M.I.O.のRoom設定から全Room backupを作成し、成功通知とfile名を記録する。
2. active Room snapshotの存在、size、SHA-256を記録する。
3. `%APPDATA%\app.moe.desktop`全体を、別driveの新規timestamp付きfolderへコピーする。
4. コピー元とコピー先のfile数、総byte数、主要snapshotのSHA-256を照合する。
5. `C:\Program Files\M.I.O`のNSIS版が存在するか記録する。勝手にuninstallしない。

active Room snapshot:

```text
C:\Users\black\AppData\Roaming\app.moe.desktop\room-snapshot-v1.json
```

既存dataの削除、rename、restoreは行わない。Store版が別のdata領域を使った場合も、
空Roomを既存dataの消失と即断せず、両方の保存場所を読取り専用で調べる。

## 3. Store版を取得してWindows 11実機で確認する

Ownerの承認後、Microsoft Store pageの取得／installを1回だけ実行する。
途中で結果が分からなくなった場合は、重複installを避けるため連打や再実行をしない。

install後、現在のWindows userに登録されたpackageを確認する。

```powershell
Get-AppxPackage -Name 'TINMOON.M.I.O' |
  Select-Object Name, PackageFullName, Version, Architecture, Publisher, SignatureKind, Status, InstallLocation
```

期待値:

- `Name`: `TINMOON.M.I.O`
- `Version`: `1.0.4.0`（次回更新では公開package詳細と照合して変更する）
- `Architecture`: `X64`
- `Publisher`: 上記の固定Publisherと一致
- `SignatureKind`: Microsoft Storeから取得したpackageとして有効
- `Status`: `Ok`

続けて次を確認する。

1. Start menuのM.I.O.から起動できる。
2. Main window、Room list、設定panelが正常に表示される。
3. 起動、通常終了、再起動が成功する。
4. 既存Roomとmessageが意図どおり見える。見えない場合は書込み試験をせず停止する。
5. Provider設定とmodel選択が保持される。実送信は必要なProviderだけ別途承認して行う。
6. Room workspaceは選択folder以外を操作できず、権限表示が従来どおりである。
7. NSIS版とStore版が同時に存在する場合、shortcut、process path、data pathを記録し、
   どちらかを削除する前に移行方針を決める。

Store版を入手できたことだけで、clean Windows 11試験やWebView2なし環境の試験を
完了扱いにしない。

## 4. 最小の製品smokeを行う

既存の試験用Roomを使い、製品dataを壊さない最小範囲で確認する。

- Roomを1回切り替える
- 設定panelを開閉する
- backupの保存先表示を確認する
- CodexのDirect modeで短い固定replyを1回だけ確認する
- 画像なしmodeとRoom tree開閉を確認する
- 通常終了後に再起動し、直前のRoomとmessageが再表示されることを確認する

Provider課金、外部送信、workspace書込み、npm導入はこの基本smokeへ含めない。
必要な場合は目的と回数を示し、別の試験として行う。

## 5. 更新とuninstallは別試験にする

初回取得・起動が合格するまでは、updateやuninstallへ進まない。

- updateが配信された場合は、更新前後のpackage versionとRoom snapshot hashを記録する。
- uninstallはOwnerの承認後にWindows設定またはStoreの正規経路から行う。
- uninstall後にRoom dataを残すか消すかは、表示された選択肢と文書どおりか確認する。
- data削除を伴う選択肢は、backupと復元手順を再確認するまで選ばない。

## 6. レーベルサイトへ正式Store linkを掲載する

Store Web pageを匿名で開け、実機installと起動が合格した後だけ掲載する。

対象repository:

```text
D:\desktop\TINMOON-Label-Site
```

M.I.O.の日本語／英語ページへ、同じStore IDの正式linkを追加する。

```text
https://apps.microsoft.com/detail/9NS9B7T71XHN
```

掲載文では次を明記する。

- Windows 11 x64向け
- Microsoft Storeから無料で取得できる
- 対応Providerと、Providerごとの契約・課金は別である
- Windows 10、macOS、Linux、Remote Relay、複数deviceはV1対象外
- privacy policy、support、sourceへのlink

編集後はlocal previewで日本語／英語、desktop／狭い幅、link先を確認する。
commit、push、Cloudflare Pages deployは別々に結果を確認し、deploy成功後に公開pageを再読込する。

## 7. 完了記録

次を`docs/V1-READINESS.md`と新しい引継ぎへ記録する。

- Partner Centerの最終statusと公開日時
- Store Web URLとStore app URIの確認結果
- package name、version、publisher、signature kind
- Store package versionと、実行ファイル／source versionが同期しているか
- install、起動、終了、再起動の結果
- NSIS版との共存有無と、実際のprocess／data path
- install前後のRoom数、message数、snapshot SHA-256
- 実施したsmokeと、未実施の試験
- レーベルサイトのcommit、deploy、公開URL
- 問題があった場合のerror文と、再試行していないこと

## 8. 公開直後の窓口とProvider変化を確認する

公開直後、翌日、1週間後を目安に、次を読取り専用で確認する。

- public repository `blackcometclub/M.I.O`の新規Issue
- private development repository `blackcometclub/M.I.O-dev`のsecurity alertと
  Private Vulnerability Reporting
- レーベルサイトのsupport窓口に届いたM.I.O.関連報告
- Codex、Claude Code、Gemini Antigravity、Grok CLIの起動可否、認証方式、model名、
  structured response形式に破壊的変更が告知されていないか

問題報告への返信、Issue操作、security advisory操作、Provider CLI更新は、この確認には
含めない。症状、version、発生日時、再現条件を記録してから別作業として扱う。

## 判定

### 2026-09-08の確認結果: PARTIAL

- **公開・実機更新**: Submission 4の公開package詳細で
  `M.I.O_1.0.4.0_x64_store-unsigned_20260907-220224.msix`、x64、minimum `10.0.22000.0`を確認した。
  Ownerが許可したMicrosoft Store appの「更新」操作が完了し、登録packageは
  `TINMOON.M.I.O_1.0.4.0_x64__kk3nwjsrdvfmr`、`SignatureKind: Store`、`Status: Ok`となった。
  `winget upgrade`は更新なしと表示したが、Store appには更新があり成功した。
  wingetの結果だけでStore配信なしと判断しない。
- **起動・終了**: Storeの「開く」から実画面とprocess pathを確認した。
  実行先は`C:\Program Files\WindowsApps\TINMOON.M.I.O_1.0.4.0_x64__kk3nwjsrdvfmr\mio-desktop.exe`。
  FileVersion／ProductVersionは`1.0.4`、SHA-256は
  `0213B7B352D06C517C05237111B678CE4AA3BDBED09ABAAA5496587283DB869D`で提出元の本体と一致した。
  通常終了後のprocess消滅と、その後の再起動を確認した。調査後は再起動済みであり終了状態ではない。
  NSIS版`C:\Program Files\M.I.O\mio-desktop.exe`も残している。
- **Room dataの差の原因**: 通常contextでは152,469 bytes／5 Room／316 message、Store package
  contextでは3,459 bytes／4 Room／7 messageのsnapshotを同じ論理pathから読み取った。
  Store側の生成日時は2026-09-06 13:21:40 JSTで、既存の会話が含まれていた。
  同じKnown Folder解決とfile読取を通常／package contextで比較して差を再現したため、
  MSIXのAppData仮想化による保存内容の分離と判断した。更新による履歴消失や、新規demoへの初期化とは
  判定しない。物理的な別保存pathは特定していない。
  参考: [MicrosoftのAppData仮想化の説明](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes#appdata-operations-on-windows-10-version-1903-and-later)。
- **保全とOwner方針**: 更新前に通常contextの`app.moe.desktop`を
  `D:\Backups\M.I.O\before-store-1.0.4.0_20260908-121303\app.moe.desktop`へコピーし、
  26 file／16,662,488 bytes、全fileのSHA-256一致を確認した。通常snapshotのhashは調査後も
  `999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`で変化しなかった。
  同じbackup rootへStore側snapshotを`store-visible-snapshot.json`として保存し、SHA-256
  `3D46A20C466A3E7514D42B08A69D309A58602D7A1E840C20AD1F597670171C1E`を確認した。
  `store-visible-data-report.json`と`rust-path-probe`も調査証拠として保持する。
  Store側全data領域のbackupや、更新前後のStore snapshot hash比較を完了したとは扱わない。
  OwnerはStore側の現状継続を選び、履歴の復元・移行・統合は不要とした。dataの上書きは実施していない。
- **公開導線**: BOOTHとitch.ioを非login browserで開き、`1.0.4`のdownload file名、容量、
  掲載SHA-256、無料／任意支援の表示を確認した。itch.ioの`1.0.4` devlogリンクも確認した。
  実downloadと取得物の再hash照合は未実施。公式紹介サイトの日英ページはStore／BOOTH／itch.ioへの
  正しいlinkを持つが、版数表示はまだStore `1.0.3.0`／直接配布 `1.0.3`であり更新が必要である。
  公開GitHub Releaseはalpha.1／alpha.2のみで、`v1.0.4` Releaseは未作成である。
- **残る確認**: 本書の最小smoke全項目、Provider設定・権限保持の全確認、clean Windows 11と
  WebView2なし環境の試験は完了扱いにしない。今回の記録更新はlocal文書のみで、site deployや
  GitHub Release公開を含まない。

### 同日の公開案内更新

2026-09-08の後続作業で、TINMOON日英ページの配布案内をStore `1.0.4.0`／直接配布 `1.0.4`へ
更新し、`1.0.4`と`1.0.3`の更新履歴を掲載した。サイトcommit `61af285`のCloudflare Pages checkは
成功し、日英公開ページのHTTP 200・新旧履歴・版数を確認した。BOOTHとitch.ioにも`1.0.4`の7項目と
Store公開済み案内を揃え、公開本文を非login browserで確認した。上記のサイト旧表記の課題は解消した。
未実施の実機smokeやclean環境試験は残るため、全体のPARTIAL判定は維持する。

### 判定基準

- **PASS**: 匿名Store page、Store版取得、package identity、起動・再起動、既存data、
  最小smoke、レーベルサイトlinkをすべて実証した。
- **PARTIAL**: Store公開は確認したが、実機、data、またはsiteの一部が未確認である。
- **FAIL**: 認定失敗、package identity不一致、起動不能、data破損などを再現した。
- **BLOCKED**: 公開処理中、地域反映待ち、Store取得不能など、再試行より待機が適切である。

未確認の段階を推測でPASSにしない。
