# M.I.O. BOOTH商品ページ草案

Status: 2026-09-07に公開ページを`v1.0.4`へ更新済み。

- BOOTH item ID: `8807279`
- 編集画面: https://manage.booth.pm/items/8807279/edit

## 登録欄

- 商品名: `M.I.O. v1.0.4｜複数AIトークルーム（Windows 11）`
- 商品カテゴリ: ソフトウェア／ダウンロード商品
- 価格: `0円`（BOOSTによる支援は任意）
- 年齢区分: 全年齢
- 対応OS: 64-bit版Windows 11
- 制作者・発行: TINMOON
- タグ: `Windows`、`AI開発`、`Codex`、`デスクトップアプリ`
- 代理購入サービス: 許可しない
- 作品file: `M.I.O_1.0.4_windows-x64_unsigned-preview.zip`（downloadable ID `9612396`）

### 公開中v1.0.4の検証値

- 商品名: `M.I.O. v1.0.4｜複数AIトークルーム（Windows 11）`
- 配布ZIP: `M.I.O_1.0.4_windows-x64_unsigned-preview.zip`
- ZIP size: 4,500,699 bytes
- ZIP SHA-256: `06D8F4262DBAE3BF1D43590D6AB1E38232EAF452528BF9163F1D1F32D218F9A1`
- 同梱installer: `M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`
- installer size: 4,516,598 bytes
- installer SHA-256: `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`
- Source commit: `fbc81b89ee64a670f13128487c06a96bfabe3238`
- Authenticode: `NotSigned`（発行者・timestampなし）

BOOTHのBOOSTを利用する場合も、支援は任意であり、M.I.O.の機能、更新、サポートに
差を付けない。無料配布と支援用の有料fileを重複登録しない。

## 商品紹介文（登録済み）

Codex、Claude Code、Gemini、Grokを、ひとつのTalk Roomへ。

M.I.O.（Malevolent Immortal Overdrive）は、複数のコマンドラインAI Providerを、ひとつの
永続的なTalk RoomにまとめるWindows 11向けデスクトップアプリです。ひとりのOwnerが、
メッセージの宛先、各AIに許可するローカルアクセス、重要な開発操作を実行してよいタイミングを
決めます。

M.I.O.はAIモデル、Provider契約、クラウド代理サービスではありません。利用したいAIの公式CLIは、
別途インストールと認証が必要です。Provider CLIがない状態でもM.I.O.本体は起動できます。

### 二つの使い方

Direct modeでは、そのメッセージを受け取るAIを毎回選べます。同じRoomに複数のAIがいても、
すべての発言が自動共有されるわけではありません。

Conductor modeでは、Codexが直接回答するか、最大3人のworkerへ1 roundだけ仕事を振り分けます。
無制限の自律ループや入れ子の委任は行いません。

Providerが処理中の間は送信buttonが停止buttonになります。送信結果が不明な場合は、重複turnを
避けるため勝手に再送しません。

### できること

- 複数のTalk Roomと端末内の会話履歴
- メッセージごとの宛先選択
- 参加AIの表示名、基本設定、icon、対応model選択
- Codexによる生成画像の表示、download、選択した作業folderへの保存
- Roomごとに選んだfolderをCodexへ読取専用または読取・編集で許可
- 固定されたGit status、Node実行、npm build／test／typecheck
- Owner確認付きの、正確なpackage名とversionによるnpm package導入
- 全RoomのJSON backupと、内容確認後の復元
- 日本語／英語UI、配色、壁紙、compact表示の変更

### ProviderごとのV1対応範囲

- Codex：会話、Conductor／worker、確認済みmodel選択、生成画像、選択folderの読取・編集、限定された開発操作
- Claude Code：会話、worker、確認済みmodel選択
- Gemini Antigravity CLI：会話、worker、Provider既定model
- Grok CLI：ローカルCLIを通じた会話と確認済みmodel選択

Claude Code、Gemini、Grokには、M.I.O. V1から一般的なworkspace編集、任意command、Web、MCPを
公開していません。Claude Webと任意のcustom Providerも正式V1接続には含まれません。

### 必要なもの

- 64-bit版Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- 利用したいAI Providerの公式CLIと、そのProviderで利用可能なaccount
- Providerの導入、認証、AIへの送信に必要なinternet接続

AI利用料、通信、保存方針、学習利用、quota、利用規約は各Providerとの契約に従います。M.I.O.の
無料配布やBOOST支援には、各AI Providerの利用料金は含まれません。

