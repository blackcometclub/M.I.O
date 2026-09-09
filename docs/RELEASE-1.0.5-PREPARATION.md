# M.I.O. 1.0.5 配布準備

準備日: 2026-09-08。22:17 JSTにStore認定申請の受付を確認。公開は手動操作まで保留。

## 対象

- source commit: `3076294e1d87821afd1e18f36420d9162820679e`（開発repositoryのmainへpush済み）。
- 製品version `1.0.5`、Store package version `1.0.5.0`。
- 修正は最後に選択したRoomの再起動時復帰と、backup復元後の選択維持。
- npm／CargoのlockfileはM.I.O.自身の版番号だけを更新し、外部依存versionは変更していない。
- [日英の更新案内](RELEASE-NOTES-1.0.5.md)を準備した。Store申請の4言語へ反映済み。TINMOON、BOOTH、itch.io、公開GitHubへの反映は未実施。

## 検証と成果物

ローカル準備先: `D:\desktop\M.O.E\.tools\public-release-prep\1.0.5\3076294-candidate\`。
準備時の検証ログ: `D:\desktop\M.O.E\.tools\release-checks\1.0.5\`。
成果物は元checkoutに保管しており、後続worktreeの同名相対pathと混同しない。

- TypeScript typecheckとRoom復帰の14ケースは1.0.5候補でも成功。
- frontend build、`cargo fmt --all -- --check`、`cargo test --workspace --all-targets --locked --offline`が成功。
  Rustは349 passed／0 failed／24 ignored。ignored項目を実施済みとは扱わない。
- 既存の`build-alpha-windows.ps1 -Installer`で未署名NSISをbuildした。THIRD-PARTY-NOTICESは最新と一致。
- Store用は既存manifest templateからWindows SDK MakeAppxでpack／unpackし、9個の入力fileのhash一致を確認。
  identityは`TINMOON.M.I.O`、versionは`1.0.5.0`、architectureはx64。layoutと展開結果を準備先に保存した。
- BOOTH用ZIP内のinstallerと単体installerのSHA-256が一致した。
- source ZIPはcommit済みtreeの公開対象394ファイル。履歴・未追跡ファイルを含めない。
- Gitleaks 8.30.1によるsource scanは0 findings。
- native Room復帰の実機結果は[NEXT-VERSION-NOTES.md](NEXT-VERSION-NOTES.md)を参照。

2026-09-08 16:49 JSTに成果物の作成とpack照合が完了した。Rust全体テストもその後正常終了した。
成果物は上記準備先の`artifacts/`に置き、`candidate-manifest.json`、`validation.json`、
`REVIEW.txt`、`artifacts/SHA256SUMS.txt`へ詳細を保存した。

| 用途 | ファイル | bytes |
| --- | --- | ---: |
| itch.ioなど直接配布 | `M.I.O_1.0.5_windows-x64_unsigned-preview_setup.exe` | 4517965 |
| BOOTH | `M.I.O_1.0.5_windows-x64_unsigned-preview.zip` | 4502071 |
| Store提出候補 | `M.I.O_1.0.5.0_x64_store-unsigned.msix` | 6520938 |
| 公開source候補 | `mio-v1.0.5-source.zip` | 7570812 |

SHA-256:

- NSIS: `F3D08FF20B656F19C8C2DB672F201336695BC44EB5C43E7DC8AC2FAE458E8641`
- BOOTH ZIP: `4FC759E1D6377A6BA8EF39EFDDB186BCDF0B8C761FE2C8AFBDE226CB68E5629C`
- MSIX: `BFAA557D0DF89120A72C4A8923A0BE32A436052361F372B81C871651523A2607`
- source ZIP: `679600F6A4FBC3E524614BB4AFFB18F7B3EDD80EA11E48D693911DE0D486ECC5`

NSISはAuthenticode `NotSigned`の未署名Preview。MSIXはStore提出用の未署名候補で、Store署名済み製品ではない。
公開中の1.0.4のtag／artifact／Store版には変更を加えていない。

## 2026-09-08 通常版1.0.5の更新・実機再起動確認

- Ownerと共同で完成したNSIS候補を確認した。候補installerのSHA-256は上表と一致。
  既存通常版1.0.4に対して`Do not uninstall`を選択し、`C:\Program Files\M.I.O`へ更新した。
  20:33 JSTに導入された実体のProductVersionが`1.0.5`であることを確認した。
- Windowsの許可操作中はComputer Useを解除した。初回のComputer Use経由起動は
  `ShellExecuteW returned 5`で失敗し、通常版は1.0.4のままだった。
  再開後はComputer Useを接続せずにinstallerを起動し、Ownerが許可した。
  セットアップへの自動クリック・キー入力は反映されなかったため接続を解除し、
  Ownerが更新操作とFinishを実行した。手動操作を自動操作成功とは扱わない。
- 20:35–20:37 JSTに通常版の既存Roomを実画面で確認した。初回PID `17036`、
  window `1643742`で先頭以外の`AI Arena`を選択し、既存会話と宛先`ChatGPT`を確認して通常終了。
  M.I.O.とcommand helperのprocessが0件になった後、通常版exeを再起動した。
- 再起動PID `51016`、window `16779600`では、Roomを操作する前から`AI Arena`、
  同じ既存会話、宛先`ChatGPT`、`Core + Room ready`を表示した。再起動によるRoom復帰はPASS。
  その後も通常終了し、20:39 JSTにM.I.O.とcommand helperは0件。Computer Useも解除済み。
  実画面はこの実機確認を行った作業のComputer Use記録に保存されている。
- インストール済みexe、command helper、THIRD-PARTY-NOTICESの3ファイルは、
  NSIS候補から7-Zipで抽出した実体とすべてSHA-256一致。導入exeのSHA-256は
  `DF0796415842EB0B58F0474387AA5D7E3C9D70BF597CED56D0EF70D731D37CCE`。
  準備時の`candidate-manifest.json`の`executableSha256`はMSIX layoutの実体
  `9460D6FA61E902C5096A74736266172C6E9F4F609834D52C5EBFEBD650488235`に一致し、
  NSIS内exeのhashとは異なる。今回の実機結果をMSIXそのものの実機確認へ転用しない。

### バックアップとデータ照合

- 更新前に`D:\Backups\M.I.O\before-install-1.0.5_20260908-202105\`へ追加backupした。
  `normal`はRoaming/Localの1,183ファイル、159,114,664 bytes、`store`は339ファイル、
  43,437,253 bytes。Store側は`Invoke-CommandInDesktopPackage`の実行環境で取得した。
  全コピーで取得前・コピー先・取得後のhash一致を確認し、既存backupは上書きしていない。
- 試験後の通常版Roamingは26ファイルすべて一致し、追加ファイルなし。
  会話snapshotは152,469 bytes、SHA-256
  `999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`を維持した。
- Storeの実行環境で照合したRoamingは10ファイルすべて一致し、追加ファイルなし。
  会話snapshotは`ACC790BFB83F6704FD6B29F823E5CC5A2256D0356458392196E3F3B1742DE5CD`、
  participant設定は`753761AB4A8F9FB6C07AFD169AC2A1A6C3CD44D5CEC85C5C37EFF4B05D7F4E99`を維持。
- Storeから見えるLocalのWebView関連329ファイルでは302ファイルが一致し、27ファイルが不在。
  不在対象はLOG、BrowserMetrics、Session Storageなどで、詳細は`store-after.json`に記録。
  消失原因は未確定であり、Storeの全保存領域が不変とは報告しない。backupには元のコピーを保持した。
  通常版LocalのWebViewは起動とRoom選択の保存で変更されるため、不変の判定対象から分けている。
- ローカル証拠は、この検証を実施したworktreeの
  `C:\Users\black\.codex\worktrees\d1ee\M.O.E\.tools\v105-install-check\`に保管。
  `first-closed.json`、`first-relaunch.json`、`final-closed.json`、
  `installed-payload-verification.json`、`normal-after.json`、`store-after.json`を参照。
- AIへの新規送信・認証・Sandbox再試験は実施していない。表示した会話は既存履歴。
  新規Roomやbackup復元の追加実機操作は行わず、先に完了した14ケースと専用QAの結果を維持する。
  Store版は導入済み`1.0.4.0`のまま。Store提出用1.0.5.0の手動インストール・提出・公開は未実施。

## 2026-09-08 Store提出候補の静的確認

21:49 JSTまでに、既存MSIX候補の静的確認を完了した。この確認範囲では提出候補の不整合は見つからなかった。
候補自体の上書き・再署名・インストール・Storeアップロードは行っていない。

- MSIXは6,520,938 bytes、SHA-256は上表の`BFAA557D...`と一致した。
  Windows SDK 10.0.26100.0のMakeAppxで新しい作業用directoryへ展開し、同梱9ファイルが
  準備時layoutとすべてhash一致。AppxBlockMapの9ファイル・303ブロックもSHA-256を個別検証した。
  ZIP entriesは9入力とAppxBlockMap／Content Typesの計11件で、想定外の同梱物はなかった。
- 同じlayoutを別名の検証専用MSIXへpackし、MakeAppxのmanifest検証も成功した。
  この`validation-only.msix`は提出候補を差し替えるものではなく、提出対象は上表の原本のまま。
- Identity `TINMOON.M.I.O`、Publisher、x64、Application ID `MIO`は導入済みStore版と一致。
  manifest versionは`1.0.4.0`から`1.0.5.0`へ増加し、exe ProductVersionは`1.0.5`。
  最低OS build `10.0.22000.0`、日英resource、`runFullTrust`だけの権限宣言を維持した。
- 前回の提出用`M.I.O_1.0.4.0_x64_store-unsigned_20260907-220224.msix`と比べ、
  manifestの差分はversionだけだった。導入済みStore署名版にはPhoneIdentity要素とその名前空間が
  追加されており、その部分とversionを除くと今回のmanifestと一致した。
  Partner Centerの今回のアップロード検証結果は、まだ取得していない。
- LICENSE、THIRD-PARTY-NOTICES、4個のPNGは固定source `3076294`とbytes一致。
  PNG寸法はStoreLogo 50×50、ほかは44×44／71×71／150×150。
  manifestも同commitのtemplateへ既定のStore値と1.0.5.0を適用した内容と一致した。
- command helperのSHA-256は準備記録と一致し、MSIX内のmain exeに同じhashの文字列が埋め込まれている。
  これは静的照合であり、Store署名後のhelper起動試験とは区別する。

### NSISとMSIXのexeのhash差分を特定

- 両exeは18,113,024 bytes。全体を比較した結果、差分はoffset `14192656`からの3 bytesだけだった。
  NSIS内は`__TAURI_BUNDLE_TYPE_VAR_NSS`、MSIX内は`__TAURI_BUNDLE_TYPE_VAR_UNK`。
  そのほかの全bytes、実行コードsection `.text`、version resource等は一致した。
- 使用中の`tauri-utils 2.9.3`の`src/platform.rs`に同markerの定義と、`NSS`をNSISへ変換し、
  未指定値ではWindows上で`None`を返す処理を確認した。NSIS buildログにもbundle typeのpatch記録がある。
  別の実装が混入したことによるhash差ではなく、配布形式markerの差と確認できた。
- MSIXとしての更新・起動・Room復帰は、引き続きStore署名・配信後の確認対象。
  通常版の実機PASSをそのままStore版の実機PASSとして扱わない。
- 日英更新案内は実装内容と一致し、公開前の文案として保持した。
  前項のWebView関連27ファイルの不在は今回のpackage静的確認とは別の未確定事項として残す。

証拠: `C:\Users\black\.codex\worktrees\d1ee\M.O.E\.tools\v105-store-review-20260908\`の
`verification.json`、`executable-comparison.json`、`fixed-source-verification.json`、
`makeappx-unpack.log`、`makeappx-validation-pack.log`、導入済みStore版manifest。
Computer Useは接続せず、アプリの起動、AI送信、WACKの実行、Store提出・公開は行っていない。

## 2026-09-08 Store認定申請

22:17 JST、Partner CenterでSubmission 5の受付と「前処理中」を確認した。

- 製品ID: `9NS9B7T71XHN`、申請ID: `1152921505701836673`。
- 上表の原本`M.I.O_1.0.5.0_x64_store-unsigned.msix`をアップロードした。
  Partner Centerの解析後に`Validated`となり、提出パッケージとして保存された。
  検証専用の`validation-only.msix`は使用していない。
- 提出パッケージは1.0.5.0のみ。同じ対象の旧1.0.4.0は下書き内で置き換わった。
  対応デバイスはDesktopのまま、段階的公開・必須更新は有効化していない。
- Store登録情報の`ja-jp`、`ja`、`en-us`、`en`の「このバージョンの最新情報」へ
  Room復帰・backup復元後の選択維持・削除済みRoomのfallbackを記載した。
  4件とも保存後にページを再読込し、文面が残っていることを確認した。
  既存の説明・機能・スクリーンショットは維持した。
- 申請オプションを再読込し、「[今すぐ公開] を選択するまで、この提出物を公開しない」が
  選択されていることを確認してから「送信して認定を受ける」を実行した。
- 送信後の概要は「更新プログラムの認定中」「Submission 5: 認定中」、進行段階は
  申請完了／前処理中／認定未開始／公開処理未開始。
  同画面に手動公開待ちの案内があり、公開中は引き続きSubmission 4と確認できた。
- これは申請受付の確認であり、認定合格・Store署名版の導入・1.0.5の公開成功ではない。
  Store署名後の実機確認、ほかの配布先への反映は未実施。

再開先: [Partner Centerの製品概要](https://partner.microsoft.com/ja-jp/dashboard/products/9NS9B7T71XHN/overview)。
過去の「提出未実施」という記載は、その検証時点の記録として保持する。

## 2026-09-09 Store認定結果の確認

08:26 JSTまでに、Edgeで上記Partner Centerの製品概要を開き、Submission 5の認定合格を確認した。
Ownerの「今日はなにか一つだけ進めましょう」に対し、この時点では認定結果の読み取り確認と記録まで行った。

- 「製品が認定プロセスに合格しました」と表示された。
- 更新は「公開準備が完了しました (Submission 5: 最終変更日 2026/09/08)」。
  「今すぐ公開」を押すと公開処理が開始する案内と、公開処理未開始の表示を確認した。
- Microsoft Storeのプレゼンスは引き続きSubmission 4（最終変更日2026/09/07）だった。
  Submission 5は手動公開待ちであり、1.0.5.0が公開・配信されたとは扱わない。
- 「今すぐ公開」「認定の取り消し」や公開時刻の変更は操作していない。
  確認用に作成したブラウザータブは閉じ、操作sessionも解除した。
- アプリの更新・起動、AI送信、Sandbox再試験、他の配布先への変更は行っていない。

## 2026-09-09 Store公開操作

08:33–08:34 JST、Ownerの「公開しちゃいましょう。全て許可するのでボタンおしちゃってー！」を受け、
Partner Centerで認定済みのSubmission 5を再確認し、「今すぐ公開」を1回押した。

- 再読込後に「更新プログラムの公開処理中」と表示された。
- 認定ステータスは合格のまま、「公開処理が開始され、間もなく Microsoft Store で提供されるようになります」
  という案内を確認した。Publishing Statusは4、公開段階はIn progress。
- 対象はSubmission 5で、手動公開の保留から公開処理中へ移った。
  同時点のMicrosoft Storeプレゼンス欄はまだSubmission 4だった。
- 公開操作の受付と処理開始を確認したのであり、1.0.5.0の配信完了・PCへの更新成功は未確認。
- 結果のページをEdgeに残し、操作sessionは解除した。
  この操作では他の配布先の更新や、アプリのインストール・AI送信は行っていない。

## 2026-09-09 Store配信完了・BOOTH／itch.io公開

09:02 JSTまでに、Microsoft Store、TINMOON公式サイト、BOOTH、itch.ioで1.0.5の公開反映を確認した。

- Partner Centerの製品概要に「お客様の製品が更新されました」「お客様の最新の製品が Microsoft Store で入手可能になりました」と表示された。
  Microsoft StoreのプレゼンスはSubmission 5（最終変更日2026/09/08）へ更新されていた。
- TINMOON公式サイトの日英M.I.O.ページをv1.0.5へ更新し、Store版v1.0.5.0、直接配布版v1.0.5、
  Room復帰の更新履歴を掲載した。1.0.4と1.0.3の履歴は保持した。
  `TINMOON-Label-Site`のcommit `5645f49`を`origin/main`へpushし、公開ページの日英両方で反映を確認した。
- BOOTHの商品名を`M.I.O. v1.0.5｜複数AIトークルーム（Windows 11）`へ変更し、
  `M.I.O_1.0.5_windows-x64_unsigned-preview.zip`（4,502,071 bytes、SHA-256
  `4FC759E1D6377A6BA8EF39EFDDB186BCDF0B8C761FE2C8AFBDE226CB68E5629C`）を配布対象にした。
  1.0.4のファイルは削除せず配布対象から外した。商品説明のinstall情報と更新履歴へ1.0.5を追記し、過去の履歴を保持した。
- BOOTH公開ページで、商品名・配布ファイル・`v1.0.5（2026-09-09）`の履歴を確認し、配布欄に1.0.4が表示されないことを確認した。
- itch.ioへ`M.I.O_1.0.5_windows-x64_unsigned-preview_setup.exe`（4,517,965 bytes、SHA-256
  `F3D08FF20B656F19C8C2DB672F201336695BC44EB5C43E7DC8AC2FAE458E8641`）をアップロードし、Windows向け公開ファイルにした。
  1.0.4以前のファイルは削除せず非表示のまま保持した。
- itch.ioの商品説明・install案内を1.0.5のfile名、version、容量、SHA-256、source commit
  `3076294e1d87821afd1e18f36420d9162820679e`へ更新し、英語の1.0.5更新履歴を過去履歴の前へ追加した。
- itch.io Devlog `M.I.O. v1.0.5 is now available`を公開し、1.0.5のEXEを添付した。
  公開URLは`https://tinmoon-label.itch.io/mio-talk-room/devlog/1657229/mio-v105-is-now-available`。
