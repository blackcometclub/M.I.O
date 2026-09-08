# M.I.O. V1 Release notes — 現行source v1.0.4

**状態: 公開済みです。** Microsoft Storeでは署名済み`1.0.3.0`を公開中です。BOOTH／itch.ioでは、
別配布の未署名`v1.0.4`直接download Previewを公開しています。Microsoft Storeの`1.0.4.0`更新packageは
Submission 4として認定中で、審査中も公開済み`1.0.3.0`は継続して取得できます。

## 主な機能

- 1人のOwnerと複数AI participantが使う永続Talk Room
- 宛先を明示するDirect modeと、1 round・最大3 workerのCodex Conductor mode
- Codexの会話のみ、選択folder読取、選択folder読取・編集profile
- 固定Git status、Node、npm build／test／typecheck、Owner確認付きexact npm package導入
- CodexとClaude Codeの確認済みmodel選択。GeminiはProvider既定
- 結果不明を自動再送しないactive turn停止
- Codex生成画像の表示、download、編集folder保存、アスペクト比・品質の希望設定
- 利用者が選ぶRoom backup先と、内容確認後の復元

## 公開済みv1.0.3直接配布更新

- 確認済み失敗と配達結果不明を区別し、重複する可能性のあるturnを黙って再送しない
- AIごとの進捗を表示し、失敗・停止後にRoomを確実に再操作可能な状態へ戻す
- Codex再接続案内を改善し、利用不能なCLI候補を選びにくくする
- M.I.O.用右クリックmenuとmessage copyを追加し、不要なbrowser menuを抑止

## 公開済みv1.0.4直接配布更新

- AIごとの進捗を表示し、別Roomを見ている間も開始済みturnを継続
- 停止時にProviderのprocess treeを終了し、Roomを確実に再操作可能な状態へ戻す
- Codex実行環境の検出を改善し、実際のreply成功後だけ「利用可能」と表示
- native folder選択結果を受け損ねてもappが操作不能のまま残らないよう修正
- Room切替直後の設定混在、遅延通知による状態上書き、プロフィール保存失敗後の操作lockを修正
- main画面終了後に補助processだけが残る常駐を防止
- 日本語のmessage copy操作名を右クリックmenu内で1行表示

## 対応環境

- 64-bit版Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- Program Filesへ全ユーザー用としてinstallし、Windows UACで管理者の承認が必要

Windows 10とWindows以外のplatformはV1で保証しません。

## Provider範囲

| Provider | V1機能 |
|---|---|
| Codex | 会話、Conductor／worker、確認済みmodel選択、画像生成、選択folder読取・編集、固定command |
| Claude Code | 会話／workerと確認済みmodel選択。workspace・toolなし |
| Gemini Antigravity | Provider既定modelの会話／worker。workspace・toolなし |
| Grok | 会話と、未追跡file本文やhost pathを含まない追跡済みGitの限定read-only review |

Claude Web、公開Remote Relay、generic／custom Provider、複数device、複数accountはV1接続ではありません。

## 安全境界

- Providerへの配達結果不明を自動再送しない
- workspace accessをM.I.O. broker経由で選択folderへ限定
- 任意shell文字列、任意path、削除／名前変更、無制限network accessを公開しない
- 重要な固定操作はscopeが一致する実行時Owner確認を要求
- Provider credentialはM.I.O.外のProvider CLIで入力

## Installとupdate

versionを変更する前にRoomをbackupし、M.I.O.を終了します。Microsoft Store版はMicrosoft Storeから
更新します。直接配布版は、未署名installerの公開SHA-256を照合してから実行します。現在利用者用として
入れた旧alphaがある場合は、保持dataを削除せずにappだけをuninstallしてからV1を導入します。
以後の直接配布V1は、Release notesに別の指定がない限り既存の全ユーザー用直接installへ上書きできます。

詳しい手順とdata保持は[V1利用ガイド](USER-GUIDE.ja.md)を参照してください。

## 配布経路

- [Microsoft Store](https://apps.microsoft.com/detail/9NS9B7T71XHN)：推奨する署名済みpackage。
  現在の公開版は`1.0.3.0`
- [BOOTH](https://tinmoon.booth.pm/items/8807279)：無料・支援任意。未署名`v1.0.4` Preview installerと
  checksum案内を含むZIP
- [itch.io](https://tinmoon-label.itch.io/mio-talk-room)：無料／支援任意の未署名`v1.0.4` Preview installer。
  [英語の更新履歴](https://tinmoon-label.itch.io/mio-talk-room/devlog/1655617/mio-v104-is-now-available)を公開済み
- [GitHub repository](https://github.com/blackcometclub/M.I.O)：sourceと開発履歴。旧alpha Releaseは変更しない
  履歴artifactで、現在のV1 installer配布経路ではない

## 既知の制約

- 自動update、公開Remote Relay、複数device同期、複数account管理なし
- Claude Code、Gemini、Grokのworkspace toolなし。ただし別gateを通した限定Git reviewを除く
- token単位のstreaming表示なし
- 生成画像の正確なpixel数保証なし。取込artifactは16 MiBまで
- 別配布の未署名BOOTH／itch.io PreviewではSmartScreenが表示される場合がある。署名済みpackageが
  必要な場合はMicrosoft Store版を使用する

## 現在の更新状況

- Microsoft Store `1.0.4.0`はSubmission 4として認定中。公開や認定取消は、引き続き個別承認が必要なStore操作として扱う
- BOOTH／itch.ioで未署名`v1.0.4`直接download Previewを2026-09-07に公開済み
- 直接配布installerにはAuthenticode署名がない。署名済みpackageが必要な利用者はMicrosoft Store版を使用する
