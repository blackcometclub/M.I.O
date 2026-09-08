# M.I.O. BOOTH／itch.io公開チェックリスト

この手順は、Microsoft Store審査とBOOTH／itch.ioの公開を別の承認境界として扱う。
ページ草案の作成は公開ではない。file upload、下書き保存、公開は、それぞれ現在の画面と
対象を確認してから行う。

## 2026-09-08: 公開本文の更新履歴を同期

- Ownerの追加依頼により、TINMOON日英ページ、BOOTH日本語本文、itch.io英語本文に`v1.0.4`の
  更新履歴7項目とStore `1.0.4.0`公開済み案内を揃えた。過去の`v1.0.3`履歴は保持した。
- BOOTH item `8807279`は既存の7項目を読みやすい日本語へ揃え、公開保存後、非login browserで
  本文・0円・`1.0.4` ZIPのdownload表示を確認した。
- itch.io game `4974122`は本文に欠けていた`1.0.4`履歴を追加した。最初のHTML編集は公開へ反映されず、
  保存結果を確認してから再入力し、通常editorで入力を確定して保存した。最終的に非login browserで
  新旧履歴と`1.0.4` installerの表示を確認した。devlogの新規作成・重複投稿は行っていない。
- TINMOON site commit `61af285d69bd72d71977309a26ca651b1a249e04`を`main`へpushした。
  Astro buildで23ページを生成し、Cloudflare Pages checkが成功した。日英公開URLはHTTP 200、
  Store `1.0.4.0`／直接配布 `1.0.4`、新旧更新履歴の表示を確認した。
- 配布file、価格、画像の変更や新規uploadは行っていない。実downloadと取得物の再hash試験は未実施。

## 1. 配布方式を決める

Microsoft Store認定後、次のいずれかをOwnerが選ぶ。

### A. Storeを正式なinstall元にする

- BOOTH／itch.ioは紹介と任意支援のページにする
- 正式な取得buttonはMicrosoft Storeへ案内する
- Storeへ提出した未署名MSIXをBOOTH／itch.ioへ再配布しない
- 誤解を避けるため、支援しなくてもStore版を無料で利用できると明記する

### B. 直接downloadも用意する

- Store packageとは別に、配布可能なinstallerを同じsource commitから作る
- Authenticode署名済みを優先する
- 未署名版を採用する場合は、Ownerが別途決定し、未署名、SmartScreen、SHA-256、source commitを
  download直前に明記する
- Store版と直接配布版のversion、data path、update方法の違いを記録する

方式A／Bを決めるまで、実行fileをuploadしない。

## 2. Page素材を確認する

- [x] BOOTH本文を日本語で最終確認する
- [x] itch.io本文を英語版のみで最終確認する
- [x] `M.I.O.`、`TINMOON`、`v1.0.0`の表記を統一する
- [x] 64-bit Windows 11、WebView2、Provider CLIの必要条件を明記する
- [x] AI利用料とProvider規約がM.I.O.の配布価格に含まれないことを明記する
- [x] 既存の5 screenshotが現在の製品UIと一致する
- [x] itch.io coverを`315:250`比率で作る。`1260 x 1000`で生成済み
- [x] BOOTH用の正方形coverを`1200 x 1200`で生成済み
- [x] screenshotの会話は撮影用デモであると注記する

## 3. Artifactを固定する

- [x] 配布file名、size、SHA-256を記録する
- [x] source commitとversionを記録する
- [x] 署名status、publisher、timestampを確認する
- [x] Defenderで最終配布fileをscanする
- [ ] cleanなWindows 11でdownload、install、起動、終了を確認する
- [ ] 既存Room dataとbackupが失われないことを確認する
- [ ] 両ページのfileが同一ならSHA-256一致を確認する

2026-09-06に直接配布方式Bの`1.0.3`未署名Previewを、Ownerの個別承認後にBOOTH／itch.ioへ
公開した。Microsoft Storeの公開packageとは別の直接配布物である。

