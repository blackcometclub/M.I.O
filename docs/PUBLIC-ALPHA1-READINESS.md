# M.I.O. v0.1.0-alpha.1 public readiness checklist

- Status: Draft execution checklist
- Release decision: ADR 0037
- Target: Windows source-first research alpha
- Rule: unchecked means **not verified**

この文書は公開完了の宣言ではない。各項目へ検証日、対象commit、環境、結果、必要な
証拠を記録し、すべての必須項目が完了してから公開可否を判断する。

## 0. Release candidate identity

- [x] 公開候補commitを固定した。
- [x] versionが`0.1.0-alpha.1`として一貫している。
- [x] ユーザー向け製品名が`M.I.O.`で統一されている。
- [x] 正式展開名が`Malevolent Immortal Overdrive`のまま表示され、日本語へ翻訳されていない。
- [x] 内部の`moe` identifierを互換性のため残す場所を確認した。
- [x] 公開用READMEの説明と実装済み機能が一致している。

Evidence:

- Date: 2026-08-21
- Commit: `8507e2d` + 未commitのGate 0整合候補
- Notes:
  - Root npm package、`package-lock.json`、desktop npm package、Tauri config、Rust workspaceを
    `0.1.0-alpha.1`へ統一した。内部PoC / fixtureのprivate packageは`0.0.0`を維持する。
  - `npm.cmd ci --ignore-scripts --dry-run --offline`が成功し、packageとlockの整合を確認した。
  - 現行README、desktop UI、package docs、architecture docsのユーザー向け名を`M.I.O.`へ統一した。
    ADR 0037以前の履歴ADRに旧名が残ることと、現在名が`M.I.O.`であることをREADMEへ明記した。
  - UI、logo、READMEの正式展開名は英語の`Malevolent Immortal Overdrive`だけで、日本語訳を
    product stringへ追加していない。
  - READMEは互換性のため残すcrate、package、environment variable、app dataの`moe`識別子を
    説明する。公開snapshotから除外する旧個人用計画書への参照を削除し、link切れを解消した。
  - READMEの完成機能、未接続、実験中、現在未対応、source-first制約をADR 0037と照合した。

Release candidate freeze:

- Date: 2026-08-27
- Commit: `35479df0871003e37834099955095bda5e98a3e3`
- CI URL: https://github.com/blackcometclub/M.I.O/actions/runs/33033278649
- Notes:
  - `origin/main`と一致するcommitから履歴なし公開snapshot 310ファイルを再生成し、Git treeの
    期待file listと展開済みsourceの実file listが完全一致した。
  - Source ZIP SHA-256は
    `D4F8E0267626C4A1A3A3FFCB1A30D313CD99A20FDA0EB04258431B6CF9D1568D`だった。
  - Gitleaks v8.30.1は展開済みsource約1.89 MBを`--redact=100`でscanし、0 findingsだった。
    credential入りURL、private key、実local desktop path、protectedなHANDOFF fileも各0件だった。
  - GitHub Actions Windows CIは9分53秒で成功し、dependency導入、typecheck、frontend build、
    evidence sanitization、Rust format、Windows dependency boundary、Rust workspace testを含む
    全stepがPASSした。
  - このcommitをalpha.1のsource-first release candidateとして固定する。以後のreadiness記録は
    candidate内容を変更しないrelease運用文書の更新として扱う。

## 1. Gate 1: incomplete controls fail safely

次のcontrolを実画面で一つずつ操作し、成功、説明付きdisabled、または明示errorの
いずれかになることを確認する。無反応、永久loading、app crash、data破損を許可しない。

- [x] Window close、minimize、maximize、resize。
- [x] Room選択、作成、rename、削除、保護Roomの削除拒否。
- [x] 参加AI一覧の展開／折りたたみ、宛先選択、AI追加。
- [x] 未接続・未対応AIの選択または追加時の説明。
- [x] Message送信、空message拒否、送信失敗後のdraft保持。
- [x] Direct／Conductor切替、conductor未設定時の安全なfallback。
- [x] Conductor設定、非対応AIの選択拒否、workerのpartial / unknown結果表示。
- [x] 参加者profileの表示名、avatar、基本設定、access mode。
- [x] 非Codex AIでunsupported workspace modeがdisabledであること。
- [x] Workspace選択、missing folder、junction / symlink拒否。
- [x] Room backup、restore、二段階確認、破損backup拒否。
- [x] 外観、言語、font、sidebar幅、文字サイズ、飾り絵調整。
- [x] MCP未設定、不正token、M.I.O.停止中のbounded error。

Evidence:

- Date: 2026-08-17
- Commit: `1c539a1`
- Environment: Windows 11 host / `target/debug/moe-desktop.exe` / existing local Room
- Notes / screenshots:
  - PASS（部分）: 参加AI一覧の展開／折りたたみ、`AIを追加`menuの開閉、Direct／Conductor切替。
  - PASS（部分）: ルーム設定、外観設定、環境設定がchatより前面に表示され、Escapeまたは外側clickで閉じた。
  - 環境設定では日本語、system font、chat文字サイズ100%、端末への自動保存説明を確認した。値は変更していない。
  - PASS（部分）: 空messageでは送信buttonがdisabled、provider unknown警告は明示表示された。
  - PASS（部分）: Window最小化と再表示、最大化と元サイズへの復元。close、手動resizeは未検証。
  - Room作成／削除／restore、provider送信、profile編集、workspace選択などstateを変更する項目はこの観察では未検証。

Retest evidence:

- Date: 2026-08-21
- Commit: `ead67b0`
- Environment: Windows Sandbox / WebView2 Runtime導入済み / AI CLIなし / network無効
- Notes / screenshots:
  - PASS: app内close、最小化と再表示、最大化と復元、window右下のdragによる手動resizeを
    一巡し、crash、永久loading、表示破損が起きなかった。
  - PASS: 参加AI一覧を展開／折りたたみでき、CodexとClaude Fableの宛先選択／解除、
    Claude Fableの追加を操作できた。宛先を全解除するとcomposerに選択案内が表示され、
    送信buttonはdisabledになった。
  - PASS: `AIを追加`ではChatGPT、Gemini、OpenAI APIなどの未対応項目がdisabledで
    `現在未対応`と表示され、追加済みClaude Fableは`CLI設定が必要`と表示された。
  - PASS: Claude Fable profileではchat-onlyだけが選択可能で、workspace read / writeは
    `このAI未対応`としてdisabledだった。Codex profileでは3つのaccess modeと、選択folder内に
    commandを制限しweb / networkを許可しない説明を確認した。
  - PASS: 新規Roomを作成して`g1`へrenameし、Room切替も行えた。削除は一度目で
    `本当に削除`へ切り替わり、確定後は`g1`が一覧から消えて標準Roomへ安全に戻った。
  - PASS: Direct／Conductorを切り替え、Codexを指揮者に設定できた。Conductor選択中に
    指揮者を`指揮者なし`へ戻すと、指揮者badgeとmode切替が消え、通常のDirect composerへ
    安全に戻った。
  - PASS: 非対応のClaude Fableは指揮者候補に現れなかった。この時点では未検証だったworker
    partial / unknown結果表示は、2026-08-23のcontrolled live retestで別途確認した。
  - PASS: Codex profileのaccess modeを`会話のみ`へ変更して保存し、再編集時にも選択が保持
    されることを確認した。その後`選択フォルダーを読取り・編集`へ戻して保存し、workspace
    未選択のparticipant cardが`設定が必要`表示へ戻ることを確認した。表示名と基本設定を
    Sandbox用の`qa`へ変更して保存し、再編集時にも両方が保持されることを確認後、表示名
    `Codex`、基本設定空欄へ戻した。Windows標準画像をavatarへ設定して保存・再編集時の保持を
    確認し、`画像を外す`でinitials `Co`へ戻して保存した。
  - OBSERVED: custom avatarなしでCodex profileを保存すると、provider既定のinitials `CX`が
    表示名由来の`Co`へ変わった。profile削除／既定値復元controlは現行UIにないため、公開前に
    意図した挙動かを判断する。
  - PASS（部分）: SandboxのDocumentsに専用folder `MI-QA`を作成してCodex workspaceへ選択し、
    `MI-QA 内の読み取り・編集を許可中です`と表示されることを確認した。その後`会話のみに戻す`
    でfolder accessを解除し、未選択状態へ戻した。missing folder、junction / symlink拒否は
    未検証なのでworkspace項目は未完了のままにした。
  - PASS: yellow / pink theme、日本語／English、system font / Arial、chat文字サイズ、
    sidebar表示サイズを変更して元へ戻した。Windows標準画像を背景へ設定して実画面への
    反映を確認し、別の標準画像を飾り絵へ設定して配置画面で倍率を108%へ変更・保存した。
    背景と飾り絵を両方とも外し、元のyellow themeへ戻ることも確認した。
  - PASS（部分）: AI CLIなしのCodexへDirect送信するとuser messageだけが履歴へ保存され、
    fake replyは生成されなかった。結果不明を二重送信防止のため自動再送しない警告が
    明示され、app操作を継続できた。送信失敗後のdraft保持は未検証なのでmessage項目は
    未完了のままにした。
  - PASS（部分）: 全4室のbackupを作成し、Documentsの互換保存先`M.O.E Backups`に
    約3 KBの`moe-room-backup-<20 digits>.json`が生成された。設定panelの最下部には
    Room数とfile名を含む成功表示が現れた。復元buttonは一度目で`本当に復元`へ変わり、
    確定後はbackupから`g1`を含む4室が復元され、file名と`4室を復元しました`が表示された。
    復元された`g1`を選択すると参加AI、既存message、composerが正常に読み込まれ、crashや
    永久loadingはなかった。破損backup拒否は未検証なのでbackup項目は未完了のままにした。
  - PASS: 標準の`M.I.O.開発室`では削除buttonが表示されず、設定panel最下部に
    `標準ルームは削除から保護されています`と表示された。

Draft retention retest evidence:

- Date: 2026-08-22
- Current commit: `bf0c1bcab86256ca22e6a7cf27a8598eb261ea22`
- App build: `6e7f02796365d9e243a4b4ebb5b93f515c5da7bb`のignored撮影専用binary
- Environment: Windows 11 host / 撮影専用identifier / 複製した隔離Room data
- Notes:
  - 空のcomposerではSendがdisabledで、空messageを送信できなかった。
  - 隔離Room dataを読取り専用handleで保持し、Codex宛てに非機密のtest draftを入力した。
    Ownerのaction-time承認後にSendを一度だけ実行した。
  - Room保存は意図どおり失敗し、英語UIに`The message could not be saved to Rust Room. The draft
    was kept so you can retry.`と表示された。test draftはcomposerへbyte-identically残り、
    Sendも再試行可能な状態を維持した。
  - 隔離`rooms.json`のSHA-256は送信前後とも
    `2D0008AE31C3B23DA50D86C9FBA1C971659F4E389A4BA9331BED1FE90A7761B8`で一致した。
    `ai-dispatch-ledger.json`を含む追加runtime fileは作成されず、外部AI dispatchへ進まなかった。
  - 読取り専用handleを解放し、撮影用M.I.O.を終了して対象windowが0件になったことを確認した。

Corrupt backup rejection retest evidence:

- Date: 2026-08-22
- Current commit: `e200767a2dbb1ed4c09b558088db0347618885aa`
- App build: `6e7f02796365d9e243a4b4ebb5b93f515c5da7bb`のignored撮影専用binary
- Environment: Windows 11 host / 撮影専用identifier / 複製した隔離Room data
- Notes:
  - M.I.O.がbackupとして認識する20桁file名
    `moe-room-backup-99999999999999999999.json`へ、意図的に不正なJSONを56 bytesで配置した。
  - 英語UIで`Restore latest…`を一度選ぶと`Confirm restore`へ変わり、二段階確認が維持された。
    Ownerの事前承認後、二度目を一度だけ実行した。
  - 破損backupは`Restore failed. Create a backup first.`と明示して拒否された。Room一覧、選択中Room、
    参加AI、既存message、composerは表示を維持し、app操作を継続できた。
  - 隔離`rooms.json`のSHA-256は実行前後とも
    `2D0008AE31C3B23DA50D86C9FBA1C971659F4E389A4BA9331BED1FE90A7761B8`で一致した。
    runtimeには元の`rooms.json`と意図的に壊したbackupの2 fileだけが残り、temp fileや追加backupは
    作成されなかった。
  - 撮影用M.I.O.を終了し、対象windowが0件になったことを確認した。

Workspace boundary retest evidence:

