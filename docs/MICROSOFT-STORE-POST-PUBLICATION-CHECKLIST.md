# M.I.O. Microsoft Store公開後チェックリスト

この文書は、Microsoft Storeの初回公開および更新公開の後に、
公開ページ、Store版の取得、Windows 11実機、既存Room data、レーベルサイトを
順番に確認するための手順である。

## 最新の整理

2026-09-08 16:04 JST時点の完了・省略・次期対応とGitHub警告の照合結果は、
[1.0.4公開後確認の整理](RELEASE-1.0.4-CLOSEOUT.md)を参照。
今回合意した公開後確認は完了し、SandboxはOwner依頼で終了済み。AI試験の省略とRoom自動復帰の次期版繰越を含むため、
本書の元の全項目基準を一括PASSには変更しない。以下の過去の「残り」は各記録時点の状態である。

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

同日13:50 JST、Owner確認後にGitHubの[通常Release v1.0.4](https://github.com/blackcometclub/M.I.O/releases/tag/v1.0.4)
を公開した。public source commitは`1376ec45771c3d73553c823f5cdbc6b6645cb549`。Store版への案内と
source ZIP／manifest／SHA256SUMSの3添付物を掲載し、匿名の実downloadとSHA-256一致を確認した。
Latestも`v1.0.4`となり、上記のGitHub Release未作成の課題は解消した。

### 2026-09-08 14:39–14:43 JSTの実機smokeと配布確認

- **対象**: 登録packageは`TINMOON.M.I.O_1.0.4.0_x64__kk3nwjsrdvfmr`、`SignatureKind: Store`、
  `Status: Ok`。実行processも同じWindowsApps配下の`mio-desktop.exe`だった。
- **画面で確認**: `M.I.O.開発室`から既存の`MCP実験室`へ切り替え、元のRoomと会話表示へ戻った。
  Room settingsとPreferences／外観panelの開閉、Room一覧の開閉、画像表示／画像なし表示の往復を確認した。
  最後は元のRoom、Room一覧あり、画像なし表示へ戻した。Provider設定やworkspace権限は変更していない。
- **Store側backup**: Room settingsの保存先は`D:\Documents\M.O.E Backups`。
  `Back up`を1回実行して`Backed up 4 rooms`を確認した。生成fileは
  `D:\Documents\M.O.E Backups\moe-room-backup-00000001788846045519.json`、3,459 bytes／4 Room／7 message。
  SHA-256は`3D46A20C466A3E7514D42B08A69D309A58602D7A1E840C20AD1F597670171C1E`で、
  同日午前に保全したStore snapshotとbyte-identicalだった。全data領域のbackupではなくRoom backupである。
  通常contextのsnapshotも`999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`を確認した。
- **終了・再起動**: windowの通常終了後、本体とhelperのprocess不在を確認した。登録済みStore app IDで
  再起動し、新processが同じStore package配下から動くことと、元のRoom・会話・画像なし表示の保持を実画面で確認した。
  再起動後の追加backupを準備する際にuser入力が検知されたため、以後のnative操作を中止し、操作sessionをresetした。
  再起動後のbackup再取得・hash比較は未実施。アプリは再起動した状態で残している。
- **証拠の制限**: 最初のoccluded window captureは別画面を示したため採用せず、対象を再選択・前面化した後の
  M.I.O. screenshotだけをsmoke証拠とした。UI Automationのaccessibilityはnullで、実画面と座標による確認だった。
- **配布取得**: itch.ioは非login browserで`No thanks, just take me to the downloads`から無料取得画面へ進み、
  `1.0.4` installerのDownloadを実行したが保存fileは確認できず、取得・hash照合は未完了。
  BOOTHは非login時にlogin画面へ遷移した。既存loginのEdgeで公開商品の無料downloadを実行したところ、
  `s6.booth.pm`で`このページは Microsoft Edge によってブロックされました`／`ERR_BLOCKED_BY_CLIENT`となった。
  ブロック原因は未特定で、解除や別経路による回避は行っていない。確認用に作成した3 tabは閉じた。
- **残り**: 実download・hash照合、Codex Direct実reply、Provider設定・権限保持の全確認、clean Windows 11と
  WebView2なし環境の試験。実Provider送信、課金、workspace書込み、install／uninstallは今回実施していない。
  最小smokeの表示操作は確認できたが、全体の**PARTIAL**判定を維持する。

### 2026-09-08 14:51 JSTの配布取得再確認

Ownerの再試行依頼により、既存loginのEdgeで両公開ページの無料downloadを再実行した。
downloadイベントの監視を併用し、Windowsの既定download先が`D:\Downloads`であることを確認した。

- **BOOTH: 取得・照合PASS**。公開商品の無料downloadから
  `D:\Downloads\M.I.O_1.0.4_windows-x64_unsigned-preview.zip`を取得した。
  4,500,699 bytes／SHA-256 `06D8F4262DBAE3BF1D43590D6AB1E38232EAF452528BF9163F1D1F32D218F9A1`で、
  公開前のlocal原本と掲載値に一致した。今回はブロックは再現せず、設定変更・警告解除はしていない。
  前回のブロック原因自体は確定していない。
- **itch.io: 受信bytesの照合PASS、browser保存完了は保留**。支援金なしの取得経路で
  `D:\Downloads\未確認 915836.crdownload`が生成された。
  4,516,598 bytes／SHA-256 `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`で、
  公開前のinstaller原本と掲載値に一致した。BOOTH ZIP内のinstallerもstreamでhashし、同じ値だった。
  一時fileのrename・実行はしていない。ダウンロード一覧の`edge://downloads/`はbrowser toolのURL policyで
  拒否されたため別経路で操作せず、OwnerにCtrl+Jで表示文言を確認するよう依頼した。
  実際の警告内容・保存保留理由は未確認であり、SmartScreen等が原因と断定しない。
- 読取り照合結果は`.tools/public-release-prep/1.0.4/download-recheck-20260908-1451/verification.json`。
  前回の「両方とも受信file未確認」はこの再検証で解消した。両媒体の配布bytesは原本と一致するが、
  itch.ioの通常file名での保存完了と、実Provider／clean環境試験は未完了である。

### 2026-09-08 Owner提示のダウンロード警告

Ownerのスクリーンショットで、itch.ioのinstallerに対してEdgeが
`一般的にダウンロードされていません`／`開く前に、信頼できることを確認してください`と表示していることを確認した。
これで保存保留時の実際の表示が判明した。この表示をmalware検出通知とは扱わない。
配布bytesの原本一致は前項で確認済み。通常file名への保存完了や警告解除・実行は確認していない。

### 2026-09-08 15:08–15:11 JSTのStore版Codex Direct試験

- Ownerの再開依頼後、Store `1.0.4.0`の新規Room `New room 5`
  （`room-3169414b-3ec1-432b-b6c9-f8a20b7ecd57`）で、宛先Codexのみ・Conductorなしの固定返信試験を行った。
  このRoomは前のturnで作成済みの空Roomで、既存Roomの会話を試験messageへ含めていない。
- 送信文は`Reply with exactly MIO_STORE_104_OK. Do not use tools, read files, generate images, or perform any other action.`。
  UI操作とuser入力の競合通知後、再送せず画面を読取り確認したところ、Owner message 1件とCodex reply 1件が存在した。
  Owner messageは`2026-09-08T06:08:41.883Z`、replyは`2026-09-08T06:08:48.176Z`。
  reply本文は正確に`MIO_STORE_104_OK`で、artifactは0件。実replyの表示と保存は**PASS**。
- Room設定から全Room backupを保存し、UIの`Backed up 5 rooms`を確認した。
  `D:\Documents\M.O.E Backups\moe-room-backup-00000001788847771789.json`は4,120 bytes／5 Room／9 message、
  SHA-256 `ACC790BFB83F6704FD6B29F823E5CC5A2256D0356458392196E3F3B1742DE5CD`。
  前回backupの既存4 RoomをIDごとにJSON比較し、全項目の一致を確認した。増えたのは新Roomと試験2 messageだけである。
  通常contextのsnapshot hashも`999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`のままだった。
- 通常終了後は本体・helperともprocess 0件。登録Store app IDから再起動し、WindowsApps配下の
  `TINMOON.M.I.O_1.0.4.0_x64__kk3nwjsrdvfmr\mio-desktop.exe`が動作することを確認した。
  起動直後は先頭の`M.I.O.開発室`が開き、`New room 5`を選択すると上記2 messageが再表示された。
  **会話の永続化はPASS、最後に開いたRoomの自動復帰は未達**。本書の最小smokeを全項目PASSとは扱わない。
- sourceの`useRooms.ts`はactive Roomを`initialRooms[0].id`で初期化し、hydrate時も現在IDがなければ
  先頭Roomを採用する。最後の選択Roomを保存・復元する処理は同hookにない。次の改善候補として記録し、
  今回は実装変更・再build・配布差替えをしていない。
- 最後はテストRoomと返信を表示したままnative操作sessionをresetした。テストRoom・backupを削除していない。
  Provider model・権限保持の全確認、clean Windows／WebView2なし環境の試験は別途残る。

### 2026-09-08 次期バージョンへの繰越決定

Ownerの指示により、最後に選択したRoomの自動復帰は次期バージョンで対応する。
現象・実装の手掛かり・受入条件を`docs/NEXT-VERSION-NOTES.md`に固定した。
1.0.4の製品codeや配布fileは変更せず、残るclean Windows／WebView2試験を続ける。

### 2026-09-08 15:26–15:34 JSTのclean Windows／WebView2試験

- **直接配布1.0.4のclean install・初回起動・通常終了・再起動: PASS**。
  新規Windows Sandbox（Windows 11 Enterprise x64、build `26100`）を使った。
  Store packageのclean取得試験ではなく、BOOTH／itch.ioで配布中のNSIS installerの試験である。
- install前に、M.I.O.のHKLM／HKCU uninstall登録、標準install先の本体、Room AppDataが存在しないことを確認した。
  WebView2もHKLM／HKCUのRuntime登録と標準配置先の実行fileがともに存在しなかった。
- 公開原本から複製した`M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`をguest側で再hashし、
  `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`の一致を確認した。
  入力folderはread-only、書込み可能な共有先は今回専用の結果folderだけ。認証情報や既存Roomをguestへ渡していない。
- Microsoft公式の`wsb exec`から`/S`でinstallし、終了code `0`を確認した。
  **WebView2未導入からの公式bootstrapper経由の導入: PASS**。Runtime `152.0.4191.66`が登録された。
  installerが取得したbootstrapperは署名`Valid`、署名者`Microsoft Corporation`、SHA-256
  `17DEBF797A6C737959BC588236E897936FFAC1AF5F7E515E674AB32F9EDFE719`だった。
- guestの`C:\Program Files\M.I.O\mio-desktop.exe`はProductVersion `1.0.4`、SHA-256
  `94CC1B3A43FCF71E097F404A1B1D33523F2C47BA02E6642EA81E9B610E523EC4`。
  15:32 JSTに初回の実画面でRoom一覧、日本語の初期サンプル会話、`Core + Room ready`を確認した。
  サンプルには`UI DEMO`表示があり、Providerの実reply試験とは扱わない。
- 本体右上の終了ボタンから通常終了し、guestの`mio-desktop` process不在を確認した。
  再起動後は新PID `2420`、version `1.0.4`で動作し、15:33 JSTの実画面で同じ初期Roomとサンプル会話が再表示された。
  初回PIDは`1936`。終了記録は旧名`moe-command-helper`を対象にしており、現行名`mio-command-helper`の残留確認は含まない。
- 最初のLogonCommandのPS1はguestの既定ExecutionPolicy `Restricted`で実行されなかった。
  policyは変更せず、公式CLIで読取り診断・installer実行・起動を分けて行った。installの重複実行はしていない。
  初回画面確認前のOwnerによるEsc停止では操作を中止し、Ownerの再開依頼後に実画面を確認した。
  以後は短い画面操作ごとにComputer Use sessionをresetし、記録作業中は使用していない。
- host側は既存Store版`1.0.4.0`の同一PID `26516`、package identity、WebView2 versionを保持した。
  通常contextのRoom snapshotもSHA-256 `999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`で不変。
  guestは再起動したM.I.O.を残しており、Sandboxを破棄していない。
- 証拠は`.tools/public-release-prep/1.0.4/clean-sandbox-20260908-1523/`の`results/baseline.json`、
  `installed.json`、`normal-close.json`、`restart.json`とhost前後記録。結果JSONはlocalに保全する。
  Provider model・権限保持の全確認は未完了。最後のRoom自動復帰は次期版へ繰越のため、公開後チェック全体は**PARTIAL**を維持する。

### 2026-09-08 15:38–15:44 JSTのStore設定保持確認と試験範囲

- Ownerは「サンドボックスでのAIの確認は無し」と指定した。SandboxのProvider CLI導入・認証・AI送信試験は
  **Owner方針により省略**する。未設定を障害や再試行待ちとは扱わず、前項のclean install／WebView2試験結果を保持する。
- **使用中のCodex設定の再起動後保持: PASS**。Store package contextから保存fileを読取り、通常終了の前後で照合した。
  `participant-profiles-v1.json`は170 bytesで、保存されていた設定はCodexの`gpt-5.6-sol`／`chatOnly`だけだった。
  SHA-256は前後とも`753761AB4A8F9FB6C07AFD169AC2A1A6C3CD44D5CEC85C5C37EFF4B05D7F4E99`。
- 再起動前後の実画面でCodexのModel `gpt-5.6-sol`、選択中の`Conversation only`を確認した。
  `Files: Blocked`／`Commands: Blocked`／`Web and network: Blocked`の表示も一致していた。
  `New room 5`のRoom設定はworkspace未選択、`No conductor`のまま。プロフィールや権限の変更・保存は行っていない。
- `room-workspaces-v1.json`は前後とも存在せず、workspace未設定の保持を確認した。
  `room-snapshot-v1.json`は前後とも4,120 bytes、SHA-256
  `ACC790BFB83F6704FD6B29F823E5CC5A2256D0356458392196E3F3B1742DE5CD`で、全Room・会話の保存bytesが一致した。
  通常context側snapshotも既存の`999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`で不変だった。
- 通常終了後にhostの`mio-desktop`／`mio-command-helper`／`moe-command-helper`のprocess不在を確認した。
  OwnerのEsc停止後は操作を中止し、再開依頼後にStore app IDで再起動した。新PID `40212`はStore `1.0.4.0`の
  WindowsApps配下から動き、packageは引き続きStore署名／Status Okだった。
  起動直後は先頭Roomへ戻る既知の動作だったため`New room 5`を選び直し、保存済み試験replyの再表示も確認した。
- 最後はプロフィールを変更せず閉じ、`New room 5`を表示したままComputer Use sessionをresetした。
  この確認ではhost／Sandboxとも新しいAI送信、CLI認証、workspace read／writeを行っていない。
- 証拠は`.tools/public-release-prep/1.0.4/store-settings-20260908-1541/`の`before.json`、`closed.json`、
  `after.json`、`comparison.json`。folder名の時刻と実測日時は異なり、実測は各JSONのUTC timestampを採用する。
  既存Codex設定の保持を確認したので、この範囲の再試験は不要。未設定の他Providerやworkspace read／writeの
  全組合せを今回検証したとは扱わない。Room自動復帰の次期版繰越を含め、元の全項目基準の判定は**PARTIAL**を維持する。

### 判定基準

- **PASS**: 匿名Store page、Store版取得、package identity、起動・再起動、既存data、
  最小smoke、レーベルサイトlinkをすべて実証した。
- **PARTIAL**: Store公開は確認したが、実機、data、またはsiteの一部が未確認である。
- **FAIL**: 認定失敗、package identity不一致、起動不能、data破損などを再現した。
- **BLOCKED**: 公開処理中、地域反映待ち、Store取得不能など、再試行より待機が適切である。

未確認の段階を推測でPASSにしない。