- Source commit: `7ff8b654cfa2a60c97c76e509b940d9993fdb8e6`
- Product version: `1.0.3`
- Installer: `M.I.O_1.0.3_windows-x64_unsigned-preview_setup.exe`
- Installer size: 4,520,172 bytes
- Installer SHA-256: `F9EF0CB56255216B82C92F7D9754FCF638A0B758BADEB9230C5BFD148ED034DE`
- Authenticode: `NotSigned`; publisherなし、timestampなし
- BOOTH ZIP: `M.I.O_1.0.3_windows-x64_unsigned-preview.zip`
- BOOTH ZIP size: 4,504,270 bytes
- BOOTH ZIP SHA-256: `C0F8AF6B135BA1CC9EA5E83586FE7617A1CAAD17F89A97863E5321412DB6FC9F`
- 準備先: `.tools/public-release-prep/1.0.3/7ff8b65-direct-unsigned-preview-20260906-212359/`

直接配布へ進む場合は、download直前に`Unsigned Preview`、SmartScreen警告の可能性、installer
SHA-256、source commitを表示する。Store版はMicrosoft Storeが署名する別packageであり、この未署名
NSISをStore版と表示しない。

2026-09-07、`1.0.4`の未署名直接配布artifactをlocalで固定し、Ownerのupload・保存・公開の
明示承認後にBOOTH／itch.ioへ公開した。

- Source commit: `fbc81b89ee64a670f13128487c06a96bfabe3238`
- Product version: `1.0.4`
- Installer: `M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`
- Installer size: 4,516,598 bytes
- Installer SHA-256: `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`
- Authenticode: `NotSigned`; publisherなし、timestampなし
- BOOTH ZIP: `M.I.O_1.0.4_windows-x64_unsigned-preview.zip`
- BOOTH ZIP size: 4,500,699 bytes
- BOOTH ZIP SHA-256: `06D8F4262DBAE3BF1D43590D6AB1E38232EAF452528BF9163F1D1F32D218F9A1`
- Source ZIP: `mio-v1.0.4-source.zip`（396 tracked files、Git履歴なし）
- Source ZIP size: 7,583,961 bytes
- Source ZIP SHA-256: `D5E520DC2108A27E22151271AA1B9A4473A20B58856E1C5F9EE4ED603BBF9E86`
- Gitleaks 8.30.1: 0 findings
- 準備先: `.tools/public-release-prep/1.0.4/fbc81b8-direct-unsigned-preview-20260907-224600/`

## 4. BOOTHを公開・更新する

- [x] ダウンロード商品／ソフトウェアとして登録する
- [x] 無料配布を基本とし、支援は任意と説明する
- [x] 非公開Draftへ直接配布候補ZIPを設定する
- [x] 商品画像と5 screenshotを登録する
- [x] 下書きで保存し、reload後も本文と画像が保持されていることを確認する
- [x] 公開buttonはOwnerの明示承認まで押さない

2026-09-05、Ownerの明示承認後に
`M.I.O_1.0.0_windows-x64_unsigned-preview.zip`（4,496,512 bytes、SHA-256
`61FAC1FF36BAE77CE4FF9B57F03B44C19129966C6427C85CF948B4C461D4C961`）をuploadした。
ZIPは同じinstaller、日英`README.txt`、installer用`SHA256SUMS.txt`だけを含む。BOOTH上の
downloadable IDは`9597654`。対象checkを有効にして`下書きで保存する`を実行し、reload後も
作品file欄に残ることを確認した。続けて未署名Preview、SmartScreen、ZIPとinstallerのSHA-256、
source commit、backup／install手順を日本語本文へ反映し、再度下書き保存した。保存後の画面で本文と
作品fileを確認し、`公開で保存する`は押していない。

同日の公開前previewで、BOOTH初期templateの`公式サイト`、`詳細`、`動作環境`、`アップデート履歴`
という4つのdummy段落が商品説明の後ろに残っていることを発見した。Ownerの削除承認後に4段落だけを
削除して下書き保存し、reload後のpreviewでdummy文が0件、ZIP、SmartScreen説明、ZIP／installerの
SHA-256が保持されていることを確認した。商品は引き続き非公開である。

BOOTH公式では無料配布にサービス利用料は発生しない。BOOSTは商品代金への任意上乗せで、
BOOST額にはサービス利用料がかかる。公開時点の画面表示を再確認する。