- Date: 2026-08-22
- Current commit: `52b6160fe53b4a649f2031e1bc058805c5b23896`
- App build: `6e7f02796365d9e243a4b4ebb5b93f515c5da7bb`のignored撮影専用binary
- Environment: Windows 11 host / 撮影専用identifier / 複製した隔離Room data
- Notes:
  - 隔離fixtureに通常directory `workspace-ordinary`と、別directoryを指すWindows junction
    `workspace-junction`を作成した。後者は`Directory, ReparsePoint`かつ`Junction`であることを
    filesystem metadataで確認した。
  - Windows folder pickerからjunctionを選択すると、英語UIは`Junctions and symbolic-link
    folders cannot be used as a workspace for safety.`と明示して拒否し、chat-onlyを維持した。
    Windows targetではjunctionとsymlinkを同じreparse-point境界で拒否する実装経路を検証した。
  - 同じpickerから通常directoryを選択すると、`Reading and editing is allowed inside
    workspace-ordinary.`と`workspace-ordinary is now the Codex workspace.`が表示され、保存済み
    workspace設定は選択した通常directoryだけを指した。
  - 撮影用M.I.O.を終了して通常directoryを同じfixture内の`workspace-ordinary-moved`へ退避renameし、
    保存済みpathをmissingにして再起動した。composerは`Codex workspace was not found · Check Room
    settings`、Room設定は`workspace-ordinary could not be found.`と明示し、`Change folder`と
    `Return to chat only`を表示した。
  - 隔離`rooms.json`のSHA-256は試験前後とも
    `2D0008AE31C3B23DA50D86C9FBA1C971659F4E389A4BA9331BED1FE90A7761B8`で一致した。missing再起動前後の
    `room-workspace.json`も`E4700AD9CDD13D655DCF657C9005F99974A740418AC5CBFA40F315306296C0E0`で一致し、
    runtime fileはこの2件だけだった。
  - 撮影用M.I.O.を終了し、対象windowが0件になったことを確認した。

MCP endpoint retest evidence:

- Date: 2026-08-22
- Current commit: `2768a5686589ac65ffa5d7fe4c90c89da739631e`
- Environment: Windows 11 host / current `target/debug/moe-desktop.exe` / per-run isolated runtime files
- Notes:
  - `MIO_MCP_TOKEN`未設定で起動したM.I.O.はprocessを継続し、`127.0.0.1:38474`を
    listenしなかった。短すぎる不正なserver tokenでも同じくprocessを継続し、listenerを
    開かなかった。
  - 有効なserver tokenでlistenerを起動し、異なるBearer tokenでPOSTすると63 msで
    HTTP 401を返した。認証失敗で待ち続けたり、MCP toolを実行したりしなかった。
  - 同じ隔離M.I.O.を終了してlistenerが消えた後のPOSTは34 msで接続不能errorを返し、
    別processがRoom処理を引き継がなかった。
  - Room、AI dispatch ledger、continuity、backup、Conductor、orchestration、workspace、app dataを
    一時directoryへ隔離し、既存workspace fileは変更しなかった。
  - これは製品endpointの直接測定であり、Codex側の実画面に出るbounded error表示は未検証である。
    そのためGate 1のMCP項目は未完了のままにした。

Codex client MCP bounded error retest evidence:

- Date: 2026-08-23
- Commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 build 26200.9168 host / active Codex `mcp__mio` client / isolated runtime files
- Notes:
  - 通常起動したM.I.O.へ、このCodex taskの実`mio_status`、`mio_room_list`、`mio_room_read`を接続した。
    statusは`ready: true`と4 capabilityを返し、Room一覧4件と、2件に制限した最新message pageを
    boundedに読めた。read結果はDirect continuity testのOwner messageとCodex replyに一致した。
  - M.I.O.を通常終了して対象windowが0件になった後の`mio_status`は約2.05秒で接続不能errorを返した。
    別processへの誤接続、永久loading、Room書込みはなかった。
  - ignoredの`<REPOSITORY_ROOT>/.tools/public-alpha1/mcp-client-error-retest-20260823/`へRoom dataと
    runtime fileを隔離し、`MIO_MCP_TOKEN`未設定で公開候補を起動した。同じ`mio_status`は約2.08秒で
    接続不能errorを返し、listener disabledがCodex側でもboundedに見えた。
  - 別の一時server tokenを生成して隔離M.I.O.だけへ渡し、Codex client tokenと不一致にした。
    `mio_status`は23 msで`unexpected server response: HTTP 401`を返した。token値は表示、記録、
    repository保存していない。
  - 各失敗はread-only status 1回だけで確認し、自動retryやOwner-proxy writeを行わなかった。隔離版を
    通常終了後、環境変数なしで公開候補を再起動し、`mio_status`が33 msで`ready: true`へ復帰した。
  - 以上によりCodex client側でも未設定、不正token、M.I.O.停止中のbounded errorを実確認できたため、
    Gate 1のMCP項目を完了とした。Gate 4のOwner-proxy writeとclean-environment試験は別途未完了である。

Conductor live delegation retest evidence:

- Date: 2026-08-22
- Current commit: `7d481a5676cd161c4f479876d49ac8e83d190749`
- App build: `7d481a5`相当のignored撮影専用binary
- Environment: Windows 11 host / 撮影専用identifier / 隔離Room data / English UI
- Notes:
  - CodexをConductor、GeminiとClaude Fableをworkerとして、各workerへ1件ずつ短いtaskを委譲する
    test messageを入力した。Ownerのaction-time承認後にSendを一度だけ実行し、再送しなかった。
  - 修正前の実機試験では、実際の委譲先`gemini`と`claude-code`が両方completedだったにもかかわらず、
    final synthesisが依頼文中の表示名から別workerを推測し、`Unknown: Claude Fable`を追加した。
    orchestration／dispatch ledgerとの不一致としてproduct bugと判定した。
  - `apps/desktop/src-tauri/src/room_orchestration.rs`で、`workerResults`を実行workerとstatusの完全かつ
    authoritativeな記録とし、owner request、task、worker本文、表示名、memory、historyからworkerや
    statusを追加、rename、duplicate、relabelしない規則をsynthesis promptへ追加した。
  - 表示名`Claude Fable`を含むowner requestと、実際の`claude-code: completed` 1件を使う回帰testを
    追加した。`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml room_orchestration --
    --nocapture`は13 passed、0 failedだった。Rustfmtと`git diff --check`も成功した。
  - 修正版の実機再試験では、final answerが`Completed: gemini and claude-code`、`Failed: none`、
    `Unknown: none`と表示し、依頼文の表示名から偽のunknown workerを追加しなかった。
  - 永続台帳は最新operationを`completed`、委譲先を`gemini`と`claude-code`の2件として記録した。
    同時刻の両dispatchも`completed`で、各reply messageが保存されていた。画面表示と台帳が一致した。
  - OBSERVED: 修正前の準備中、Conductor modeでClaude FableをRoomへ追加すると、recipient lock中でも
    Claude Fableのrecipient chipが一時表示された。Directへ切り替えてからConductorへ戻すとCodexだけへ
    正規化された。修正版ではこの追加操作を再試験しておらず、別のUI確認項目として残す。
  - 成功workerだけのdelegation経路と、表示名からの偽unknown追加が解消したことは確認できた。
    実際にworkerを失敗または結果不明へしたpartial / unknown経路は実画面で未検証だったため、
    この時点ではGate 1とGate 4のConductor項目を未完了のままとした。

Controlled partial / unknown live retest evidence:

- Date: 2026-08-23
- Current commit: `af92304b0b5b195b96bbb0b9b59bc5f478bb1689`
- App build: `7d481a5`相当のignored撮影専用binary
- Environment: Windows 11 host / 撮影専用identifier / 隔離Room data / English UI
- Notes:
  - Claude Code adapterだけを安全に結果不明へするため、撮影専用processの`MOE_CLAUDE_BIN`を
    `C:\Windows\System32\whoami.exe`へ限定した。Claude Code形式の引数ではlocal processが終了code 1に
    なり、実Claude providerへの送信は発生しない。Gemini adapterは通常設定のままにした。
  - 最初のcontrolled requestはGeminiと表示名`Claude Fable`を各1件指定したが、plannerが`gemini`だけを
    委譲した。画面と永続台帳で委譲1件を確認し、同じmessageは再送しなかった。
  - 続く独立したtest messageでは、`targetParticipantId`を`gemini`と`claude-code`の2件に固定した。
    Ownerのaction-time承認後にSendを一度だけ実行し、処理中も再送しなかった。
  - final answerは`gemini — completed`と`claude-code — unknown: No result was available to synthesize.`だけを
    表示した。さらにunknown結果が1件あり、duplicate turn防止のため再試行しなかった旨を明示した。
  - 最新orchestration ledgerはoperationを`completed`、delegation countを2、委譲先を`gemini`と
    `claude-code`として記録した。同時刻のdispatch ledgerは`gemini: completed`、
    `claude-code: externalStarted`で、完了結果のないexternal turnを画面が正しくunknownへ集約した。
  - 実workerのcompleted + unknown部分結果、明示status、非自動再送を実画面と台帳の両方で確認できた。
    これによりGate 1のConductor集約項目を完了とした。後日のOwner決定でhost-isolated runtimeを
    Gate 4の正式な受入環境としたため、Gate 4のConductor項目も完了とする。

## 2. Gate 2: experimental and unavailable states are honest

- [x] Codex CLIなしは`設定が必要`相当になり、Readyにならない。
- [x] Claude Fable CLI検出だけではReadyにならず、初回live reply前はInstalled相当になる。
- [x] Gemini CLIなしではReadyにならず、通常版Browser Bridgeは実験版と分かる。
- [x] Grok CLI検出だけではReadyにならず、初回live reply前はInstalled相当になる。
- [x] Claude Webは未接続であることが分かる。
- [x] ChatGPT Web、OpenAI API、Generic MCP client、Custom adapterは現在未対応と分かる。
- [x] Fable、Gemini、Grokのworkspace read / writeを選択できない。
- [x] 未接続AIへ送った時、demoまたは別AIのreplyで成功を偽装しない。
- [x] Provider `unknown outcome`を成功または自動再送へ読み替えない。
- [x] READMEの対応表と画面の状態表現が一致している。

Evidence:

- Date: 2026-08-17
- Commit: `1c539a1`
- Environment: Windows 11 host / `target/debug/moe-desktop.exe` / existing local Room
- Notes / screenshots:
  - `Claude Web`は`Web接続待ち`、Claude Fable / Grokは`CLIあり・初回返信待ち`と表示された。
  - `AIを追加`ではChatGPT、OpenAI API、Generic MCP、Custom AI / その他がすべて`現在未対応`のdisabled項目として表示された。
  - Gemini宛て4件の結果不明警告は成功扱いされず、`二重送信を防ぐため、自動再送していません`と表示された。
  - 2026-08-17の観察時点ではdestructive操作、provider送信、CLI missing構成、workspace権限を
    未検証としていた。CLI missingとworkspace権限は、下記の後続Sandbox証拠とtestで補完した。
  - 2026-08-19のGate 3 Sandbox再試験では、AI CLIなしでCodexが`設定が必要`、Geminiが
    実験版相当になり、未接続Codexへの送信がuser messageだけを保存してfake replyを
    作らないことを実画面で確認した。
  - 2026-08-21に`8507e2d`とGate 0文書／metadata候補を対象として、`moe-desktop` lib testを
    再実行した。118 PASS、0 FAIL、5 ignoredだった。
  - `normal_product_keeps_the_browser_experiment_disabled`、
    `reports_unsupported_recipients_without_faking_a_reply`、
    `validates_bounded_profile_and_avatar_data`が、通常版Geminiの`実験版のみ`、unsupported宛の
    reply不生成、非Codex workspace accessのbackend拒否を固定する。
  - `ParticipantProfileEditor`は`participant.id === "codex"`の場合だけworkspace read / writeを
    enabledにし、それ以外は`現在未対応`相当のdisabled radioを表示する。
  - READMEのCodex、Gemini、Fable、Grok、Claude Web、未対応adapter表をconnection status生成と
    `ParticipantBar`の表示mappingへ照合した。

## 3. Gate 3: no-AI-CLI Windows Sandbox smoke

### Isolation setup

- [x] Windows Sandboxまたは同等の破棄可能なclean Windowsを使用した。
- [x] Node.js、Rust、Codex、Claude、Gemini、Grokを導入していない。
- [x] Networkを無効にした。
- [x] Clipboard redirectionを無効にした。
- [x] 検査対象build folderだけをread-onlyで共有した。
- [x] HostのM.I.O. app data、Documents backup、credential、Room workspaceを共有していない。

