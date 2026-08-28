# M.I.O. Desktop

**M.I.O. (Malevolent Immortal Overdrive)** v0.1.0-alpha.2開発中のWindows desktop実装です。最新の公開版はv0.1.0-alpha.1です。内部のcrate、package、command、environment variableには互換性維持のため`moe` / `MOE_`識別子が残ります。

## Current Room boundary

Desktop Roomは3つの標準RoomをRust catalogから読み、Tauri版のRoom切替、全Roomの送信、新規Room作成、参加AI追加、名前変更、未参照AIの取り外し、ユーザー作成Roomの削除を同じversioned JSON snapshotへ保存します。標準3室とownerは削除せず、最低1人のAIとmessage履歴の参照整合性を守ります。primary破損時は壊れたfileを退避して直前の正常backupから復旧し、Room設定からDocumentsへ全Room backupを作成して最新1件を二段階確認で復元できます。既存のsingle-room fileは履歴を残したまま不足catalogだけを補完します。利用不能なProviderにはデモ返信を作らず、ユーザーメッセージの保存完了と接続状態を表示します。ブラウザpreviewだけは外部作用のないローカルデモを維持します。

Tauri 2、React、TypeScriptで構成するM.I.O.のDesktopアプリです。

## Current alpha product path

- Codex、Gemini Antigravity、Claude Fable、Grokの利用可能なローカルCLIへ会話を配送します。
- Direct modeではOwnerが宛先を明示し、Conductor modeではCodexが1 round・最大3 workerまでを分担します。
- Codexだけが、Roomごとに明示選択されたworkspaceのchat-only / read / writeに対応します。
- ChatGPT Web、OpenAI API、Generic MCP client、Custom adapterは現在未対応です。
- Claude Web、Remote Relay、Google Search / AI Mode Browser Bridgeは完成機能ではありません。
- Provider結果が不明なturnを自動再送せず、偽のreplyや成功表示へ置き換えません。

## UI source layout

```text
src/
  App.tsx                 画面構成とfeatureの接続だけを担当
  components/             propsで描画するUI部品
  hooks/
    useBootstrapStatus    Tauri / browser previewの起動状態
    useRooms              Rust Room読取・browser preview・ローカル操作状態
    useAppearance         背景色・背景画像・飾り絵調整
  styles.css              下記CSS moduleの読込順だけを定義
  styles/
    base                  resetと共通基礎
    shell                 window、sidebar、workspace、header
    popovers              外観設定と参加AI menuの共通popover枠
    appearance            外観設定固有のcontrol
    room-settings         Room名・参加AI・削除の管理popover
    participants          参加AI、menu、avatar
    conversation          飾り絵、会話、message
    composer              宛先と入力欄
    responsive            responsiveとreduced motion
```

`App.tsx`へfeature固有のstateや副作用を戻さず、表示部品は`components/`、画面単位の状態と操作は`hooks/`へ置きます。CSSのcascadeを保つため、`styles.css`のimport順を変更するときは意図を確認してください。

```powershell
npm.cmd run dev
npm.cmd run typecheck
npm.cmd run build
npm.cmd run tauri:build
```

Reactは表示とユーザー操作だけを担当します。filesystem、外部process、秘密情報、Agent接続はRust backendを経由し、WebViewへ任意shell権限を公開しません。

2026-08-12の隔離PoCでは、Windows Credential ManagerのGeneric Credentialでdevice credentialの保存、別process復元、更新、削除がPASSしました。製品へ昇格する場合もsecret値をWebViewへ返さず、Rust backendが接続時に直接使用する境界を維持します。PoC targetの命名やAPIをそのまま製品仕様にはしません。

製品候補の `moe-credential-store` crateと、metadata-onlyの `relay_credential_status(accountId)` Tauri commandまで追加済みです。WebViewは保存有無だけを受け取り、credentialの保存・読取・削除はcommand surfaceへ公開しません。

Relay backendにはtransport非依存のlifecycle状態機械と、actionを実background taskへ写像する内部executorがあります。executorはaccountごとの二重taskを拒否し、retry timerのcancel、古いgenerationのevent破棄、shutdown時のcancel / joinを担当します。start / stop commandはWebViewへ公開していません。

`DesktopRelayOrchestrator`の内部event pumpがexecutor eventを状態機械へ戻し、UIやstatus pollに依存せずunexpected disconnectからtimerと次の接続taskを自動生成します。`relay_connection_status(accountId)`はruntime稼働中に`retryWaiting` / `stopping`、retry回数、次の待ち時間をmetadataだけで返します。localhost integrationではWindows credential loadから1秒後の再接続、socket cancel、offline復帰までPASSしています。

production HTTPS transportはWindows Credential ManagerからのloadとDesktop taskへ結合済みです。非secretの `MOE_RELAY_ENDPOINT`、`MOE_RELAY_ACCOUNT_ID`、`MOE_RELAY_DEVICE_ID`をbuild時に3項目とも設定した配布物だけが起動時に自動接続します。未設定buildはnetworkへ接続せず、不完全設定やHTTP endpointは起動前に拒否します。credential値や任意endpointを受け取るTauri commandはありません。

確立後のHTTPS linkはstrictな `moe_read_room` requestをRust backendのRoom sourceへrouteし、同じrequest IDを持つresponseを返します。未知method、不正params、重複requestは固定errorへ変換し、stop後のresponseは抑止します。現在のRoom sourceは `RwLock` storeとversioned JSON persistenceを持ち、Tauri writeとAI responseをRelay readからも観測できます。

Tauri画面の `useRooms` は起動時にboundedな `desktop_room_list` と `desktop_room_read` commandを呼び、全Roomの参加者・messageをRust sourceから表示します。初回だけbundled catalogを使い、以後はTauri `app_data_dir()` の `room-snapshot-v1.json` から復元します。作成、参加AI追加、名前変更、参加AI取り外し、ユーザー作成Room削除もRust commandへ通し、成功responseの相関を検査してから画面へ反映します。browser previewは従来のローカルデモへfallbackします。Tauri成功時はfooterが `Core + Room ready`、失敗時は `Room offline` になるため、デモfallbackでbackend障害を隠しません。

全Roomのユーザーmessage送信は `desktop_room_write_message` でRust Roomへ保存します。client-generated message IDをidempotency keyとして、同じ内容の再試行は二重追加せず、内容が違うID再利用は拒否します。成功後だけRust responseを画面へ追加し、失敗時は本文を入力欄に残して直下へ再試行案内を表示します。mutation後のsnapshotは64 MiB上限のversioned JSONへtemp + 1世代backupで保存し、file失敗時はmemoryも書込み前へ戻します。Tauriでは接続前ダミーAI返答を出しません。browser previewだけはローカルデモ送信を維持します。

対応AI宛messageは保存成功後に `desktop_room_dispatch_message` で各製品adapterへ渡します。Codex driverはglobal npm版、`MOE_CODEX_BIN`、`MOE_CODEX_CLI_JS`の明示launcherを使い、Roomのaccess modeに応じたpermission profileとworkspace境界を動的設定します。Gemini Antigravity、Claude Fable、Grokは会話専用で、filesystemやtool accessを許可しません。1つのsource message / recipientにつき外部turnは一度だけ開始し、結果不明後は自動再送しません。各AIの最終応答だけを同じRust Roomへ保存し、未接続・未対応Providerをダミー応答へfallbackしません。
