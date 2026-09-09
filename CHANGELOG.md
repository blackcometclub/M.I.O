# Changelog

M.I.O. (Malevolent Immortal Overdrive) の主な変更を記録します。

## Unreleased

## v1.0.5 — 2026-09-08

Status: manifest versionは`1.0.5`、Store版は`1.0.5.0`。Microsoft Store、TINMOON公式サイト、
BOOTH、itch.ioで2026-09-09に公開済み。GitHubではsource-onlyの通常Releaseとして配布し、
未署名installerは添付しない。

### Fixed

- 再起動後、最後に開いていたRoomを表示する。一覧の読込失敗時は保存した選択を上書きせず、
  削除済み・不正なRoom IDなら存在する先頭Roomへ戻る
- バックアップ復元後も表示中のRoomが存在すれば選択を維持し、存在しなければ復元先の先頭Roomへ戻る

## v1.0.4 — 2026-09-07

Status: manifest versionは`1.0.4`。Microsoft Storeの`1.0.4.0`はSubmission 4として公開済み
（2026-09-08確認）。同じ実装の未署名直接配布版`v1.0.4`はBOOTH／itch.ioで2026-09-07に公開済み。

### Fixed

- AI応答中に別Roomへ移動してもturnを継続し、Room一覧へ進捗ringと応答中表示を出す
- Provider停止時に子processも含むprocess treeを終了し、停止後の再送を可能にする
- Codexの実応答前後を「初回送信待ち」／「利用可能」と区別し、返答成功直後に状態を更新する
- native folder選択結果を受け損ねても、app全体の操作lockが残らないようにする
- backup復元の成否にかかわらず、表示中Roomのcommand sessionを安全に再開する
- Artwork Editorが予期せず閉じた場合も待機状態を解除し、破棄失敗時には操作をやり直せるようにする
- Room切替直後はworkspace／Conductor設定の再読込完了まで送信と設定操作を待ち、前のRoomの状態を表示しない
- 検出済みCLIの「初回送信待ち」を未接続扱いせず、実際に返信できないAIだけを保存のみと案内する
- 遅れて届いたAI通知が、別Roomまたは同じRoomの新しいエラーや応答中表示を消さないようにする
- 参加者プロフィールの保存処理が想定外に失敗しても、保存中のまま操作不能にならないようにする
- main画面を閉じた後に補助処理だけが残っても、無画面のM.I.O.プロセスを常駐させない
- Store用MSIXの4桁versionを、通常版manifestの3桁versionから自動的に揃える
- 発言の右クリックメニューで、日本語のコピー操作名の末尾だけが次の行へ折り返されないようにする

## v1.0.3 — 2026-09-06

Status: BOOTH／itch.ioの直接配布版を公開済みです。Microsoft Storeの`1.0.3.0`更新候補は別の提出として扱います。

### Fixed

- Codexへ送信した結果が不明な場合に、再送で重複させず、再接続や更新へ進める案内を追加
- Codex、Claude Code、Gemini、Grokの処理状況をAIごとに表示し、停止・異常終了後に操作可能な状態へ確実に戻す
- 利用できないCLI候補を選びにくくし、Codex再接続時の状態表示と復旧導線を改善
- message、設定画面、Artwork Editorの右クリックをM.I.O.向けに整理し、messageにはコピー操作を追加

## v1.0.0 — 2026-09-04

Status: 正式V1の基礎となった履歴releaseです。その後、Microsoft Store `1.0.2.0`と
BOOTH／itch.io直接配布`v1.0.3`が一般公開されました。

### Added

- Room単位で選択したworkspaceをCodexが読取り・編集できる、host仲介のWorkspace Broker
- Codexの固定Git status、Node／npm build・test・typecheck、確認付きのexact npm package導入
- Codex、Claude Code、Grokの検証済み候補から選ぶProvider別model設定と、Provider既定を維持する選択肢
- Codex生成画像のRoom表示、原寸表示、download、選択済み編集フォルダへの非上書き保存
- 画像生成時の1:1、4:3、3:4、16:9、9:16のアスペクト比と品質希望

### Changed