### Smoke acceptance

- [x] M.I.O.がmissing DLLまたはruntime errorなしで起動する。
- [x] Talk Room、初期Room、参加AI一覧、composerが描画される。
- [x] Footerがbackend障害をdemo fallbackで隠さない。
- [x] 全AIが実態に合うsetup-required / unsupported状態になる。
- [x] 未接続AIへの送信がfake replyを生成せず、app操作を継続できる。
- [x] Room作成、rename、profile、外観設定を操作できる。
- [x] 同じSandbox session内のM.I.O.再起動でRoom dataが復元される。
- [x] 二重起動で2つのRoom writerが動かない。
- [x] 正常終了時にcrash dialogやentry-point errorが出ない。
- [x] AI CLI child processまたは意図しないnetwork依存を観測しない。

Evidence:

- Date: 2026-08-18
- Commit: `7f8507e`（検査した2つのEXEのsource commit）
- Windows version: Windows Sandbox / Windows 11 host（guest build番号は未記録）
- Artifact hash:
  - dynamic CRT:
    `056EDD24A9CF6ECFF6C0B801FD549D12723E197DEDA9F2B44EBCC1AB74AAA5FA`
  - static CRT diagnostic:
    `3F05E3D77EAC20DA3524C466B64717A98289CCA96342F4B88D35B3496AF5B810`
- Notes / screenshots:
  - PASS（Isolation setup）: network、clipboard、printer、audio / video input、vGPUを
    無効化し、ProtectedClientを有効化したWindows Sandboxを使用した。
  - PASS（Isolation setup）: 検査用folderだけを`C:\MIO-Alpha1`へread-onlyで共有し、
    Hostのapp data、backup、credential、workspaceを共有しなかった。
  - 通常release EXEは`VCRUNTIME140.dll`不足で起動前に停止した。EXE単体ではGate 3を
    満たさない。
  - 同じsourceを`-C target-feature=+crt-static`でbuildしたdiagnostic EXEでは上記DLL
    errorが解消し、次に`Could not find the WebView2 Runtime.`で停止した。
  - WebView2 Runtimeを導入していないclean SandboxではUI描画まで到達していないため、
    Smoke acceptanceはすべて未完了のまま維持する。
  - 次回はREADMEへ記載したMicrosoft Edge WebView2 Evergreen Runtimeだけを前提として
    offline Sandboxへ導入し、再現可能な静的CRT公開候補buildで残りを確認する。

Retest evidence:

- Date: 2026-08-19
- Commit: `5e54b58`（検査候補に使ったbuild script修正をそのまま固定したcommit）
- Windows version: Windows Sandbox / Windows 11 host（guest build番号は未記録）
- Artifact hash:
  `CE23BC0C7B9692595595FE5A8DDCBC6115CDD3D61AE0E5A8F2225948D4FAA08D`
- WebView2 installer hash:
  `57B4B8731044F5C7E60A045A22BBB115428E7EFF43E54645CDDE2C5FF1F9CBF1`
- Notes / screenshots:
  - Microsoft署名を検証済みのWebView2 Evergreen Standalone RuntimeだけをSandbox内へ
    導入し、AI CLI、Node.js、Rustは導入しなかった。
  - 修正候補は`localhost:1420`へ接続せず、埋め込まれたTauri frontendを表示した。
    `VCRUNTIME140.dll`、entry-point、runtime errorも発生しなかった。
  - Talk Room、初期Room、参加AI一覧、composer、Footerが描画された。demo messageは
    `UI DEMO`と明示され、空の新規Roomではdemo replyを生成しなかった。
  - Claude Webは`Web接続待ち`、Codexは`設定が必要`、Geminiは実験版相当として表示され、
    利用可能なAIとして偽装されなかった。
  - 新規Roomを作成して`g3`へrenameし、Codex profileの表示名を`cx`へ変更、外観をpinkへ
    変更できた。未接続の`cx`へ送信するとuser messageだけを保存し、fake replyを生成せず、
    結果不明を自動再送しない警告を表示した。
  - app内のclose buttonで正常終了し、同じSandbox sessionで再起動すると`g3`、message、
    `cx` profile、pink外観、結果不明警告が復元された。
  - 起動中に同じEXEを再度開くと既存Room windowへ戻り、第二のRoom windowやerror dialogは
    現れなかった。
  - Network無効のSandboxで上記操作を完了したため、UI/core smokeに外部networkは不要と
    確認した。
  - 2026-08-21に同じ隔離設定のfresh Sandboxへ、同じhashのWebView2 Runtimeだけを導入し、
    同じ公開候補EXEが埋め込みUIを正常描画することを再確認した。
  - Task Managerの詳細を名前順で全範囲確認した。M.I.O.本体と通常の
    `msedgewebview2.exe`群以外に、Codex、Claude、Gemini、Grok、Node等のAI CLI processは
    観測されなかった。Sandboxのnetworkは検証中を通して無効のままだった。

## 4. Gate 4: completed feature controlled-Windows pass

公開READMEに書いた導入元、version、署名またはhashを確認する。Providerの認証済み実CLIを使う
機能試験は、全M.I.O. runtimeをtest専用pathへ向け、旧dataを読み込まないcontrolled Windowsで行う。
Providerの利用規約、契約、課金、retentionは試験時点の実条件として記録する。

### Local product core

- [x] 初回起動と再起動。
- [x] Room lifecycleとparticipant membership。
- [x] Message persistence、idempotent retry、backup、restore。
- [x] Participant profileとdevice-local settings。
- [x] Direct modeの宛先選択とrecipient別結果。
- [x] Conductor modeのdirect answer、delegation、partial / unknown worker result。
- [x] Codex chat-onlyと、workspace read / writeがalpha.1でdisabled / fail-closedであること。
- [x] Local MCP read toolsとOwner-proxy writeの認証、idempotency、`via Codex` provenance。

### Provider adapters claimed by the release

- [x] Codex: documented installationから初回live reply、continuity、error表示まで確認した。
- [x] Gemini: documented installationから初回live reply、continuity、error表示まで確認した。
- [x] Claude Fable: documented installationから初回live reply、continuity、error表示まで確認した。
- [x] Grok: documented installationから初回live reply、continuity、error表示まで確認した。
- [x] 各Providerを個別に利用不能にし、ほかのrecipientとM.I.O.全体が継続することを確認した。

Evidence:

- Date: 2026-08-23
- Commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Windows version: Windows 11 host上のWindows Sandbox（Sandbox内のbuild番号は未記録）
- Provider / CLI versions: AI CLI未導入。Provider adapterのlive試験は未実施。
- Notes / screenshots:
  - Network、clipboard、printer、audio、video、vGPUを無効にし、検査対象folderだけをread-onlyで
    共有したfresh Sandboxを使用した。Node.js、Rust、AI CLI、旧app data、credential、continuity、
    backup、Room workspaceは持ち込まず、Microsoft署名を検証したWebView2 Evergreen Runtimeだけを
    導入した。
  - 公開候補を初回起動し、`Core + Room ready`まで到達した。新規Roomを作成して`gate4`へrenameし、
    Claude Fableを追加して`CLI設定が必要`表示を確認後に外した。別の一時Roomは二段階確認で削除し、
    標準Roomへ安全に戻った。
  - Codex profileの表示名を`cx4`へ変更し、device-localの外観をpinkへ変更した。workspaceを選択して
    いないため実効accessはchat-onlyのままとした。M.I.O.終了後に同じ候補を再起動すると、`cx4`、
    `gate4`、participant membership、pink外観が復元された。
  - `gate4`を含む状態でbackupを作成し、Room名を`temp4`へ変更後、`最新を復元...`の二段階確認を
    実行した。Room名とmembershipはbackup時点の`gate4`／`cx4`へ戻り、device-localのprofileと外観は
    維持された。
  - Direct composerで`cx4`だけを宛先にし、非機密test message `gate4local`を一度だけ送信した。
    Owner messageは1件だけ保存され、fake replyは生成されず、結果未確認を明示して二重送信防止のため
    自動再送しない警告が表示された。5秒後も追加messageはなく、M.I.O.再起動後もmessageと警告が
    復元された。
  - Network無効かつAI CLI未導入のため、Directのrecipient別live結果、Conductor、workspace境界、
    Local MCP、Provider adapterの各項目は未完了のまま維持した。

Clean Sandbox Codex CLI installation / provider smoke evidence:

- Date: 2026-08-23
- Mapped candidate commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- Environment: Windows Sandbox / `MIO-alpha1-live-providers.wsb` / network有効 / host folder
  `C:\MIO-Alpha1`へread-only mapping / clipboard、printer、audio、video、vGPU無効
- Provider / CLI versions: PowerShell 7.6.5 X64 / Codex CLI `0.149.0` / `Sign in with ChatGPT` /
  model `gpt-5.6-sol`
- Notes:
  - Microsoft署名がValidの`PowerShell-7.6.5-win-x64.msi`を使用した。SHA-256は
    `3A87C24E044EC792047D734C841917EE4323A535E25F645AE6C33141A35FCA8D`だった。
  - Windows PowerShell 5.1から公式standalone installerを実行すると、host supplemental evidenceと
    同じ`OSArchitecture` property errorで停止した。PowerShell 7.6.5から
    `irm https://chatgpt.com/codex/install.ps1 | iex`を一度だけ実行すると、Codex CLI `0.149.0`の
    導入に成功した。MSIやinstallerの重複起動は行っていない。
  - Clipboard redirectionが無効のため、ChatGPT認証は外側の既存account sessionからDevice Codeで
    完了した。password、API key、OAuth credential、one-time codeはrepositoryへ保存していない。
  - 初回起動でSandbox専用userのhome `C:\Users\WDAGUtilityAccount`を信頼し、推奨のdefault
    `elevated` Windows sandboxを設定した。完了後、CLIは`Sandbox ready`と入力欄を表示した。
  - file参照やcommand実行を依頼しない非機密prompt `Reply only GATE4_OK. No tools.`を一度だけ送信し、
    Codexは`GATE4_OK`を返した。これによりclean environmentで公式導入、認証、Codex自身のsandbox、
    providerへの初回live round tripを確認した。
  - Microsoft署名がValidのWebView2 Evergreen Runtime installer（SHA-256
    `57B4B8731044F5C7E60A045A22BBB115428E7EFF43E54645CDDE2C5FF1F9CBF1`）を導入し、Runtime
    `151.0.4129.10`を確認した。Runtime導入前に残ったheadlessなM.I.O. processをTask Managerで終了後、
    Sandbox Desktopへcopyした公開候補を起動すると、`Core + Room ready`とCodex `利用可能`へ到達した。
    read-only mapping内の原本は変更していない。
  - M.I.O.のDirect composerでCodexだけを宛先にし、Owner承認後に非機密test message
    `reply only miogatefourone`を一度だけ送信した。Owner messageは1件だけ保存され、Codexは
    `miogatefourone`と1件だけ返信した。重複messageや自動再送は観測されなかった。
  - 同じRoomとCodex宛先を維持し、`repeat previous token only`を一度だけ送信すると、識別子を再掲して
    いないにもかかわらずCodexは`miogatefourone`と1件だけ返信した。同一M.I.O. session内のnative
    continuityとrecipient別live resultを確認した。
  - bounded error確認では、Codexへ50桁同士の乗算を依頼する非機密test messageをOwner承認後に
    一度だけ送信した。Owner messageが1件保存され、Codexの処理中表示が出た時点でM.I.O.を通常終了した。
    同じ候補を再起動するとOwner message 1件、Codex reply 0件が復元され、UIは
    `Codexへの送信に、結果を確認できていないものが1件あります。二重送信を防ぐため、自動再送していません。`
    と表示した。10秒後も追加message、thinking／waiting、自動再送はなく、Codexは`利用可能`を維持した。
  - 以上によりM.I.O.経由のCodex初回reply、同一session内continuity、bounded error表示、再起動越しの
    非自動再送をclean environmentで確認し、Gate 4のCodex項目を完了とした。Direct全体のrecipient別結果と
    ほかのProviderは未確認なので、対応する項目は未完了のまま維持する。
  - 同じclean Sandbox sessionでRoomの指揮者をCodexに設定し、Conductor modeから非機密test message
    `do not delegate answer directly with mioconductorclean only`をOwner承認後に一度だけ送信した。Owner
    messageは1件だけ保存され、Codexはworkerへ委任せず`mioconductorclean`と1件だけdirect answerを返した。
    5秒後も重複message、thinking／waiting、errorはなく、CodexのConductor表示を維持した。これにより
    clean environmentでConductorのdirect-answer経路を確認したが、delegationとpartial／unknown worker
    resultは未確認なのでGate 4のConductor項目は未完了のまま維持する。
  - 続けてOwner承認後に非機密test message
    `delegate one task to gemini ask it to reply geminiworkerclean then synthesize and report the exact worker status`
    を一度だけ送信した。CodexはGeminiが`availableWorkers`に存在しないため委任できないと判断し、正確な
    worker状態を「利用可能なワーカーは0人」と1件だけ返した。6秒後もworker message、重複message、
    thinking／waiting、errorは増えなかった。これはclean environmentでの委任不可判定を確認した証拠であり、
    実際のdelegationおよびpartial／unknown worker resultは未確認なのでGate 4のConductor項目は未完了とする。
  - Google公式Windows installer `https://antigravity.google/cli/install.ps1`からAntigravity CLIを導入し、
    `%LOCALAPPDATA%\agy\bin\agy.exe --version`が`1.1.19`を返すことを確認した。Google OAuthの開始までは
    成功したが、SandboxではHTTPS URLの既定handlerがなく自動openに失敗し、clipboard redirectionも無効な
    ため、長いauthorization URLの安全な受け渡しとSandbox内での手動credential入力が必要になった。この
    sessionではOwnerの負担を増やさず認証前で停止した。Geminiのlive reply、continuity、error表示は未確認
    なので、Gate 4のGemini項目とConductor delegation項目は未完了のまま維持する。