- itch.io公開ページで1.0.5のfile名・SHA-256・release status・更新履歴・Devlogを確認し、download欄に1.0.4が表示されないことを確認した。
- BOOTHとitch.ioの直接配布は引き続き未署名Previewであり、SmartScreen警告、Authenticode `NotSigned`、
  Microsoft Store版への案内を保持した。Computer Useは作業後に終了した。

## 2026-09-09 Store署名版1.0.5.0の実機更新・Room復帰確認

09:16 JSTまでに、このPCへ導入済みだったStore署名版を`1.0.4.0`から`1.0.5.0`へ更新した。
09:22 JSTまでに、Store署名版の実画面で非先頭Roomの再起動時復帰を確認し、PASSと判定した。

- 更新前はpackage `TINMOON.M.I.O_1.0.4.0_x64__kk3nwjsrdvfmr`、status `Ok`だった。
  Windowsの`ScanForUpdatesAsUser`は終了code 0だったが、その時点ではversionは変わらなかった。
- Windows公式`AppInstallManager.UpdateAppByPackageFamilyNameAsync`をpackage family
  `TINMOON.M.I.O_kk3nwjsrdvfmr`へ実行した。返されたproduct IDは`9NS9B7T71XHN`で、
  update queueの初期stateは`Downloading`だった。
