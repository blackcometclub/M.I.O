# 次期バージョンへの申し送り

## 最後に選択したRoomを再起動時に復元する

- 決定日: 2026-09-08。Ownerの「メモしておいて次期バージョンに取り入れましょう」により、次期バージョンで対応する。公開済み1.0.4のtag・artifactは変更しない。
- 現象: Store版1.0.4.0で末尾の`New room 5`を開いて通常終了しても、再起動直後は先頭の`M.I.O.開発室`が開く。`New room 5`を選び直すと保存済みの会話は表示される。会話消失ではない。
- 実装の手掛かり: `apps/desktop/src/hooks/useRooms.ts`は先頭Roomでactive Roomを初期化している。最後の選択Roomの保存・復元を追加する。
- 受入条件: 先頭以外のRoomを選んで通常終了・再起動したとき、同じRoomと会話が表示される。保存されたRoom IDが削除済み・不正・未保存の場合は、存在するRoomへ安全に戻る。初回起動、Room削除、backup復元後も確認する。
- 試験証拠: `docs/MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md`の「2026-09-08 15:08–15:11 JSTのStore版Codex Direct試験」。会話の永続化PASSと、選択Roomの自動復帰未達を分けて記録している。

### 2026-09-08 実装と検証

- Ownerの「続けてください」により次期版向けsourceへ実装した。`CHANGELOG.md`のUnreleasedに記録し、
  版番号、公開済み1.0.4のtag／installer／Store packageは変更しない。
- 選択Room IDを既存の画面設定と同じlocalStorageへ保存する。実アプリ用とbrowser demo用のkeyを分離する。
  会話本体やbackup形式は変更しない。WebView側の保存領域を削除した場合は先頭Roomへ戻る。
- `loading`／`error`中の仮表示では保存値を上書きせず、読込済みのRoom一覧からIDを選ぶ。
  復帰するRoomの参加者に合わせて宛先も初期化し、Conductor／workspaceは既存処理で再取得する。
- Room削除後はfallbackを保存し、backup復元後は元のRoomが残れば選択を維持する。localStorageの
  読取・書込に失敗しても、Room切替や会話を妨げない。
- 回帰確認: `scripts/test-room-selection.mjs`で実際の`useRooms`をReact StrictMode下にmountする。
  専用headless Edgeで、遅延した一覧読込、reload、初回起動、削除済み／不正／空／長過ぎるID、
  storageの読取・書込失敗、一覧読込失敗、demoとの分離、Room削除、新規Room、backup復元2経路を確認する。
  desktop bridgeはfixtureへ置換し、実データ・Provider・認証には接続しない。
- 実行方法: Playwrightが利用できる環境で`node scripts/test-room-selection.mjs`。
  bundled runtimeを使う場合は`MIO_PLAYWRIGHT_MODULE`にPlaywrightの`index.mjs`の絶対pathを指定する。
  2026-09-08 16:17 JSTまでに14ケースがPASSし、`npm run typecheck`、`npm run build`、`git diff --check`も成功した。
  試験harness初回はHTML内moduleの解決で待機timeoutしたため、ViteのHTML変換を通すよう修正して再実行した。
  続いて下記の確認用native appで実機再起動を確認した。

### 2026-09-08 16:30–16:35 JST 実機での通常終了・再起動

- 対象source: `26064693a7f8c126bcd60903b09411e5ebc6d2e4`。Windows x64のrelease buildに
  `app.moe.roomrestorecheck`という専用identifierと`M.I.O. Room Restore Check`というwindow titleを指定した。
  Roaming側のRoomデータとLocal側のWebView保存領域は試験前に存在せず、通常版／Store版から分離した。
- `tauri build --no-bundle --target x86_64-pc-windows-msvc --config <専用config> -- --locked --offline`が成功。
  static CRTでbuildしたexe、command helper、LICENSE、THIRD-PARTY-NOTICESを確認用ZIPにまとめた。
  versionはsourceの`1.0.4`を継承した検証専用品であり、新しい公開版やStore更新packageではない。

| 実画面で確認した経路 | 結果 |
| --- | --- |
| 初回起動 | 先頭の`M.I.O.開発室`と`Core + Room ready`を表示 |
| 既存Roomの復帰 | `回答くらべ部屋`を選んで通常終了。PID `31248`が終了した後、PID `40672`で再起動し、Roomをクリックする前から同じRoom、初期サンプル会話、宛先ChatGPTを表示 |
| 新規Roomの復帰 | `新しいルーム 4`を作成して通常終了。PID `40672`が終了した後、PID `24324`で再起動し、同じ新規Room、空の会話、宛先Codexを表示 |
| データ保全と終了 | 新規Roomのsnapshotは再起動前後でhash一致。16:35 JSTに確認用exeのprocessは0件。Store版はPID `40212`のまま継続 |

- 新規Room ID: `room-7e4b106b-cade-41e8-b256-65aaed8ab037`。
  QA snapshot SHA-256: `9C46591AEB1E66AFE1B824D7866C6D9D7722A9501D92884E64EC52CE8DB5EA8F`。
  初期サンプルRoomを選ぶだけではsnapshot fileは未作成で、新規Room作成後に保存された。
- Store packageの実行環境で読み取った会話・参加者設定のhashは試験前後で一致し、workspace設定fileは両方で不在。
  通常版の会話snapshotも既存の`999EB1B0F560E60080DEA7A4317C83D7FD2FE3C02F672FE2F453E308068F2B0A`のままだった。
- 実画面はこの作業のComputer Use記録にある。確認用window IDは初回`19861640`、2回目`75170590`、
  3回目`12061276`。ウィンドウ移動・他画面の重なりがあるcaptureは再取得し、最終画面でRoom名と宛先を確認した。
  各操作の区切りでComputer Use sessionを解除した。
- ローカル証拠: `.tools/room-restore-native/2606469-20260908/`の`tauri.qa.conf.json`、
  `package-manifest.json`、`launch-*.json`、`closed-*.json`、`qa-before-restart.json`／`qa-after-restart.json`、
  `host-before.json`、`store-before.json`／`store-after.json`、`comparison.json`。
  ZIPは`MIO-room-restore-check-2606469.zip`、SHA-256は
  `20F3757AB28FB1CAC380C269B95A7743C6BD90904AFDC358D517A857794D3970`。
- 制限: native WebViewでの選択Room復帰を確認した。初期サンプル表示を新しいAI返信の証拠にしない。
  AI設定・認証・送信やSandbox再試験は行っていない。削除済みIDなどの異常系は上記14ケースの回帰確認による。
  正式な次期版の番号決定、installer／MSIX作成、Store更新としての試験と公開は別作業。

公開済み1.0.4のclean Windows／WebView2依存関係の試験は2026-09-08に完了した。結果は公開後チェックリストを参照。
公開後確認全体の完了・省略・継続確認は[1.0.4の整理](RELEASE-1.0.4-CLOSEOUT.md)にまとめた。

その後、次期版の番号を`1.0.5`として、直接配布とStore提出のpackageを作成・公開した。
通常版とStore署名版の実機更新・Room復帰もPASSした。[1.0.5の公開・確認結果](RELEASE-1.0.5-PREPARATION.md)を参照。
現在残っている作業と追加機能候補は、固定の[現在地カード](CURRENT-STATUS-CARD.md)だけを更新する。