Host Codex installation / live reply supplemental evidence:

- Date: 2026-08-23
- Commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 build 26200.9168 host / PowerShell 7.6.4 X64 / 既存local Room data
- Provider / CLI version: Codex CLI `0.149.0` / `Sign in with ChatGPT`
- Notes:
  - Windows PowerShell 5.1でstandalone installerを実行すると、`OSArchitecture` propertyを取得できず
    停止した。PowerShell 7.6.4から同じ公式installerを実行すると導入に成功し、`codex login status`は
    `Logged in using ChatGPT`を返した。READMEのCodex手順をPowerShell 7前提へ修正した。
  - 公開候補をstandalone CLIの絶対pathを`MOE_CODEX_BIN`へ指定して起動した。Codex participantは
    `Ready`となり、宛先をCodexだけにして非機密test messageをOwnerのaction-time承認後に一度だけ
    送信した。
  - Codexは`MIO_HOST_CODEX_OK`と1件だけ返信した。5秒後も重複返信、thinking表示、unknown／error表示は
    なく、Codexは`Ready`を維持した。
  - 継続確認ではcomposerをDirectへ切り替え、Codexだけを宛先にした。1通目で識別子
    `MIO_CONTINUITY_A23`を記憶させ、Codexが同じ文字列だけを1件返信したことを確認してから、M.I.O.を
    通常終了した。
  - `MOE_CODEX_BIN`を指定せず同じ公開候補を再起動しても、標準導入先のstandalone Codex CLIが検出され、
    Codexは`Ready`へ復帰した。Room履歴とDirect composerが復元され、continuity bindingのsession IDは
    再起動前後で`01a02d7d-d95e-7121-9365-0fecac5cf6bc`を維持した。
  - 再起動後の2通目には識別子を再掲せず、再起動前のDirect turnで記憶した識別子だけを返すよう依頼した。
    Codexは`MIO_CONTINUITY_A23`と1件だけ返信し、7秒後も追加返信、thinking／waiting、unknown／error表示は
    なく、`Ready`を維持した。
  - continuity bindingの`lastSyncedMessageId`は1通目後の
    `message-1e277acb-5648-4fbd-aadd-e48abd67819f`から、2通目後の
    `message-4709f41b-ecd4-42c9-9508-136148ba3543`へ進んだ。continuity fileのSHA-256も
    `B25D5CCD27F904586CD61682F928F73AB43E930F92C6D961040103CFDAF39561`から
    `97C203B413D5F7E0BF0164AADFE2C441D9B75279326F519FEF92EFC4AA20CD5F`へ変化した。
  - bounded error確認では、既存Room snapshotの隔離copyと、Room、dispatch ledger、continuity、
    Conductor、orchestration、workspace、backupの全runtime pathをignoredの
    `<REPOSITORY_ROOT>/.tools/public-alpha1/codex-error-retest-20260823/`へ向けた。`MOE_CODEX_BIN`は
    存在しない隔離path `missing-codex.exe`へ明示し、実Provider processを起動できない条件にした。
  - 明示launcherが設定済みのためCodexは画面上`Ready`となった。Codexだけを宛先にしてOwner承認後に
    非機密test messageを1回送信すると、Owner messageは1件だけ保存され、Codex replyは0件だった。
    UIは`The message may have reached Codex. It was not retried to prevent a duplicate turn.`とbounded errorを
    表示した。7秒後も重複message、thinking／waitingはなく、Codexは`Ready`を維持した。
  - 隔離dispatch ledgerにはsource message
    `message-60526a9d-faae-4a21-a9a6-300d29af61a7`、recipient `codex`、安定reply ID
    `reply-codex-cccdb486c254930d`の1 recordだけが`externalStarted`として残った。missing launcherのspawn前に
    external phaseを記録するため、結果不明として保守的に保持する経路である。
  - 同じ隔離runtimeでM.I.O.を再起動すると、Owner message 1件とreply 0件が復元され、
    `There are 1 messages to Codex whose results are unknown. They were not retried to prevent duplicate turns.`を
    表示した。自動再送、thinking／waiting、追加messageはなく、再起動越しの非自動再送も確認した。
    その後は隔離版を通常終了し、環境変数なしで公開候補を再起動して通常local dataへ戻した。
  - 以上により既存local RoomでのCodex初回reply、Direct continuity、bounded error表示、非自動再送を
    確認できた。clean environmentの証拠ではないため、Gate 4のDirect／Codex各項目は未完了のまま
    維持した。

Host Gemini / Antigravity supplemental evidence:

- Date: 2026-08-24
- Commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / 既存local Room data
- Provider / CLI version: Antigravity CLI `1.1.18`
- Notes:
  - 標準導入先`%LOCALAPPDATA%\agy\bin\agy.exe`を検出し、公開候補を通常起動するとGemini participantは
    `Installed`と表示された。同じhost Room履歴にはGeminiからの既存live replyが複数残っており、
    英語指定への英語replyと、続く日本語指定への日本語replyを実画面で再確認した。
  - UIは`There are 4 messages to Gemini隊長 whose results are unknown. They were not retried to prevent duplicate
    turns.`を表示し、結果不明を成功扱いせず自動再送しない既存のbounded recovery状態も維持していた。
  - 追加のGemini-only test messageはOwner承認後に入力したが、送信直前にcomposerが
    `Saved to the Room only for disconnected AI · No reply is available`と表示した。この条件ではlive reply
    検証にならないためSendを実行せず、新しいOwner message、Provider dispatch、重複turnを作らなかった。
  - 以上はhost上の既存live replyと非自動再送表示を再確認した補足証拠であり、clean environmentでの
    documented installation、明示的continuity、bounded errorの新規一連試験ではない。Gate 4のGemini項目と
    Conductor delegation項目は未完了のまま維持する。

Host-isolated Codex + Gemini Direct / Gemini continuity evidence:

- Date: 2026-08-24
- Repository commit: `ee1af3e0e8524865e75eb037ab0883ff3125b788`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / ignored host-isolated runtime / test-only Room
- Provider / CLI versions: Codex CLI `0.149.0` / Antigravity CLI `1.1.19`
- Notes:
  - Ownerの判断によりWindows Sandboxを今後のProvider試験には使用せず、通常M.I.O.を終了してから公開候補を
    起動した。Room、dispatch ledger、continuity、Conductor、orchestration、workspace、backupの全runtime
    pathをignoredの`<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-gate4-20260824/`へ向けた。
  - 起動直後に製品組込みの3つのUI demo Roomだけが表示され、既存local Room履歴は読み込まれなかった。
    test専用の`New room 4`を作成し、CodexとGeminiだけを参加させた。初回送信前のmessage数は0だった。
  - Codexは`Ready`、Geminiは`Installed`と表示された。両方をDirect宛先にし、Ownerのaction-time承認後、
    file参照やtool実行を依頼しない非機密test messageを一度だけ送信した。Owner messageは1件だけ保存され、
    Codexは`MIO-G4-DIRECT-1 OpenAI`、Geminiは`MIO-G4-DIRECT-1 Google`とそれぞれ1件だけ返信した。
  - dispatch ledgerは同じsource messageに対するrecipient `codex`と`gemini`を各1件だけ`completed`として記録し、
    画面のrecipient別replyと一致した。重複message、自動再送、errorはなく、Gemini表示は初回成功後に`Ready`へ
    更新された。これによりGate 4のDirect mode宛先選択とrecipient別結果を完了とした。
  - 続けてCodexを宛先から外し、Geminiだけへ直前replyのprovider wordを返すよう依頼した。OwnerがSendを
    一度だけ実行するとGeminiは`Google`と1件だけ返信した。Geminiのcontinuity bindingは同じopaque sessionを
    維持し、`lastSyncedMessageId`だけが2通目のsource messageへ進んだ。追加dispatchも1件だけ`completed`だった。
  - bounded errorは別のignored host-isolated runtime
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-gemini-missing-after-detect-20260824/`で確認した。実物の
    Antigravity CLIには触れず、`whoami.exe`をtest専用`agy.exe`としてruntime内へ複製してGeminiを一度
    `Installed`として検出させた後、その複製だけを同じignored root内の別名へ移動した。選択済みのGeminiだけへ
    非機密test messageをOwner承認後に一度送信し、Provider processや外部送信を開始できない状態を意図的に作った。
  - 画面にはOwner messageが1件だけ保存され、Gemini replyは作られず、結果不明と「重複ターン防止のため再試行
    しなかった」旨が表示された。dispatch ledgerは該当source messageについてrecipient `gemini`の1件だけを
    `externalStarted`として保持した。約1分後もsnapshotとledgerの更新時刻、該当dispatch件数、reply件数は変化せず、
    自動再送がないことを確認した。M.I.O.は応答を維持し、Codexは`Ready`のままだった。
  - 同じcontrolled missing-after-detect runtimeで、Codexと選択済みGeminiを同時宛先にした混在recipient試験も
    一度だけ実施した。source message `message-79907825-26c7-4493-9480-bf4ac19761c5`に対し、Codexは
    `MIO-G4-GEMINI-MIXED-1 OpenAI`を1件だけ返信し、Gemini replyは作られなかった。画面にはGeminiについてだけ
    `The message may have reached Gemini隊長. It was not retried to prevent a duplicate turn.`と表示され、
    M.I.O.と入力欄は応答を維持した。
  - 同じsource messageのdispatch ledgerはrecipient `codex`を1件だけ`completed`、recipient `gemini`を1件だけ
    `externalStarted`として保持した。Codex continuityだけがこのsource messageへ進み、Gemini continuityは直前の
    messageのままだった。8秒後もsnapshot、ledger、continuityはそれぞれSHA-256
    `4D882FA74AAA0EE6081EB8C3724456D982304041ADDBB6861CD697A4C2C22D67`、
    `36D8FFDE2477F3095131FB945FCD745057116516648E1F4A9233A306171AE907`、
    `D6C29BFE9C5A3C03C5B5B072C89FC517C30B0118324D8EFEB7E701FA82514974`で不変だった。成功recipientを失敗扱いに
    せず、結果不明recipientだけを再送しない分離動作を確認した。
  - clean SandboxでのGoogle公式installerによるAntigravity CLI `1.1.19`導入、host-isolated runtimeでの初回live
    replyと明示的continuity、controlled missing-after-detectでのbounded errorを組み合わせ、Gate 4のGemini項目を
    完了とした。ほかのProviderを個別に利用不能にする試験は残るため、全Provider unavailable項目は未完了のままとする。

Host-isolated Claude Fable Direct / continuity / bounded error evidence:

- Date: 2026-08-24
- Repository commit: `f3513d6d332834371a7e88b1456599ab45006dae`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / ignored host-isolated runtime / test-only Room
- Provider / CLI version: Claude Code `2.1.240` / SHA-256
  `F7EE87C58D315BEDBD38FC8923CE7B955DBBAB2AFE50D9D15C059463A70869DA`
- Notes:
  - Anthropicのdocumented Windows手順にあるWinGet package `Anthropic.ClaudeCode`からClaude Codeを導入し、
    `claude auth status`でClaude.ai subscription login済みを確認した。account識別子は記録していない。
  - 公開候補へClaude Codeの絶対pathを`MOE_CLAUDE_BIN`として指定し、Room、dispatch ledger、continuity、
    Conductor、orchestration、workspace、backupの全runtime pathをignoredの
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-claude-gate4-20260824/`へ向けた。起動時に既存local Roomは
    読み込まれず、test専用の`New room 4`を使用した。
  - Claude FableだけをDirect宛先にしてOwnerが最初の非機密test messageを一度送信すると、Owner messageと
    `MIO-G4-FABLE-1 Anthropic`が各1件だけ保存された。dispatch ledgerはrecipient `claude-code`の1件だけを
    `completed`として記録し、Claude Fable表示は`Installed`から`Ready`へ更新された。
  - 継続確認では直前replyのprovider wordだけを返すよう依頼した。Claude Fableは`Anthropic`を1件だけ返信し、
    continuity bindingは同じopaque sessionを維持したまま`lastSyncedMessageId`を2通目のsource messageへ進めた。
    最終snapshotはOwner message 2件とClaude reply 2件、dispatch ledgerは`completed` 2件で、各本文の完全一致数は
    それぞれ1件だった。重複messageと自動再送はなかった。
  - bounded errorは別のignored runtime
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-claude-missing-after-detect-20260824/`で確認した。実Claude CLIへ
    触れず、SHA-256 `23240EF9F8B0A9A324110B1C2331DE31DC1B0E08F5359CB707E51A939AF56CD3`の
    `whoami.exe`をtest専用`claude.exe`として複製し、Claude Fableを`Installed`として検出させた。
  - Claude Fableだけを宛先にしてOwnerが`MIO-G4-FABLE-ERROR-1`を一度送信すると、local `whoami.exe`は
    Claude形式の引数で非zero終了し、Anthropicへの送信は発生しなかった。画面にはOwner messageが1件だけ保存され、
    Claude replyは0件で、結果不明と「重複ターン防止のため再試行しなかった」旨が表示された。
  - 隔離dispatch ledgerは該当source messageについてrecipient `claude-code`の1件だけを`externalStarted`として
    保持した。7秒後もsnapshotとledgerのSHA-256、message数、dispatch数、reply数は変化せず、自動再送がないことを
    確認した。実Claude CLIは変更なく残り、Codexは`Ready`、M.I.O.は応答を維持した。
  - 同じcontrolled missing-after-detect runtimeで、Codexと選択済みClaude Fableを同時宛先にした混在recipient
    試験も一度だけ実施した。source message `message-9998bf55-b800-40fd-8f55-f9d57440b5a4`に対し、Codexは
    `MIO-G4-FABLE-MIXED-1 Codex`を1件だけ返信し、Claude Fable replyは作られなかった。画面にはFableについてだけ
    `The message may have reached Claude Fable. It was not retried to prevent a duplicate turn.`と表示され、
    M.I.O.と入力欄は応答を維持した。
  - 同じsource messageのdispatch ledgerはrecipient `codex`を1件だけ`completed`、recipient `claude-code`を
    1件だけ`externalStarted`として保持した。Codex continuityだけがこのsource messageへ進み、Fable continuityの
    新しいbindingは作られなかった。8秒後もsnapshot、ledger、continuityはそれぞれSHA-256
    `F92074EEFB644FDC9708DAFD0F5EADF0E769F523A4944FBC52B9F925D2494FBD`、
    `9D8F6689B133DD7A9A07404FD4510B065759F68CF439639E2C26155CB4FC818C`、
    `F04C925F9206A05EAE139EB8CA2F01E7B712401AD701713B3C9C96DE61833371`で不変だった。成功recipientを失敗扱いに
    せず、結果不明recipientだけを再送しない分離動作を確認した。
  - documented installation、host-isolated初回live reply、同一sessionの明示的continuity、controlled invalid
    launcherのbounded errorを組み合わせ、Gate 4のClaude Fable項目を完了とした。

Host-isolated Grok Direct / continuity / bounded error evidence:

- Date: 2026-08-24
- Repository commit: `542abd99574d68b3d9fd81ae24ffcaa42a3c7bd0`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / ignored host-isolated runtime / test-only Room
- Provider / CLI version: Grok CLI `0.2.77 (44e77bec3a) [stable]` / SHA-256
  `128289DD81265EAECA6E144845AD9DA5F6E5BE942CE1E17A93D9C65130064E1E`
- Notes:
  - 標準導入先`%USERPROFILE%\.grok\bin\grok.exe`を使用した。Authenticode署名は`Valid`で、署名者は
    `X.AI LLC`だった。account識別子とcredentialは記録していない。
  - 公開候補へGrok CLIの絶対pathを`MOE_GROK_BIN`として指定し、Room、dispatch ledger、continuity、
    Conductor、orchestration、workspace、backupの全runtime pathをignoredの
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-grok-gate4-20260824/`へ向けた。起動直後は製品組込みの
    3つのUI demo Roomだけが表示され、既存local Room履歴は読み込まれなかった。test専用の`New room 4`を
    作成し、CodexとGrokを参加させたが、Direct宛先はGrokだけにした。
  - Ownerのaction-time承認後、file参照やtool実行を依頼しない非機密test message
    `Reply with exactly one line: MIO-G4-GROK-1 xAI. Do not use tools or access files.`を一度送信した。
    Owner messageとGrok reply `MIO-G4-GROK-1 xAI.`は各1件だけ保存され、Grok表示は`Ready`を維持した。
  - 継続確認では直前replyのprovider wordだけを返すよう依頼した。OwnerがSendを一度だけ実行すると、Grokは
    `xAI`を1件だけ返信した。最終snapshotはOwner message 2件とGrok reply 2件、dispatch ledgerはrecipient
    `grok`の一意な2件をいずれも`completed`として保持した。continuity bindingは1件だけで、同じopaque sessionを
    維持し、`lastSyncedMessageId`は2通目のsource messageと一致した。重複messageと自動再送はなかった。
  - bounded errorは別のignored runtime
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-grok-missing-after-detect-20260824/`で確認した。
    `MOE_GROK_BIN`を、Grok引数を受けるとnonzeroで終了するtest専用のMicrosoft署名済みlauncher
    `fake-cli/grok.exe`へ向けた。launcherのSHA-256は
    `23240EF9F8B0A9A324110B1C2331DE31DC1B0E08F5359CB707E51A939AF56CD3`で、実際の
    `%USERPROFILE%\.grok\bin\grok.exe`とcredentialは変更していない。
  - test専用Room `New room 4`でDirect宛先をGrokだけにし、Ownerのaction-time承認後、非機密test message
    `MIO-G4-GROK-ERROR-1`を一度送信した。snapshotにはOwner message
    `message-affbd5c4-b63d-48be-a190-3431163b3bb9`が1件だけ保存され、Grok replyは作成されなかった。
    UIは`The message may have reached Grok. It was not retried to prevent a duplicate turn.`を表示し、
    Sendをdisabledにした。結果不明を成功扱いせず、重複の可能性がある外部turnを再送しない表示だった。
  - dispatch ledgerは
    `room-message:message-affbd5c4-b63d-48be-a190-3431163b3bb9:grok`の1件だけを`externalStarted`として保持し、
    continuity bindingは作成されなかった。8秒後の再確認でもsnapshotとledgerは変化せず、自動再送と重複messageは
    なかった。最終SHA-256はsnapshot
    `B0496C0DC560D623211ACE4C86CB72A5CCC8BCAA2E99A1AF4E61EB9ABC7AF5AD`、dispatch ledger
    `341DCE007BC4C8D1D4AD8FAF7FAA5DFAD8A47566C3C16980FB0404E9364848A6`だった。
  - 同じcontrolled invalid launcher環境で、Grok利用不能時のmixed-recipient isolationも確認した。CodexとGrokを
    Direct宛先にし、Ownerのaction-time承認後、非機密test message
    `Reply with exactly one line: MIO-G4-MIXED-1 <provider>. Do not use tools or access files.`を一度だけ送信した。
    Owner message `message-8c34c70f-4765-4fd6-bebc-94e45adb3d6e`は1件だけ保存され、Codexは
    `MIO-G4-MIXED-1 Codex`を1件返信した。Grok replyは作成されず、UIはGrokだけについて結果不明と非自動再送を
    表示した。Codexの成功結果は失われず、M.I.O.全体とRoom入力欄は応答を維持した。
  - 同じsource messageのdispatch ledgerはrecipient `codex`を`completed`、recipient `grok`を
    `externalStarted`として各1件だけ保持した。Codex continuity bindingだけがsource messageまで進み、Grok bindingは
    作成されなかった。8秒後もsnapshot、dispatch ledger、continuityは変化せず、自動再送と重複messageはなかった。
    最終SHA-256はsnapshot `F89ABBD8AB4C6EDFFA4E15E769886983F7171BDE84CB8B93E9D9EA5BCF62F380`、
    dispatch ledger `532EA4779EBA3E329A81963AB6E9C2A95BF656B3BE35C014262E887D0E84AB78`、continuity
    `32A2591858916B1DBD4D2F14101A1DC4A2A6582599D929381A13EE4AAADCA649`だった。これによりGrok利用不能時に
    ほかのrecipientとM.I.O.全体が継続することを確認した。ほかのProvider分は残るため、全Provider unavailable項目は
    未完了のままとする。
  - 公開READMEのxAI公式導入手順、標準導入先で署名を検証した実CLI、host-isolated初回live reply、同一sessionの
    明示的continuity、controlled invalid launcherのbounded errorを組み合わせ、Gate 4のGrok項目を完了とした。

Host Local MCP read / Owner-proxy supplemental evidence:

- Date: 2026-08-23
- Commit: `decd2c2de6d50a52b5a8350cf86482d2053fdd0e`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 build 26200.9168 host / active Codex `mcp__mio` client / isolated runtime files
- Notes:
  - 正常系の`mio_status`、`mio_room_list`、bounded `mio_room_read`と、未設定、不正token、停止中の
    Codex client errorはGate 1のCodex client MCP retest evidenceへ記録した。
  - Owner-proxy writeでは、既存Room snapshotの隔離copyと全runtime pathをignoredの
    `<REPOSITORY_ROOT>/.tools/public-alpha1/mcp-owner-proxy-retest-20260823/`へ向け、通常clientと同じ
    有効な`MIO_MCP_TOKEN`で公開候補を起動した。token値は表示、記録、repository保存していない。
  - Ownerのaction-time承認後、recipient `codex`、request ID
    `public-alpha1-mcp-idempotency-20260823`で非機密test messageを2回呼んだ。1回目は`appended`、
    同一payloadの2回目は`duplicate`を返し、両方とも同じmessage ID
    `mcp-owner-public-alpha1-mcp-idempotency-20260823`と同じ作成時刻を返した。
  - `mio_room_read`と隔離snapshotの双方でtest messageは1件だけだった。authorは`owner`、recipientは
    `codex`、provenanceは`codexOwnerProxy`で、UIにも`via Codex` badgeが表示された。
  - toolの契約どおりAI dispatchとConductorは開始せず、thinking／waiting、AI replyはなく、隔離runtimeに
    AI dispatch ledgerとorchestration ledgerは作成されなかった。最終Room snapshotのSHA-256は
    `A9582CF17433F2E407FD4C822C1788DADF15F38B2E4B8B612FB0B4FFB5E48BB4`だった。
  - 隔離版を通常終了後、環境変数なしで公開候補を再起動した。通常Roomにはtest messageが存在せず、
    Codexは`Ready`、`mio_status`は43 msで`ready: true`へ復帰した。
  - 以上によりhost隔離環境でLocal MCP read、認証、Owner-proxy idempotency、immutable provenance、
    `via Codex`表示を確認した。後日のOwner決定でhost-isolated runtimeをGate 4の正式な受入環境としたため、
    Gate 4のLocal MCP項目も完了とする。

## 5. Automated verification

- [x] `npm.cmd ci`
- [x] `npm.cmd run typecheck`
- [x] `npm.cmd run build`
- [x] `npm.cmd run test --workspace @moe/spike-codex-app-server`
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `git diff --check`
- [x] GitHub ActionsのWindows CIがrelease candidate commitで成功した。
- [x] Environment-dependent ignored testsの対象と実施結果を記録した。

Evidence:

- Date: 2026-08-21
- Commit: `53d6636`
- Environment: Windows 25H2 (build 26200.9168。registry上のProductNameはWindows 10 Pro)
- CI URL: https://github.com/blackcometclub/M.O.E/actions/runs/32429483300
- Notes:
  - `npm.cmd ci`は127 packagesを再導入し、auditは0 vulnerabilitiesだった。
  - TypeScript typecheck、Vite production build、Codex App Server evidence test 2件は
    すべて成功した。
  - `cargo test --workspace`は197 passed、0 failed、5 ignoredだった。Windows test EXEは
    Tauri既定のCommon Controls v6 manifestをlink時に一度だけ埋め込む修正候補により、
    手動加工なしで起動した。
  - Windows SDK `mt.exe`で同test EXEのmanifest resource `#1`を抽出し、
    Common Controls v6 dependencyと`asInvoker`設定が埋め込まれていることを確認した。
  - ignored 5件は、認証済みCodex App Serverを必要とするlive test 4件と、Windows
    Credential Managerへ隔離targetを作成・削除するtest 1件。環境依存／外部state変更を
    伴うため、この自動検証では実行していない。
  - `cargo +stable clippy --workspace --all-targets -- -D warnings`、Rustfmt、
    `git diff --check`はすべて成功した。
  - GitHub Actions run `32429483300`のWindows checksは10分3秒で完了し、JavaScript
    dependency導入、typecheck、frontend build、evidence test、Rustfmt、Rust workspace
    testを含む全stepが成功した。