- 更新後はpackage `TINMOON.M.I.O_1.0.5.0_x64__kk3nwjsrdvfmr`、status `Ok`、
  install先は`C:\Program Files\WindowsApps\TINMOON.M.I.O_1.0.5.0_x64__kk3nwjsrdvfmr`だった。
- Store版を起動し、先頭の`M.I.O.開発室`から非先頭の`回答くらべ部屋`へ切り替えた。
  Room名、既存message、宛先`ChatGPT`、`Core + Room ready`を実画面で確認して通常終了した。
  [終了前の実画面](assets/screenshots/mio-store-1.0.5-room-before-restart.png)を保存した。
- 同じStore App ID `TINMOON.M.I.O_kk3nwjsrdvfmr!MIO`から再起動した。
  Roomを操作する前から`回答くらべ部屋`、同じ既存message、宛先`ChatGPT`、
  `Core + Room ready`を表示した。[再起動直後の実画面](assets/screenshots/mio-store-1.0.5-room-after-restart.png)を保存した。
- AIへの送信、新規Room作成、backup復元、通常版の起動は行っていない。確認後はStore版を通常終了した。

## 2026-09-09 GitHub v1.0.5公開

10:56 JST、公開repositoryへsource-onlyの通常Release `v1.0.5`を公開した。