### Install前の確認

Microsoft Store版とは別に、BOOTHではWindows x64用NSIS installerを含むZIPを
**未署名Preview**として直接配布します。ZIP内のinstallerにはAuthenticode署名がないため、
Windows Defender SmartScreenの警告が表示される場合があります。Microsoftによる署名を希望する場合は、
無料のMicrosoft Store版を利用してください。

- 配布ZIP: `M.I.O_1.0.4_windows-x64_unsigned-preview.zip`
- ZIP size: 4,500,699 bytes
- ZIP SHA-256: `06D8F4262DBAE3BF1D43590D6AB1E38232EAF452528BF9163F1D1F32D218F9A1`
- 同梱installer: `M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`
- installer size: 4,516,598 bytes
- installer SHA-256: `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`
- Version: `1.0.4`
- Source commit: `fbc81b89ee64a670f13128487c06a96bfabe3238`
- Authenticode: `NotSigned`（発行者・timestampなし）

download後はZIPのfile名とSHA-256が上記と一致することを確認してください。旧版を利用中の場合は、
Room設定から全Room backupを作成し、M.I.O.を終了してからinstallerを実行します。全ユーザー向けの
`C:\Program Files\M.I.O`へ導入するため、Windowsの管理者確認が表示されます。既存Room dataはinstallerを
更新または削除しても自動削除されません。

### V1で対応しないもの

- 公開Remote Relay、複数端末同期、複数account管理
- 自動update、background automation
- token単位のstreaming表示
- 無制限のConductor roundや入れ子の委任
- 任意shell、任意path、無制限network、一般的なdesktop操作

### 更新履歴

#### v1.0.4（2026-09-07）

BOOTH／itch.ioでは9月7日に公開。Microsoft Store版 v1.0.4.0も9月8日に公開済みと確認しました。

- AIの応答中に別のRoomへ移っても処理を継続し、Room一覧で進捗を確認できるようにしました。
- 停止時にProviderの子プロセスも終了し、停止後に安全に再送できるようにしました。
- Codexの実応答に合わせて「初回送信待ち」と「利用可能」を正しく切り替えるようにしました。
- Room切替、フォルダ選択、プロフィール保存、Artwork Editor終了時に操作不能が残る問題を修正しました。
- 遅れて届いた通知が、新しいエラーや別Roomの応答中表示を消さないようにしました。
- M.I.O.終了後に補助プロセスだけが残る問題を修正しました。
- 発言の右クリックメニューで、日本語のコピー操作名が2行に折り返されないようにしました。

#### v1.0.3（2026-09-06）

- AI応答中の進捗表示を追加し、作業状況と経過時間を確認しやすくした
- 送信失敗や停止後の状態表示を見直し、次の操作へ戻りやすくした
- Talk Roomや設定画面を含む右クリック操作をM.I.O.内の専用menuへ統一した
- Windows直接配布版とMicrosoft Store更新候補のversionを`1.0.3`／`1.0.3.0`へ統一した
- installerの初回起動速度と、Windows上の製品名・検索表示を改善した

### Licenseとリンク

- 公開source：`AGPL-3.0-only`
- 公式ページ：https://tinmoon-label-site.pages.dev/works/mio/
- repository：https://github.com/blackcometclub/M.I.O
- 不具合報告：https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml
- 非公開の脆弱性報告：https://github.com/blackcometclub/M.I.O/security/advisories/new

本商品は無料配布を基本とし、BOOSTによる支援は任意です。支援の有無によって機能、update、supportに
差はありません。

スクリーンショットに表示されている会話は、撮影用のデモ内容です。

## 公開時に確認する情報

- [x] 配布候補file名とMicrosoft Store link
- [x] file size
- [x] SHA-256
- [x] 署名状態と発行者
- [x] 公開日: 2026-09-07
- [x] update方法

## 使用する画像

1. 商品cover: `docs/assets/storefronts/mio-booth-cover-1200x1200.png`
2. `docs/assets/screenshots/mio-talk-room.png`
3. `docs/assets/screenshots/mio-room-settings.png`
4. `docs/assets/screenshots/mio-preferences.png`
5. `docs/assets/screenshots/mio-appearance.png`
6. `docs/assets/screenshots/mio-compact-image-free.png`

画像内の会話は撮影用デモであることを商品説明末尾に注記する。