Retest evidence:

- Date: 2026-08-22
- Commit: `aae09d3`
- Environment: Windows 11 host
- Notes:
  - `cargo test -p moe-desktop`は118 passed、0 failed、5 ignoredだった。
  - Desktop Rust testで、backup export / restore validation、corrupted primaryの隔離と
    valid backupからの復旧、選択時または選択後に差し替えられたWindows junction rootの拒否、
    MCP Bearer tokenのbounded validation／debug redaction／厳密比較、MCP Streamable HTTPの
    Bearer必須、external turn unknown結果の非自動再送、Conductor workerのpartial result集約と
    unknown synthesisの非再送を再確認した。
  - `npm.cmd run typecheck --workspace @moe/desktop`と`git diff --check`は成功した。
  - ignored 5件は、認証済みCodex App Serverを使うlive test 4件と、Windows Credential
    Managerへ隔離targetを作成・削除するtest 1件。環境依存／外部state変更を伴うため、
    この再検証では実行していない。
  - Computer UseはM.I.O.のwindowを選択する前にhelper初期化で失敗し、M.I.O.へのclick／typingは
    0件だった。これはproduct failureとは判定せず、Gate 1の未完了UI確認も完了扱いにしない。

Pinned Actions CI evidence:

- Date: 2026-08-22
- Commit: `3a32d49`
- CI URL: https://github.com/blackcometclub/M.I.O/actions/runs/32548861306
- Notes:
  - `actions/checkout`と`actions/setup-node`を、公式repositoryの`v6` tagが指す完全commit SHAへ
    固定した。
  - Windows checksは8分43秒で完了し、checkout、Node.js setup、JavaScript dependency導入、
    typecheck、frontend build、evidence sanitization test、Rustfmt、Rust workspace testを含む
    全stepが成功した。

Codex unavailable mixed-recipient regression evidence:

- Date: 2026-08-26
- Source: `687de38`
- Notes:
  - `ai_dispatch::tests::keeps_other_recipient_success_when_codex_outcome_is_unknown`を追加し、同じOwner
    messageのCodexを`TimedOut`、Grokを正常応答とするin-memory adapterで検証した。
  - Codex resultは`Unknown`、ledgerは`externalStarted`のまま、replyとcontinuity bindingは作られなかった。
    Grok resultは`Completed`となり、replyが保存され、continuityの`lastSyncedMessageId`だけがsource messageへ進んだ。
  - 同じsource messageを再度dispatchすると、Codexは`aiDispatchOutcomeUnknown`、Grokは既存replyを返す
    `Duplicate`となり、結果不明recipientを再送せず、成功recipientを失わないことを確認した。
  - 対象test、`ai_dispatch::tests` 22件、`cargo test -p moe-desktop`は120 passed、0 failed、5 ignoredだった。
    `cargo +stable clippy -p moe-desktop --all-targets -- -D warnings`、`cargo fmt --all -- --check`、
    `git diff --check`も成功した。
  - これはbackendの回帰証拠であり、公開候補実画面のwarning、recipient別表示、入力継続はまだ確認していない。
    そのためGate 4の各Provider unavailable項目は未完了のまま維持する。

Codex unavailable mixed-recipient live-attempt evidence:

- Date: 2026-08-26
- Source: `687de38`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / ignored host-isolated runtime / test-only Room
- Notes:
  - Codexだけをcontrolled unavailableにするため、`MOE_CODEX_BIN`をMicrosoft署名済みの
    `whoami.exe`の隔離copyへ向けた。Grokは署名確認済みの実CLI
    `0.2.77 (44e77bec3a) [stable]`を`MOE_GROK_BIN`で指定した。実credentialは変更・記録していない。
  - 1回目は既存の合格済みGrok runtimeからRoom snapshotとcontinuityを隔離copyして、非機密test message
    `Reply with exactly one line: MIO-G4-CODEX-UNAVAILABLE-1 <provider>. Do not use tools or access files.`を
    Ownerが一度だけ送信した。CodexとGrokはともにresult unknownとなり、replyは作成されなかった。同じsource
    messageは再送しなかった。
  - stale continuityの影響を除くため、2回目はRoom snapshotとdispatch ledgerだけを新しいignored runtimeへcopyし、
    continuity fileを持ち込まずに公開候補を再起動した。前回warningを明示的に閉じ、CodexとGrokを選択して、
    一意な非機密test message `MIO-G4-CODEX-UNAVAILABLE-2`をOwnerが一度だけ送信した。
  - 2回目のOwner message `message-829a1bbd-7d82-438f-b867-60b117e3c3f8`は1件だけ保存されたが、
    CodexとGrokはともに`externalStarted`のままでreplyを作成しなかった。UIは両recipientについて
    `may have reached`と非自動再送を表示し、M.I.O.自体は応答を維持した。continuity fileは作成されなかった。
  - 最終SHA-256はsnapshot `91922F900D925010212D72EF013348B3A82AFB7C27D4388949CF66D9500BF609`、
    dispatch ledger `E853E5CCCD8749CAD4DC79ABFA84D2F92F7B285FDD763FBB7634783384289FC5`だった。
    backend回帰testと異なり、公開候補実画面ではGrokの成功継続を確認できなかったため、このlive attemptは
    Codex unavailable mixed-recipient isolationの合格証拠にせず、Gate 4の全Provider unavailable項目は未完了のままとする。

Codex unavailable mixed-recipient live-pass evidence:

- Date: 2026-08-26
- Repository HEAD: `6d209e06165f4b282246b58f1c44b109c0265963`
- App build: `moe-desktop-alpha1-decd2c2.exe` / SHA-256
  `BA6C4A20EBB07D7865925DAC0BE46DEEBC272AF0F9E720238E4DA8618294E09B`
- Environment: Windows 25H2 host / ignored host-isolated runtime / test-only Room
- Notes:
  - 前2回のGrok側result unknownを成功扱いせず、独立したignored runtime
    `<REPOSITORY_ROOT>/.tools/public-alpha1/host-isolated-codex-unavailable-gemini-20260826/`を作成した。
    合格済みGemini Room snapshotだけをcopyし、continuityとdispatch ledgerは持ち込まなかった。
  - Codexだけをcontrolled unavailableにするため、Microsoft Windows署名がValidな`whoami.exe`の隔離copyを
    test専用`codex.exe`として`MOE_CODEX_BIN`へ指定した。SHA-256は
    `23240EF9F8B0A9A324110B1C2331DE31DC1B0E08F5359CB707E51A939AF56CD3`だった。GeminiはGoogle LLC署名が
    Validな実Antigravity CLI `1.1.21`を使用し、SHA-256は
    `2947E70D02CD2206B043566170439613BE1723F0DF3D417314A4AC2220AD9B26`だった。credentialは変更・記録していない。
  - CodexとGeminiをDirect宛先にし、Ownerが非機密test message
    `Reply with exactly one line: MIO-G4-CODEX-UNAVAILABLE-GEMINI-1 <provider>. Do not use tools or access files.`を
    一度だけ送信した。Owner message `message-6a96998e-60c3-4cbc-894b-9c5151708a7d`は1件だけ保存された。
  - Geminiは`MIO-G4-CODEX-UNAVAILABLE-GEMINI-1 Google`を1件だけ返信した。reply IDは
    `reply-gemini-d714ed89bfae46fd`で、dispatch ledgerはGeminiを`completed`として記録し、Gemini continuityの
    `lastSyncedMessageId`だけがsource messageへ進んだ。opaque session identifierは記録していない。
  - Codex replyは作成されず、dispatch ledgerはCodexを`externalStarted`のまま保持した。画面はCodexだけについて
    `The message may have reached Codex. It was not retried to prevent a duplicate turn.`を表示し、自動再送しなかった。
    Gemini reply後にcomposerは空の通常入力状態へ戻り、M.I.O.全体と両participant表示は応答を維持した。
  - 5秒超の安定確認後、snapshot、dispatch ledger、continuityのSHA-256はそれぞれ
    `DB8B61675D7C9F1E7367CB7E7658625110E135154B7B4DD0ED394C2DEE7480BA`、
    `EB156706D89DF9357F8A750897C39C806C9BEFC688FFB035F52A344FB17D378A`、
    `1926615D2CB2D8C24AD67761DDC8BCFDC2057C4DD4FB3769457F9162714E0B2C`で不変だった。
  - 既存のGemini unavailable + Codex成功、Claude Fable unavailable + Codex成功、Grok unavailable + Codex成功の
    mixed-recipient証拠と本試験を合わせ、releaseがclaimする4 Providerをそれぞれ個別に利用不能にしても、別recipientの
    成功結果を保持し、結果不明recipientだけを再送せず、M.I.O.全体が継続することを確認した。これによりGate 4の
    全Provider unavailable項目を完了とした。

Codex selected-workspace live-test evidence:

- Date: 2026-08-26
- Environment: Windows host、Codex CLI `0.149.0`、`windows.sandbox = "unelevated"`
- Notes:
  - ignored live testの既知markerをprompt本文から除き、実際に`input.txt`を読まなければ返せない一意値を
    `output.txt`へ複製・照合する形へ修正した。WindowsでApp Server終了直後にfixtureがロックされる場合に備え、
    cleanupも短時間再試行する形へ修正した。
  - 修正版`live_codex_workspace_reads_and_writes_only_the_selected_root`を実送信したところ、Codexは
    `Unable to complete: the workspace filesystem sandbox failed before allowing file access.`と返し、testは失敗した。
  - モデルを使わない`codex sandbox -P ":read-only"`は成功した一方、M.I.O.と同じrestricted read/write
    custom profileは`Restricted read-only access requires the elevated Windows sandbox backend`で拒否された。
    App Serverの`config/read`でも有効なWindows sandboxが`unelevated`であることを確認した。
  - nested junction live testは境界assertion前のfixture cleanupで失敗したため、境界結果としては採用しない。
    testはcontrol fileとproof fileで実tool使用を証明してからescapeを判定する形へ修正したが、再実送信は保留した。
  - `unelevated`では選択root内writeとroot外read拒否を同時に強制できない。root外readを許すprofileへ弱めず、
    elevated Windows sandbox setupまたは製品側fail-fastを決めるまでCodex workspace Gateは未完了とする。
  - 製品側fail-fastを追加した。WindowsでCodex workspace accessを選んだ場合、App Server初期化後かつ
    `thread/start`前に`config/read`を行い、`windows.sandbox = "elevated"`でなければモデルturnを開始せず
    `codexWorkspaceSandboxUnavailable`として確定失敗を返す。text-only Codex Room turnにはこのpreflightを適用しない。
  - dispatch ledgerには、adapterがモデル未送信を保証したworkspace preflight失敗だけを
    `externalStarted`から`failed`へ確定する専用遷移を追加した。timeoutなど通常の外部結果不明は従来どおり
    `externalStarted`を維持し、自動再送しない。
  - Room UIは`codexWorkspaceSandboxUnavailable`だけを専用表示し、elevated Windows sandboxが必要なこと、
    text-only送信は利用できること、保存済みmessageを自動再送しないことを案内する。他の確定失敗と
    結果不明warningは従来の表示契約を維持する。
  - OpenAI公式Windows sandbox文書で`elevated`が推奨modeであり、`config.toml`の
    `[windows] sandbox = "elevated"`で構成することを再確認した。READMEとAI access権限説明へ、
    Codex workspace read / writeの必要条件と、`unelevated`ではtext-onlyを維持する制約を明記した。
  - `workspace_preflight_requires_the_elevated_windows_sandbox`、
    `workspace_sandbox_preflight_failure_is_not_marked_unknown`、ledger terminal-state testを追加・更新した。
    `cargo test -p moe-desktop`は122 passed、0 failed、5 ignored、`cargo test --workspace`、
    `cargo +stable clippy -p moe-desktop --all-targets -- -D warnings`、`cargo fmt --all -- --check`、
    `npm.cmd run typecheck --workspace @moe/desktop`、`npm.cmd run build --workspace @moe/desktop`も成功した。
  - 続く実機診断では、通常のCodex設定を変更せず、一時overrideで`windows.sandbox = "elevated"`を指定した。
    model turnを開始しない`codex sandbox whoami`は専用のoffline sandbox userを返し、elevated backendが実際に
    起動できることを確認した。試験専用launcherはignored rootだけに置き、通常のM.I.O. dataと設定へは入れていない。
  - 同じelevated backendで`live_codex_workspace_reads_and_writes_only_the_selected_root`を独立した一時workspaceへ
    実送信した。最初の試行はroot内の一意な`input.txt`読取り、`output.txt`書込み、内容照合まで成功した後、試験専用
    launcherの子process終了待ちだけが3秒を超えてcleanup failureになった。launcherへWindows Job Objectによる
    child cleanupを追加して再試験し、1 passed、0 failedでfixture削除まで完了した。これは選択root内の通常read / write
    controlがelevated backendで成立する証拠とする。
  - 続いて`live_codex_workspace_blocks_nested_junction_escape`を1回だけ実送信した。選択workspace内の通常
    `control.txt`読取りは成立し、junction経由のroot外writeは作成されなかった一方、root外`secret.txt`のmarkerが
    model responseへ現れたため、testは`workspace permission followed a junction for read`で失敗した。同じmessageは
    再送していない。fixtureと試験launcher processはtest後に残っていない。
  - したがって`elevated` requirementとroot自身のreparse-point拒否だけでは、nested junctionを介したroot外readを
    防げない。ADR 0019の「選択rootだけをread / write可能」とADR 0028の未検証境界を満たさないため、Codex
    workspace Gateは未完了のままとする。公開契約をroot内writeだけへ弱める、再帰scanを無断で追加する、または
    failを再送で上書きすることはしない。
  - 承認された是正trancheでは、OpenAI公式のworkspace-only permission profile例に合わせ、M.I.O.が生成する
    Codexの全custom profileへ明示的な`:root = "deny"`を追加した。`:minimal`のreadと選択workspace rootのmode別
    read / writeだけを再許可し、network無効と再帰scanを追加しない設計は維持した。この変更は同じlive境界testが
    成功するまで候補修正であり、Gate完了の証拠にはしない。
  - permission profile構造testを追加し、`cargo test -p moe-desktop`は123 passed、0 failed、5 ignored、
    `cargo fmt --all -- --check`も成功した。承認済みの同じnested junction live testを1回だけ実行したが、今回は
    App Server turnが`InvalidResponse`で終わり境界assertionまで到達しなかったため、成功・失敗どちらの境界証拠にも
    採用せず、model messageを再送しなかった。fixtureと試験launcher processは残っていない。
  - 続いてmodel turnを開始しないApp Server `command/exec`で、同じ`:root = "deny"`、`:minimal = "read"`、
    workspace root `write`、network無効profileを隔離junction fixtureへ適用した。workspace内controlは読めたが、
    junction経由でroot外secret markerも読めた。root外writeは`AccessDenied`となり、実fileは作成されなかった。
    fixtureは削除済みである。したがって明示的root denyもWindows nested junction read境界を満たさない。
  - 承認済みfallbackとして、Windows alpha.1ではCodex workspace read / writeを無効化した。新規・legacy
    `providerDefault`はchat-only、profile UIのread / writeとRoom folder選択はdisabled表示とし、保存済みの明示
    workspace modeがadapterへ届いてもProvider process起動前に確定失敗する。README、ADR 0028、0034、0037、
    失敗案内もこの公開境界へ合わせた。実画面確認まではGateを完了にしない。
  - fallback実装後、`cargo test --workspace`は全crate成功し、`moe-desktop`は124 passed、0 failed、5 ignoredだった。
    `cargo +stable clippy -p moe-desktop --all-targets -- -D warnings`、`cargo fmt --all -- --check`、
    `npm.cmd run typecheck --workspace @moe/desktop`、`npm.cmd run build --workspace @moe/desktop`も成功した。
  - 変更を含むdebug buildを通常Windowsで起動し、Room設定にalpha.1の安全確認未完了案内が表示され、
    `フォルダーを選ぶ`がdisabledであることを実画面確認した。Codexの参加者プロフィールでは`会話のみ`が選択され、
    workspace read / writeの両項目がnested junction理由付きでdisabledだった。保存変更や外部AI送信は行っていない。

## 6. Public safety and privacy

- [x] Current snapshotへsecret scannerを実行した。
- [x] API key、token、cookie、private key、tunnel URL、local credentialを含まない。
- [x] 公開証跡からuser name、local path、session / thread / turn / item ID、user-agentを除いた。
- [x] Private review、handoff、個人会話、未承認画像を公開snapshotへ含めていない。
- [x] Runtime Room history、profile、continuity、ledger、backupをartifactへ含めていない。
- [x] M.I.O. local MCPはtoken未設定時に起動せず、loopback以外へbindしない。
- [x] Relayは完全な明示設定なしに起動せず、HTTP endpointを受け入れない。
- [x] Providerへの外部送信、契約、課金、retentionをREADMEとUIで説明した。
- [x] `SECURITY.md`のprivate vulnerability reporting導線を実際に確認した。

Evidence:

- Date: 2026-08-21
- Commit: `0d0cf60` + 未commitの文書匿名化候補
- Scanner / version: Gitleaks v8.30.1 + 追跡済みファイルへの補助pattern scan
- Notes:
  - GitHub公式release asset `gitleaks_8.30.1_windows_x64.zip`のSHA-256
    `D29144DEFF3A68AA93CED33DDDF84B7FDC26070ADD4AA0F4513094C8332AFC4E`を検証した。
    追跡済みcurrent snapshot約1.87 MBだけを`--redact=100`でscanし、0 findingsだった。
  - 2026-08-21の補助pattern scanでは、代表的なAPI key、token、private key、
    credential入りURL、tunnel URL、個人呼称、実ユーザー名入りpath、local workspace絶対pathを
    0件と記録した。ただし旧個人表示名をpatternへ含めておらず、後日の再検査で判定漏れと確認した。
  - 追跡済み文書10ファイルを匿名化し、個人呼称、旧内部owner ID、実ユーザー名入りpath、
    local workspace絶対pathを0件にした。
  - `spikes/**/evidence/*.json`を構造監査し、session / thread / turn / item ID、
    author / display name、server user-agentの対象fieldがすべて`<redacted>`であることを確認した。
  - 開発repositoryには追跡済みHANDOFFが4件残る。匿名化済みでもpublic snapshotからは
    除外する。`scripts/export-public-alpha1.ps1`の実測候補では、これら4件、旧個人用計画、
    旧V1完成判定の計6文書がfile listと展開先の両方に存在しないことを確認した。
  - 同じ実測候補は固定commitの追跡ファイルだけを`git archive`し、未追跡ファイル、`.git`、
    `.tools`、`.moe`、`artifacts`、`logs`、`target`、`node_modules`を含まない。
  - local MCPは`127.0.0.1:38474`固定、token未設定時disabled、Host / Origin制限あり。
    Relayは設定が全欠落ならdisabled、一部だけならerror、endpointはHTTPSのみを受理する。
    これらのunit testとWindows CIはGate 5で成功済み。
  - READMEはローカルCLIがローカル推論を意味しないことと、Providerのnetwork、利用規約、
    契約、課金、保存方針の対象になり得ることを説明している。
  - GitHub repositoryは現在privateでsecret scanningはdisabled。Private vulnerability
    reportingはpublic repository向け機能のため、公開導線の実確認はpublic candidate側で行う。
  - 2026-08-26に`blackcometclub/M.I.O`をGitHub APIで再確認した。repositoryは引き続き`private`で、
    repository情報の`security_and_analysis.private_vulnerability_reporting`は未提示だった。専用の
    private vulnerability reporting endpointをGETしても404となり、設定変更は行っていない。
    `SECURITY.md`は公開Issueへ秘密情報を書かずSecurity tabから非公開報告するよう案内しているが、実際の
    reporting formはvisibility変更後に確認する必要があるため、checkboxは未完了のままとする。
  - 2026-08-27にrepositoryをpublicへ変更した後、Private vulnerability reporting専用APIを有効化した。
    同APIの読み戻しはHTTP 200と`{"enabled":true}`を返した。公開repository本体と
    `https://github.com/blackcometclub/M.I.O/security/policy`はいずれも匿名アクセスでHTTP 200となり、
    `SECURITY.md`のSecurity tab経由の非公開報告案内と実設定が一致した。

Correction evidence:

- Date: 2026-08-22
- Commit: `6d77f58` + 未commitのtest fixture匿名化差分
- Scanner / version: Gitleaks v8.30.1 + 追跡済みworktreeへの補助pattern scan
- Notes:
  - 前回候補`705c4879daa1b1687f8e9883a400696cede86f5c`と現行HEADの双方で、
    旧個人表示名がRust test fixture 4ファイル8箇所に含まれていたことを確認した。
  - fixtureを`Sample Owner`へ置換し、追跡済みworktreeの個人呼称scanを0件にした。
  - `moe-desktop`は118 passed / 5 ignored、`moe-mcp`は8 passed、失敗0だった。
  - 既存候補へ匿名化差分4ファイルだけを反映したignored pre-commit snapshot 306ファイルは、
    Gitleaks 0 findings、個人呼称、local workspace絶対path、credential入りURLが各0件だった。
  - commit固定候補の再生成はfixture匿名化差分のcommit承認後に行う。

## 7. License, assets, and documentation

- [x] AGPL-3.0-onlyと商用ライセンス方針を公開文書で正しく説明した。
- [x] Third-party dependencyとassetのlicenseを確認した。
- [x] Pixelify Sans由来outlineとSIL OFL 1.1 noticeを同梱した。
- [x] App icon、logo、sample imageの公開権利を確認した。
- [x] 公開用screenshotの公開権利を確認した。
- [x] READMEにWindows要件、build、起動、Provider前提、制約を記載した。
- [x] Source-first alphaであり、installer、署名、自動更新、SLAがないことを明示した。
- [x] Changelogまたはrelease notesを作成した。

Evidence:

- Date: 2026-08-21
- Commit: `0d0cf60` + 未commitのlicense監査候補
- Notes:
  - `apps/desktop/src/assets/mio-logo.svg`の由来commentとoutlineを確認。
  - `apps/desktop/src/assets/licenses/Pixelify-Sans-OFL.txt`はGoogle Fonts公式配布と
    同じcopyright headerおよびSIL OFL 1.1全文93行を保持。
  - JavaScript外部dependencyはlockfile上のlicense identifierを集計済み。
  - 2026-08-21にJavaScript外部entry 180件を再集計した。license fieldがない5件は
    すべてrepository自身のnpm workspaceだった。
  - 全targetのRust外部crate 541件をoffline metadataから集計し、license identifier
    またはlicense fileがないcrateは0件だった。詳細は`THIRD-PARTY-NOTICES.md`へ記録した。
  - App iconはrepository内の幾何学SVG、vision PNGは同梱script生成、seeded SVGは
    repository内の文字と単純図形だけであることを確認した。
  - README、LICENSE、COMMERCIAL-LICENSE、CHANGELOGのalpha.1公開境界を照合した。
  - Ownerは2026-08-21にApp icon、生成済みplatform icon、M.I.O. logo、vision PNG、
    seeded bug SVGをsource-first公開snapshotへ含めることを承認した。
  - 2026-08-22にOwnerは、設定済みartworkを含む公開用screenshot 4枚について公開掲載できる
    権利を持ち、GitHubの公開READMEへ掲載してよいことを明示確認した。

## 8. Public repository and release authorization