- private source commitは`c6631dd301312c19a8ef0cd6d692c8866c55f443`、public release commitは
  `887a6a1dd2478fc590784a5ba78f8a66278e4da6`。public `main`とannotated tagをatomic pushした。
- 公開snapshotは397ファイル。public commitの全blobを準備済みsnapshotと照合し、397件すべて一致した。
- source ZIP `mio-v1.0.5-source.zip`は7,956,967 bytes、SHA-256は
  `E2BE040569C0525DBAE1C4DDAA620D175C1DD5C4565AF0DE3787E26703C40725`。
- source ZIP、`snapshot-manifest.json`、`SHA256SUMS.txt`の3点を添付した。未署名installerは添付していない。
- Releaseは`draft: false`、`prerelease: false`。匿名GitHub APIでLatest `v1.0.5`を取得し、
  3添付物を認証なしでdownloadしてlocal hashと全件一致した。
- [`v1.0.5` Release](https://github.com/blackcometclub/M.I.O/releases/tag/v1.0.5)
- 公開直後のDependabot APIは旧`fast-uri`／`qs`警告を残していた。公開lockfileは修正版なので、
  server側の再解析後に再確認する。`glib`はWindows targetの依存に含まれないため、根拠なくdismissしない。

## 完了後の扱い

1. 1.0.5の公開と通常版／Store署名版の実機確認は完了。現在残っている作業と追加機能候補は
   [現在地カード](CURRENT-STATUS-CARD.md)から再開する。

通常版NSISとStore署名版の両方で、1.0.5への更新・再起動・Room復帰を確認済み。
候補rootの`candidate-manifest.json`と`validation.json`は準備時点の記録として保持し、
その後の実機結果は上記の追記とworktree側の証拠を再開地点とする。
Sandbox内のAI設定・認証・送信はOwner指定により省略したままとする。