2026-09-06、Ownerの明示承認後に公開ページを`v1.0.3`へ更新した。新しいZIPのdownloadable IDは
`9606110`で、対象checkを有効にした。旧`1.0.0` fileは削除せず対象checkを外して保持した。商品名、
本文のfile名、size、SHA-256、source commit、署名説明を`1.0.3`へ揃え、同日の更新履歴を追記した。
公開ページ `https://tinmoon.booth.pm/items/8807279` で、0円、`v1.0.3`の表題、更新履歴、
`M.I.O_1.0.3_windows-x64_unsigned-preview.zip`だけが無料download対象であることを確認した。

2026-09-07、Ownerの明示承認後に公開ページを`v1.0.4`へ更新した。ZIPのdownloadable IDは
`9612396`で、対象checkを有効にした。旧fileは削除せず対象checkを外して保持した。商品名、本文、
更新履歴、ZIP／installerのsize・SHA-256、source commit、未署名説明を`1.0.4`へ揃え、
`公開で保存する`を実行した。公開ページ `https://tinmoon.booth.pm/items/8807279` で、0円、
`v1.0.4`の表題、更新履歴、`M.I.O_1.0.4_windows-x64_unsigned-preview.zip`がdownload対象で
あることを確認した。

## 5. itch.ioを公開・更新する

- [x] KindをDownloadable、PlatformをWindowsにする
- [x] `$0 or Donate`を選び、suggested donationを`$2.00`にする
- [x] cover、5 screenshot、英語本文を設定する
- [x] 非公開Draftへ直接配布候補fileを設定する
- [x] pageをDraft／Restrictedのまま保存し、reload後に内容を確認する
- [x] Publicへの変更はOwnerの明示承認まで行わない

2026-09-05、Ownerの明示承認後に
`M.I.O_1.0.0_windows-x64_unsigned-preview_setup.exe`をuploadし、`Success`／`Executable`、
表示size 4 MB、download 0を確認した。file固有のWindows platformを選び、
`Hide this file and prevent it from being downloaded`を解除した。未署名Preview、SmartScreen、
installerのSHA-256、source commitを英語本文とdownload／install説明へ反映してDraft保存した。
reload後もfile、Windows指定、download表示、英語本文、install説明、Draft状態が保持されていた。
Publicは選択していない。

itch.io公式では最低価格0でも任意支援を受けられる。無料downloadを選んだ利用者には
ownership keyが付かず、支払った利用者には付くため、将来有料化する場合は影響を再確認する。

2026-09-06、Ownerの明示承認後に公開ページを`v1.0.3`へ更新した。英語本文とdownload／install
説明のfile名、version、size、SHA-256、source commit、署名説明を揃え、英語の`Release notes`へ
同日の5項目を追記した。新しいinstallerを`Executable`／Windowsとして公開し、旧`1.0.0` fileは
削除せず`Hide this file and prevent it from being downloaded`を有効にした。公開ページ
`https://tinmoon-label.itch.io/mio-talk-room`で、`$0 or Donate`、更新履歴、および
`M.I.O_1.0.3_windows-x64_unsigned-preview_setup.exe`だけがdownload対象であることを確認した。

2026-09-07、Ownerの明示承認後に`v1.0.4` installerをuploadして公開した。新fileを
`Executable`／Windowsとして公開し、旧`1.0.0`／`1.0.3` fileは削除せず非表示にした。英語本文と
download／install説明のfile名、version、size、SHA-256、source commit、未署名説明を更新した。
本文欄では追加の更新履歴が保存されなかったため、itch.io標準Devlogとして
`M.I.O. v1.0.4 is now available`を公開した。公開ページで`$0 or Donate`、新installer、Devlogを確認した。

- 公開ページ: `https://tinmoon-label.itch.io/mio-talk-room`
- Devlog: `https://tinmoon-label.itch.io/mio-talk-room/devlog/1655617/mio-v104-is-now-available`

## 6. 公開後確認

- [ ] loginしていないbrowserで両ページを開く
- [ ] 無料で入手でき、支援が必須に見えない
- [ ] downloadまたはStore linkが正しい
- [ ] file名、size、hash、署名説明が一致する
- [ ] 公式ページ、repository、Issue、非公開security報告linkが正しい
- [ ] downloadしたfileを再hashし、公開前の値と一致する
- [ ] 公開日時とURLを`docs/V1-READINESS.md`へ記録する

公開後も、既存のdownload fileを先に削除してから差し替えない。新fileをuploadし、内容とlinkを
確認してから旧fileの扱いを決める。