- [x] Private development historyをpublicへ直接公開しない方式を確認した。
- [x] 検査済みcurrent snapshotだけでpublic candidateを作成した。
- [x] Public candidateのfile listとsecret scanを再確認した。
- [x] 個人ラベルfixture修正後のcommitでpublic candidateを再生成し、補助scan 0件を確認した。
- [x] Repository名、description、topics、default branch、Issue、Security設定を確認した。
- [x] Screenshot専用Room、架空名、英語会話で公開画像を撮影した。
- [x] Screenshotへ個人名、既存履歴、local path、credential、内部errorを写していない。
- [x] Ownerがrepository作成またはvisibility変更を明示承認した。
- [x] Ownerがtag作成とGitHub Release公開を明示承認した。

Evidence:

- Date: 2026-08-21
- Source commit: `705c4879daa1b1687f8e9883a400696cede86f5c`
- Method: `scripts/export-public-alpha1.ps1`による履歴なし`git archive`
- Notes:
  - Ignored local review area `.tools/public-alpha1/705c4879daa1/`へ、source ZIP、展開済みsource、
    machine-readable manifest、file listを生成した。公開repository作成、visibility変更、tag、
    GitHub Release作成は行っていない。
  - 期待file list 305件と展開済みfile 305件が完全一致した。
  - Source ZIP SHA-256:
    `9AA2A4E5FBF3479002F5A73DB5C82F4A825B76A5CC10081CF08A6C633E4C285F`
  - Public candidate約1.78 MBをGitleaks v8.30.1の`--redact=100`でscanし、0 findingsだった。
    実ユーザー名入りpathとlocal workspace絶対pathは0件だった。旧個人表示名を
    patternへ含めておらず0件と誤判定したため、下記Correction evidenceで訂正する。
  - 同じsource commitのGitHub Actions Windows CI run `32440289991`は8分54秒で成功し、
    TypeScript、frontend build、evidence sanitization、Rust format、Rust workspace testがPASSした。
  - Exporter自身とこの記録を含むcommitの作成後、同じcommitからcandidateを再生成する。
    自己参照でcommitを変えないよう、再生成後のfile list、hash、secret scanはignored manifestと
    release承認時の外部証跡へ記録する。ほかの必須項目が残るためrelease commit自体はまだ固定しない。

Correction evidence:

- Date: 2026-08-22
- Source: `6d77f58` + 未commitのtest fixture匿名化差分
- Method: 既存の履歴なし候補へ承認済み4ファイルだけを反映したpre-commit再検査
- Notes:
  - `.tools/public-alpha1/worktree-owner-anonymization/source/`のfile数は306件だった。
  - Gitleaks v8.30.1は約1.80 MBをscanし、0 findingsだった。
  - 個人呼称、local workspace絶対path、credential入りURLの補助scanは各0件だった。
  - TryCloudflareの一致は実URLではなく、技術説明、検証コード、匿名化済みevidenceの
    provider名だった。evidenceは公開URLと秘密pathを保存していない。
  - commit固定候補ではないため、fixture匿名化差分をcommitした後にexporterで再生成する。

Regenerated candidate evidence:

- Date: 2026-08-22
- Source commit: `429f2c0114c132b51553d2dfd232b71116178e12`
- Method: `scripts/export-public-alpha1.ps1`による履歴なし`git archive`
- CI URL: https://github.com/blackcometclub/M.I.O/actions/runs/32568384394
- Notes:
  - commit済みcurrent snapshotから310ファイルを生成し、Git treeの期待file listと展開済み
    sourceの実file listが完全一致した。公開用screenshot 4枚を含む。
  - Source ZIP SHA-256は
    `44A9D33FB9EBBE6B10E76B88CB5BA47AA127EEB89DFF46FBC8BB1073FA4B8129`だった。
  - Gitleaks v8.30.1は展開済みsource約1.80 MBを`--redact=100`でscanし、0 findingsだった。
  - 匿名化前の個人表示名、実local workspace path、credential入りURL、個人呼称の補助scanは
    各0件だった。公開file list内のHANDOFF文書も0件だった。
  - Manifestはhistoryとuntracked filesを含まないことを記録し、protectedな未追跡Fable / handoff
    文書はcandidateへ含まれていない。
  - GitHub Actions Windows CI run `32568384394`は9分13秒で成功し、dependency導入、typecheck、
    frontend build、evidence sanitization、Rust format、Rust workspace testを含む全stepがPASSした。

Screenshot evidence:

- Date: 2026-08-22
- Base commit: `6e7f02796365d9e243a4b4ebb5b93f515c5da7bb` + 未commitのscreenshot / 文書差分
- Notes:
  - ignoredの撮影専用runtimeで英語Room `Midnight Tinfoil Club`を作成し、架空のOwner
    `Alex Rivera`とCodex、Claude Web、Geminiによる陰謀論風の架空会話を表示した。
  - AI messageには`UI DEMO`を表示し、実際のProviderへ送信せず、撮影専用のlocal dataだけを使った。
  - Main Talk Room、Preferences、appearance、Room settingsの4枚を
    `docs/assets/screenshots/`へ追加した。
  - 4枚を目視確認し、実個人名、既存Room履歴、local path、credential、内部errorが
    表示されていないことを確認した。
  - Ownerは設定済みartworkを含む4枚の公開掲載権利を明示確認した。
  - SHA-256:
    - `mio-talk-room.png`: `36778BB3E272CAE4F9A7B9FF5AC3425D96F937FEA596D286FAB775725ACD85A7`
    - `mio-preferences.png`: `49ACCC5DEEE6F594BF182F6471E07D9DE8D2AFF177140B16ED6F3E2DD2053EA5`
    - `mio-appearance.png`: `9A6379FCC609B5921A03B84DC49C46376A27934BE6BBAC9BAAD062D9D54D640E`
    - `mio-room-settings.png`: `E7C6B98EC87BF2113DD3E853910B0B58F6265CCBF22A53A5C3AB159E81BBA379`

Repository settings evidence:

- Date: 2026-08-22
- Commit: `3a32d49`
- Method: authenticated GitHub API and GitHub Actions result
- Notes:
  - Repositoryは`blackcometclub/M.I.O`、visibilityはprivate、default branchは`main`だった。
  - Description、9 topics、Issuesが設定済みであることを確認した。
  - Vulnerability alertsは有効、Automated security fixesは有効かつpausedではなかった。
  - Open Dependabot alertは`Cargo.lock`の`glib 0.18.5`に対するmedium 1件だった。Owner方針どおり
    Windows targetに含まれないLinux限定経路としてopenのまま維持し、dismissしていない。
  - Dependabot Updates run `32548869147`は`security_update_not_possible`で終了した。現在解決可能な
    versionは`0.18.5`、最初の非脆弱versionは`0.20.0`であり、通常CIのfailureとは分離した。
  - 2026-08-26にalert #1を再確認し、openのmedium `GHSA-wrw7-89jp-8q8g`だけであることを確認した。
    `cargo tree --target x86_64-pc-windows-msvc -i glib@0.18.5`は依存なし、Linux targetでは
    TauriのGTK / WebKit経路に存在した。最新Tauri `2.11.5`もGTK `0.18`を使用し、上流のGTK4移行
    Issue `tauri-apps/tauri#12561`はopenであるため、互換性を壊さない修正版は現在存在しない。
  - Windows CIへdependency boundary checkを追加した。`cargo tree`でWindows targetに`glib`が1件でも
    入った場合はCIを失敗させ、Linux限定というrisk acceptanceが将来無効になったことを検出する。
    alert自体は上流修正を追跡できるようopenのまま維持する。
  - Private vulnerability reporting APIはprivate repositoryに対して404だったため、Gate 6の
    実導線確認は未完了のまま維持した。
  - Visibility変更、公開用screenshot、tag、GitHub Releaseは実施していない。
  - 2026-08-27にOwnerは`blackcometclub/M.I.O`のvisibilityをprivateからpublicへ変更し、
    Private vulnerability reportingを有効化・確認する範囲を明示承認した。変更後のGitHub APIは
    `visibility: public`、`private: false`を返し、公開URLへの匿名アクセスもHTTP 200だった。
    TagとGitHub Releaseはこの承認範囲に含めず、未実施のまま維持した。
  - 同日、Ownerはtag `v0.1.0-alpha.1`とGitHub Prereleaseの公開内容、対象commit、asset、
    SHA-256を確認して明示承認した。Tagはrelease candidate
    `35479df0871003e37834099955095bda5e98a3e3`を指し、ReleaseはdraftではないPrereleaseとして
    `https://github.com/blackcometclub/M.I.O/releases/tag/v0.1.0-alpha.1`へ公開した。
  - 添付した`mio-v0.1.0-alpha.1-source.zip`は4,637,195 bytesで、GitHub asset APIのdigestは
    `sha256:d4f8e0267626c4a1a3a3ffcb1a30d313cd99a20fda0eb04258431b6cf9d1568d`だった。
    Release URLは匿名アクセスでHTTP 200を返した。Installerや実行binaryは添付していない。

Post-publication boundary correction:

- Date: 2026-08-27
- Notes:
  - 公開用custom source ZIPは検査済み310ファイルだけを含み、Gitleaks 0 findingsだった。一方、
    development repository自体のvisibilityをpublicへ変更したため、120 commitの履歴とexporterが
    除外する開発計画／HANDOFF 6ファイルがrepository本体から参照可能になった。
  - GitHubがtagから自動生成するSource code ZIPにも除外対象6ファイルが含まれること、代表文書の
    raw URLが匿名アクセスでHTTP 200となることを確認した。これはcredential漏えいではないが、
    「検査済みcurrent snapshotだけを公開する」という承認済み境界に違反するため、GOを撤回した。
  - 公開cloneの全120 commitをGitleaks v8.30.1でscanし、約2.29 MB、0 findingsだった。
    Release assetの確認時download count、repositoryのstar、forkはいずれも0だった。
  - Owner承認後、`blackcometclub/M.I.O`をprivateへ戻した。repository本体とRelease URLは
    未認証アクセスでHTTP 404となり、新規の匿名アクセスが止まったことを確認した。
  - 修復ではprivate development repositoryを`M.I.O-dev`として保持し、除外済みsnapshotだけを
    1 commitの新しい`M.I.O` repositoryへ公開する。既存開発履歴のforce rewriteは行わない。

Remediated repository evidence:

- Date: 2026-08-27
- Private development repository: `blackcometclub/M.I.O-dev`
- Public candidate repository: `blackcometclub/M.I.O`（検査時点ではprivate）
- Snapshot root commit: `74d0a441cdeab7a65da239c3fee7d58d462db212`
- CI URL: https://github.com/blackcometclub/M.I.O/actions/runs/33037366785
- Notes:
  - Private development repositoryを`M.I.O-dev`へ改名し、local originも同じprivate URLへ固定した。
    旧120 commit、private tag / Release、追跡外Fable / HANDOFFはprivate側だけに保持している。
  - `8b06159e9061e66d31925f3c9ff69338b43b641f`からexporterで310ファイルを再生成した。
    開発計画、旧readiness、HANDOFFの除外対象は0件、historyとuntracked fileも含まない。
  - 新しい`M.I.O`はsnapshot root 1 commitから開始した。Root authorはGitHub noreply identityを使用し、
    tracked file 310件、除外対象0件、Gitleaks v8.30.1は1 commit・約1.95 MBで0 findingsだった。
  - GitHub Actions Windows CIは10分19秒で成功し、dependency導入、typecheck、frontend build、
    evidence sanitization、Rust format、Windows dependency boundary、Rust workspace testを含む
    全stepがPASSした。
  - GitHub側でもcommit数1、blob 310件を確認した。Description、9 topics、Issues、Dependabot
    vulnerability alerts、automated security fixesを設定した。Publicへ変更する前に検査を完了した。
  - 最終readiness記録はsnapshot後のsanitized documentation commitとしてpublic historyへ追加する。
    Release tagをそのcommitへ固定し、GitHub自動生成Source codeにもsanitized historyだけが入ることを
    public変更前後に再確認する。

## Final decision

- [x] **GO:** 必須項目が完了し、未完了項目はalpha.1の公開範囲外である。
- [ ] **NO-GO:** 必須項目に未解決の失敗がある。公開せず、原因と次の確認を記録する。

- Decision date: 2026-08-27
- Release commit: tag `v0.1.0-alpha.1`が指すsanitized public repository HEAD
- Decision maker: Owner
- Remaining deferred items: installer、code signing、自動更新、安定版SLA、Windows以外の対応保証、
  workspace access、公開Remote Relay、複数device / account、各experimental feature。