- AI応答待ちの送信ボタンを停止ボタンへ切り替え、結果不明時は自動再送しない
- Conductorがworkerの表示名と選択modelを引き継ぎ、Direct／Conductorともmodel変更後に古いcontinuityへ戻らない
- Windows installerの製品名、ファイル名、shortcut名を`M.I.O`へ統一する

### Safety

- workspace pathをProviderへ渡さず、絶対path、reparse point、複数hard link、workspace外操作をbrokerで拒否する
- 固定commandをAppContainer、network制限、bounded I/O、timeout、Job Object、temporary resource cleanup付きで実行する
- Owner確認が必要な操作はRoom、session、workspace、action、targetの完全一致scopeで一度だけ許可する
- Grokのworkspace利用を、未追跡file本文とhost pathを含まない追跡済みGit差分の読取り専用reviewへ限定する

### Release result

- Microsoft Storeの署名済み`1.0.2.0`を公開
- BOOTH／itch.ioで未署名Preview `v1.0.3`を公開
- Microsoft Store `1.0.3.0`更新を別Submissionとして認定へ提出
- 公開後の実機確認と配布記録は`docs/MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md`および
  `docs/storefronts/BOOTH-ITCHIO-PUBLISHING-CHECKLIST.md`へ記録

## v0.1.0-alpha.2 — 2026-08-28

Status: 履歴分離済みrepositoryからGitHub Prereleaseとして公開。安定版ではありません。

### Fixed

- Rustupの標準導入先に`cargo.exe`があり、起動中のPowerShellの`PATH`へまだ反映されていない場合も、Windows alpha EXEをbuildできるようにした

### Changed

- Windows alpha buildとpublic source exportのscriptを版番号非依存にし、committed manifestのversionを使用する
- READMEを英語の標準版と日本語版に分け、相互に切り替えられるようにした
- 公開済みのalpha.1 Release、tag、assetを変更せず、alpha.2を別Releaseとして公開する

## v0.1.0-alpha.1 — Public alpha release

Status: 2026-08-27に履歴分離済みrepositoryからGitHub Prereleaseを再公開。安定版ではありません。

最初のsource-first公開α版です。複数のローカルAIをひとつのTalk Roomへ集め、直接会話または指揮者モードで協働させるWindows向け研究用α版として区切ります。

### Added

- Tauri 2 / React / TypeScriptによるWindows Talk Room UI
- Room作成、名前変更、参加AI管理、永続message履歴
- 参加者の表示名、avatar、AI向けローカル案内、端末内設定
- 全RoomのJSON backupと、二段階確認付きrestore
- Codex、Gemini Antigravity、Claude Fable、GrokのローカルCLI会話adapter
- 明示した宛先へ送るDirect mode
- Codexが1 round・最大3 workerまでを扱うConductor mode
- Codexのchat-only接続と、workspace read / write requestの送信前fail-closed
- token設定時だけloopbackへ起動するlocal MCP read toolsとOwner-proxy write
- M.I.O.ロゴ、外観設定、参加AI一覧の折りたたみUI

### Safety boundaries

- recipient単位のdurable dispatch ledgerとidempotency
- 結果不明のProvider turnを自動再送しないunknown outcome処理
- 未接続AIの偽replyや成功表示を生成しない
- single-instanceによるRoom writerの重複防止
- Windows alpha.1でworkspace read / write UIを無効化し、保存済みrequestもProvider起動前に拒否
- local MCPのloopback限定、token認証、bounded request

### Experimental or unavailable

- Claude Webの正式製品接続
- 公開Remote Relay、複数device、複数account
- ChatGPT Web、OpenAI API、Generic MCP client、Custom adapter
- Google Search / AI Mode Browser Bridgeの正式製品化
- Codex、Fable、Gemini、Grokのworkspace access
- token streaming UI、Provider turn途中のcancel、model選択UI
- background automation、無制限のConductor round、nested delegation
- Windows以外の対応保証

### Distribution

- source-firstの研究用α版
- 配布用installer、コード署名、自動更新、安定版SLAは未提供
- 公開判断と検証証拠は `docs/PUBLIC-ALPHA1-READINESS.md` に記録
