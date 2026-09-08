# M.I.O. V1 readiness checklist

- Status: Working draft
- Date: 2026-09-04
- Last verified: 2026-09-08（Store `1.0.4.0`公開・実機更新、直接配布 `1.0.4`）
- Target: Windows x64向けの一般公開安定版 `v1.0.0`
- Starting point: `v0.1.0-alpha.2`
- Rule: 未確認項目を推測で完了にしない

## 2026-09-08の公開後確認

Store Submission 4／`1.0.4.0`の公開と、Owner承認済みのStore実機更新・起動・通常終了・再起動を
確認した。登録状態は`SignatureKind: Store`／`Status: Ok`で、実行中の本体も`1.0.4`だった。
Room表示の差は通常版とStore版のAppData仮想化による分離であり、既存dataは両側に残っていた。
Ownerの方針は「Store側の現状を使い、履歴の復元・移行は不要」。backupを保持し、data変更は行っていない。

BOOTH／itch.ioの公開downloadは`1.0.4`を確認した。同日の後続作業で、公式紹介サイトの日英ページを
Store `1.0.4.0`／直接配布`1.0.4`へ更新し、3媒体の`1.0.4`更新履歴も揃えて公開した。
GitHubの`v1.0.4` Releaseは未作成。
clean環境試験や最小smokeの未確認項目は継続し、公開後チェック全体は**PARTIAL**とする。
証拠、backup path、確認範囲は[Store公開後チェックリスト](MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md#判定)を参照。

Ownerの2026-09-08追加依頼により、glib警告の確認後、TINMOONサイト、BOOTH、itch.ioの3ページへ
`v1.0.4`の更新履歴7項目とStore公開済み案内を揃えた。`v1.0.3`の既存履歴は保持した。
サイトcommit `61af285d69bd72d71977309a26ca651b1a249e04`は`TINMOON-Label/TINMOON-Label-Site`の
`main`へpush済み。Astro build（23ページ）、Cloudflare Pages check成功、日英の公開HTTP 200と
版数・新旧履歴の存在を確認した。BOOTH／itch.ioも非login browserの公開本文で反映を確認した。

## V1の完成条件

V1は、alpha.2で安全性を確認した会話機能をWindows利用者がinstallerから導入し、
日常的に使い、既存データを保ったまま更新・削除できる最初の安定版とする。

V1で接続先や自動化機能を際限なく増やす必要はない。対応範囲を明示し、対応済みの
経路が再現可能で、安全に失敗し、配布物を検証できることを優先する。

V1の中核は、Room単位の安全なworkspace操作、Providerごとのmodel選択、Direct／Conductor
による複数AI協働とする。Remote Relayと複数deviceはV1へ含めない。

## 現在までに完了している土台

- [x] Room作成、名前変更、参加AI管理、message履歴、backup／restore
- [x] Direct modeと、1 round・最大3 workerのConductor mode
- [x] 結果不明時の自動再送防止、dispatch ledger、single-instance
- [x] AI応答待ちの送信ボタンを停止ボタンへ切り替え、Direct／Conductorのactive turnへ
      provider-neutralな停止信号を渡す。送信済みmessageは残し、停止結果は自動再送しない
- [x] Codex、Gemini Antigravity、Claude CLI、Grok CLIの会話専用adapter
- [x] CLIなし、認証切れ、timeout、partial／unknown outcomeの安全な表示
- [x] workspace read／writeをProvider起動前に拒否するalpha.2境界
- [x] local MCPのloopback限定、token認証、bounded error
- [x] Windows上のsource build、CI、secret scan、公開snapshot作成
- [x] 英語README、日本語README、license、security／contribution窓口
- [x] current-user alpha installerのbuild、interactive install、初回起動、quiet uninstall
- [x] ADR 0044のper-machine V1 installerをbuildし、UAC、Windows設定への表示、起動、uninstallを確認
- [x] installer試験後の既存ユーザーデータのbyte-identical復元
- [x] privateからpublicへの段階的な公開準備・draft公開script

## V1リリースを止める必須項目

### 1. V1の範囲と名称を固定する

- [x] V1をWindows x64・workspace協働中心の安定版としてADR 0043で固定する
- [x] 対応Providerと保証範囲をADR 0043で固定する。実accountのrelease gateを通った経路だけを
      公開文書で対応済みと説明する
- [x] Room単位のworkspace read／writeをV1必須範囲としてADR 0038／0043で固定する。writeは
      Windows境界を個別に証明したCodexだけに許可する
- [x] Providerごとのmodel選択をV1必須範囲としてADR 0039で固定する
- [x] Remote Relay、複数device、複数account、macOS、LinuxをADR 0043でV1対象外として固定する
- [x] `Claude Code`を製品上の正式な接続名、`Fable 5`／`Opus 5`／`Sonnet 5`をmodel名として分ける。既存の端末内
      participant表示名は上書きせず、内部participant ID `claude-code`も維持する（ADR 0040）
- [x] installer用Windows製品名を`M.I.O`へ分離し、installer名`M.I.O_<version>_x64-setup.exe`、
      shortcut名`M.I.O.lnk`に修正する。画面上のブランド表記`M.I.O.`は維持する（ADR 0041）
- [x] V1に含めない機能がUIで選択不可または明確な説明付きになっていることを再確認する
  - 2026-08-31にsourceとWindows製品画面を再確認した。Remote Relay／複数device／複数accountの
    有効化設定は公開していない。Claude Webは`Web接続待ち`と接続設定が必要である旨を表示する
  - Provider未対応AIのworkspace権限は選択不可で`このAIは未対応`と表示し、file／command／Webは
    すべて`禁止`と表示する。Codexだけが選択folderのread／writeと限定commandを選択でき、Webは
    `禁止`、commandは`限定操作・一部は確認`と表示する

### 2. Room workspaceを安全に完成させる

- [x] 外部AIを使わないAppContainer process隔離spikeを実施する
- [x] 一時workspace内のread／writeが成功することを確認する
- [x] workspace外への直接read／writeをOS境界で拒否する
- [x] nested junction経由のworkspace外read／writeをOS境界で拒否する
- [x] sandbox内からjunction／symlinkを作成して境界を迂回できないことを確認する
- [x] 通常開発のbaseline grantと、外部送信・download・credential・権限拡大の
      action-time confirmation境界をADR 0038で固定する
- [x] shell文字列や任意引数を受けないprovider-neutral command分類contractを独立crateで固定し、
      Git調査系、標準build／test、action-time対象、read-only拒否を単体試験する
- [x] V1で公開する実行経路をhost内libgit2のGit status、固定`mio-main.mjs`のNode実行、固定npm
      `build`／`test`／`typecheck`、確認付きのexact npm package導入へ限定する。Cargo、shell文字列、
      任意npm script／引数、Git変更commandは製品toolへ公開しない
- [x] 一時Git fixtureで固定`git status`をshell文字列、任意argument、credential、継承環境なしで実行し、
      同じAppContainerのworkspace外／nested junction拒否とOS-level network拒否を確認する
- [x] AppContainer childをsuspendedでJob Objectへ収容してから開始し、bounded stdout／stderrとtimeoutの
      実process fixtureを確認する
- [x] 固定Git process実行をWindows内部crateへ切り出し、AppContainer tokenとJob Object所属を再確認してから
      環境非継承、出力上限、timeout付きで起動する経路を同じ隔離fixtureで確認する
- [x] desktop hostから隔離helperへ渡す固定命令を16 KiB以下のversion付きrequestへ限定し、workspace絶対path、
      executable、shell文字列、任意argumentを含めずhelper側で再分類するprotocol contractを固定する
- [x] helper requestをworkspace fileやcommand lineへ載せず、hostだけがwrite端を持つ一回限りの匿名pipeで渡し、
      AppContainer childへread handle一本だけを明示継承するfixtureを確認する
- [x] 固定requestをstdin pipeだけから受け、brokerで再分類してからWindows runnerを呼ぶ専用helper binaryを作り、
      同じAppContainer／Job Object内の別processとして固定`git status`を実行するfixtureを確認する
- [x] installer build時のhelper SHA-256をdesktop EXEへ埋め込み、同じinstall directoryの正規名、非reparse、
      単一hard-link、一致hashのhelperだけを起動候補とする。欠落、置換、hash不一致はprocess起動前にfail closedする
- [x] activeなhost-owned Room sessionと現在のworkspace identityを再照合し、検証済みhelperがある場合だけ、
      列挙済みGit baselineを固定helper requestへ変換するdesktop内部準備契約を実装する。processとtool公開は行わない
- [x] 準備済みdesktop commandの実行直前にRoom session、workspace identity、helper、Gitを再検査し、差し替え時は
      AppContainer processを作らずfail closedする内部実行接続を追加する。Tauri commandとCodex toolには公開しない
- [x] desktop内部準備から実helper／固定Gitまでを通すsandbox外fixtureを実施し、temporary resource残存0件を確認する
- [x] Codex turnへhost-owned Room IDをpromptやtool引数とは別の内部fieldで渡し、将来のcommand toolが別Roomを
      モデル指定できない土台を作る。現段階ではcommand contextとdynamic toolを接続しない
- [x] host-owned Room ID、保存済みworkspace、現在のRoom command session、participant accessから不透明な
      turn command contextを作り、準備と実行の両方で同一sessionを再検査する。同じRoom／workspaceでも
      session再開始後は古いcontextと準備済み命令を拒否する
- [x] Codexへ引数なし・workspace root固定の`mio_command.git_status`だけを公開する実験接続を作り、exact turn
      contextから隔離helperを実行する。任意command、argument、working directory、別turn／別namespaceは受け付けない
- [x] 実`D:\desktop\M.O.E`でhost内libgit2版`mio_command.git_status`が60.0 msで成功し、Git index SHA-256と
      workspace ACLが前後一致することを確認する。Git process、temporary ACL、hook、networkは使用しない
- [x] Git executableをPATHやProvider指定から解決せず、既知のGit for Windows配置、正規名、非reparse、
      単一hard-linkを満たすfileだけ内部候補にし、実行準備時にfile identityとSHA-256を再検査する
- [x] 検証済みworkspace、helper、Git、固定requestだけを受け、AppContainer profile、network capabilityなし、
      workspace限定ACL、一時drive、kill-on-close Job、bounded pipe、完全cleanupを必須化するhost launcher契約を分離する
- [x] 実`moe-command-helper.exe`をAppContainer childとしてsuspendedで作成し、Job収容後に開始して固定`git status`を
      完了するhost経路を一時fixtureで確認する。専用環境と全temporary resourceのcleanupも実機で確認する
- [x] Codex CLIをAppContainerで起動できるか、認証・通信なしの`--version`で互換性を確認する
- [x] Codex App Serverは専用runtime directoryから起動し、認証情報はCodex自身の保存領域へ任せてM.I.O.から
      読み取らない。Provider通信とbrokered workspace toolを分離し、workspace path／cwd／writable rootを渡さず、
      tool側のnative sandboxはread-only・network disabledに固定する
- [x] `今回だけ許可`、`このRoom sessionの間だけ許可`、`拒否`、dismiss、timeoutを扱う
      host-owned確認状態contractを実装し、完全一致scopeと一度だけのauthorization消費を単体試験する
- [x] hostが登録したpending requestだけをRoom別に安全な表示情報へ変換し、Ownerの回答だけを受ける
      desktop backend境界を実装する。UIからrequest、scope、timeoutは作成できない
- [x] desktop backend応答を厳密に検証するfrontend bridgeと、拒否を初期focusにしたhost-owned確認画面を
      実装する。Esc／closeはdismissとして回答し、pending中はRoom画面を操作不能にして待機を表示する
- [x] host内部だけが確認requestを登録して待機し、Ownerの回答、host-owned timeout、Room session終了で
      wakeupする境界を実装する。timeoutはhost内の120秒へ固定し、許可は完全一致scopeで一度だけ消費、
      拒否と終了時はfail closedする
- [x] Room workspaceをpath文字列だけで識別せず、Windowsではvolume serial numberとfile index、
      Unixではdevice IDとinodeからhost内部のworkspace keyを生成する。同じpathのfolderが置換された場合は
      keyが変わることを単体試験で固定する
- [x] command用Room session IDをdesktop host内部で生成してRoomとworkspace keyへ結び付け、同じRoomの
      session再開始または終了時は旧sessionの確認状態と待機中callerをfail closedで終了するcontractを固定する
- [x] activeなhost-owned Room sessionからだけ確認requestを作り、request IDと完全一致scopeをhost内部で
      生成する入口を固定する。request直前にもworkspace identityを再検査し、古いsessionや同じpathの
      folder置換からはpending requestもauthorizationも生成しない
- [x] frontendはactive Room IDだけをhostへ通知し、hostが保存済みworkspaceを再検査してcommand用Room
      sessionを開始する。Room切替、chat-only化、workspace不明時は旧sessionを先にfail closedで終了する
- [x] host-owned確認UIと待機中turnを接続し、実製品UIのexact npm導入でOwnerの許可後だけ実行されることを確認する。
      promptやRoom historyからrequest ID、scope、許可を作れない
- [x] 検証済みNode／npmのread／execute、1回限りのAppContainer data／TEMP／cache、request・stdout・stderr以外を
      継承しないchild process境界を実装する。Git statusは子processを使わないlibgit2境界へ分離する
- [x] 列挙済みbaseline planだけをGit／npm／Cargoの固定argument、環境非継承、stdin無効、出力上限、
      timeout、workspace／network／child process隔離必須条件へ変換するprovider-neutral runner contractを
      独立crateで固定する。OS隔離backendが全条件を証明するまでprocessは起動しない
- [x] exact npm package取得だけを製品のaction-time confirmationへ接続する。`git push`、credential利用、
      未登録tool、管理者操作、大量削除は製品toolへ登録せずprocess起動前に拒否する
- [x] Windows 11 x64ではfile操作をworkspace broker、Node／npm processをAppContainerで隔離する。
      Windows 10は実機証拠がないためADR 0043に従いV1で保証しない
- [x] Codex native sandbox単独ではjunction境界を満たさなかったため、M.I.O.がfile操作を仲介する
      workspace broker方式を採用する。Providerへhost pathや直接file toolを渡さない
- [x] brokerが絶対path、`..`、root／nested reparse point、複数名を持つhard link fileを拒否し、providerへhost pathを
      公開しないprovider-neutral path contractを独立crateと単体試験で固定する
- [x] `cap-std 4.0.3`のopen directory capabilityをbroker内に保持し、最大1 MiBのread、新規作成、
      同一directory内のtemporary fileを使う既存file置換を実装する
- [x] Windows nested junction越しのread／createがpath preflightとcapability I/Oの両方で拒否され、
      workspace外にfileを作らない回帰試験を固定する
- [x] Codex App Server `dynamicTools`／`item/tool/call`でbroker operationだけを往復し、Providerへ
      workspace host pathやworkspaceをcwd／writable rootとして渡さない接続contractを実装する
- [x] 実Codex turnでbroker read／create／再readが成功し、nested junction越しのread／createは拒否されて
      workspace外を変更しないことを確認する
- [x] `会話のみ`、`読取り`、`読取り・編集`をRoom／participant設定へ正直に表示する
      （実画面でCodexだけが読取り／読取り・編集を選択でき、Geminiでは両項目が無効かつ
      `このAIは未対応`と表示されることを確認。双方でcommand／Webは禁止と表示）
- [x] folder消失、root junction、broker初期化失敗をProvider起動前の確定失敗として
      fail closedし、unknown／自動再送へ送らない
- [x] Windows ACLによる権限不足を一時directoryの実fixtureで再現し、broker初期化が
      `WorkspaceUnavailable`としてProvider起動前に確定失敗することを確認する
- [x] Codexが選択workspaceの既存fileを読み、新規fileを作成して再読取りする代表操作を
      製品UI経路で確認する
- [x] Codexのdirectory listingを実Codex App Serverの製品adapter経路で確認する
- [x] Grokの最初のworkspace役割を読取り専用Git reviewへ限定する。M.I.O.のlibgit2境界が
      追跡済み差分とstatusだけを128 files／64 KiB以内で作り、未追跡file本文、workspace host path、
      file tool、編集、command、Web、MCPをGrokへ渡さない（ADR 0042）
- [x] 実accountと製品UIでGrok reviewが追跡済み差分を指摘でき、未追跡sentinel本文とhost pathを
      応答へ含めず、write選択が表示・hostの両方で拒否されることを確認する。2026-08-31の実Grok Build
      probeは推論前に利用残高切れのHTTP 402で拒否されたため再送しなかった
  - 2026-09-03にGrok Build CLIを署名有効な`0.2.77`から署名有効な`1.0.13 stable`へ更新し、OAuth再認証後の
    `grok models`で`grok-4.6`既定と`grok-4.5`を確認した。1.xで削除された`--no-memory`をargumentから外し、
    公式に残る`GROK_MEMORY=0`、tool-free agent、Web／subagent無効化を維持した。終了理由は旧`EndTurn`と
    新`end_turn`の両方を受理する
  - 同日の実account adapter試験で、一時Git fixtureの追跡済み1行変更だけを送り、Grokは`REVIEW_OK`を返した。
    未追跡sentinel本文とworkspace host pathは応答へ含まれず、修正版parserが完了responseを受理した。
    一時fixtureは削除済み。同じ修正を含むsource release本体
    `657D44F10C208B65DF6FFBBEB9C9590295AE08571A3AE6B457593D7A41CB4D03`を起動し、製品UIのDirect modeから
    Grokへ固定replyを1回送り、完全一致`GROK_1_0_13_OK`を1件だけ保存した。これは`Providerの既定`によるchat経路の
    確認である
  - 続いてヒツジさん所有の専用Git fixtureをRoom workspaceへ選び、Grokを`workspaceRead`、modelを`grok-4.6`にして
    製品UIのDirect modeからレビューを1回送った。Grokは`canRelease`の論理反転とtrue／false双方の誤動作を正しく指摘した。
    replyには未追跡sentinel本文とworkspace host pathが含まれず、dispatch ledgerは`completed`、Grok session
    `01a064e1-9534-7bb1-821c-3ed344a1d274`は`grok-4.6`のmodel call 1回と正常終了`end_turn`を記録した。
    先にsandbox所有fixtureで行った1回はOwner不一致をProvider起動前に拒否し、model callは発生しなかった。
    Grok profileのread-write選択肢が未対応表示で無効なことをOwnerが実画面で確認し、host側も
    `workspace_review_rejects_write_access_before_starting_grok`でProvider起動前に拒否することを再確認した
- [x] 代表的なbuild／test／typecheckを製品経路で確認する。2026-08-31に実helper、検証済みNode／npm、
      一時AppContainerを通して固定3 scriptを実行し、各生成fileを選択directoryから読戻した。全scriptで
      network capabilityなし、clean environment、null stdin、bounded output／timeout、Job containment、
      一時ACL／drive／profile cleanupを維持した
- [x] file／command dynamic toolを列挙済みnamespaceとschemaへ限定し、workspace外path、hard link、junction、
      inherited credential、任意network、MCPへ権限が広がらないことを自動試験と実Node／npm fixtureで確認する。
      npm導入だけは固定registryへの`internetClient`をOwner確認後に一時付与する
- [x] Fable、Geminiはchat-onlyを維持し、host profile検証でもworkspace read／writeを拒否する。Grokは
      ADR 0042のread-only Git reviewだけを許可し、write／commandはUIとhostの両方で拒否する

Model-free AppContainer evidence:

- Date: 2026-08-29
- Environment: Windows display version 25H2 / build `26200.9168`
- Probe: `moe-windows-appcontainer-workspace-spike`
- Probe SHA-256: `FE5B2662C475CED28A6A714063FFBA4756FC4F2CFC5CD0208832F1B7DABEF8A8`
- Result: `BOUNDARY=PASS`、`CHILD_PROCESS_COMPATIBILITY=PASS`
- Notes:
  - AppContainerは許可workspace内のread／writeと`cmd.exe` child writeに成功した。
  - workspace外の直接read／writeと、既存nested junction経由のread／writeを拒否した。
  - AppContainer内からworkspace外を指すjunctionの作成は拒否され、escape fileは作られなかった。
  - 一意なAppContainer profileはprobe終了時に削除した。fixtureはignored `.tools/`へ証拠として保存した。
  - 追加probe `8733C29051F0AB3BAEE891A7923C35F9964AEDCDD3630F962EADCA892F930FB7`では、
    `git` processは起動したが、`node`、`npm`、`cargo`はAppContainerから利用できず、
    `TOOLCHAIN_COMPATIBILITY=FAIL`だった。境界試験は同じrunでもPASSを維持した。
  - Codex、credential、network、toolchainへの最小read-only許可、Windows 10は未検証である。
  - targeted Codex probe `E4042B34653326FC3F81CAB9913524401656F42419CAD6977B5CB3CC4CE8FEA4`では、
    fixtureへcopyしたCodex CLI `0.149.0`の`--version`がAppContainer内でPASSした。同じrunの
    boundaryもPASSし、profileは削除済みである。`app-server`、認証、通信、tool実行は未検証である。
  - App Server initialize probe `0DC2763A7A78CD9F31C32482DC08F61E3029C9E5226B68E5D34A84BC836D2208`では、
    boundaryとCodex CLI `--version`はPASSしたが、workspaceとAppContainer専用profile folderの
    `canonicalize()`がともにaccess deniedとなり、`CODEX_HOME`正規化でinitialize前に停止した。
    volume ancestorへ追加ACLを与える方法は未採用であり、直接収容方式には互換性blockerが残る。
  - 専用profile storage probe `E34FB652F391B988B9A0AA5340BEDA3BEF8D44080E7513A6E86CED64F1391ABF`では、
    host側で`codex-home`とTEMPを事前作成し固有SIDへmodifyを付与しても`canonicalize()`はaccess denied
    のままだった。boundary、generic child、Codex `--version`はPASSし、一時ACEとprofileは終了時に
    削除済みである。
  - volume rootからの通過権追加は`D:\`で管理者権限を必要とした。通常のRoom開始でUACやvolume root
    ACL変更を要求する方式はV1候補にしない。CodexはApp Server自身のWindows native sandboxと
    `workspaceWrite` contractをM.I.O.から指定する経路を次に検証する。
  - Codex CLI `0.149.0`のnative sandbox follow-upでは、現在の設定は`unelevated`だった。App Server
    schemaに合わせてthread=`workspace-write`、turn=`workspaceWrite`、writable root=選択workspace、
    network=falseを明示すると通常のworkspace read／writeはPASSしたが、nested junction経由の
    workspace外writeが成功した。fixtureは削除済みである。`unelevated`はV1 fallbackにせず、製品gateを
    OFFのまま`elevated`で同じ試験を再実行する。
  - 公式App Serverの`windowsSandbox/setupStart(mode=elevated)`は成功し、Codex Doctorでも
    backend=`elevated`、provisioning=`complete`を確認した。通常workspace read／writeは再度PASSしたが、
    同じ明示contractのnested junction試験ではworkspace外writeが成功した。比較したbetaの名前付き
    permission profileはworkspace外writeを拒否した一方、workspace外readを許した。両fixtureは削除済みで
    あり、Codex native sandbox経路はどちらもV1境界不合格、製品gateはOFFのままとする。
  - brokered dynamic tools経路では、workspace host pathをCodexへ渡さず、native sandboxをread-only、
    networkを無効のまま、M.I.O. hostがUTF-8 text fileのread／create／replaceだけを処理する。ACL拒否を
    Provider起動前の`WorkspaceUnavailable`として固定した後、このCodex限定経路をsource上の製品gateへ
    接続した。folder選択とaccess modeの実画面確認に加え、2026-08-29の製品UI試験で、選択workspaceの
    `seed.txt`読取り、`result.txt`新規作成、再読取りが成功し、host側でも内容の完全一致を確認した。
    directory listingも製品adapter経路で確認済みである。代表的なbuild／test commandは未完了である。
  - 2026-08-30のfixed Git baseline probe
    `705FA57ED8D43AC3053DC4D5432294DF80DDBE091B26FDAEF764F7BC8F0D6013`では、command brokerが分類し、
    runner contractが生成したGit status planをAppContainer child processへ接続した。一時Git fixtureを未使用
    drive letterへrun中だけ割り当て、固定`git status`、workspace境界、localhost接続拒否がPASSした。mapping、
    固有SIDのACE、AppContainer profileは終了時に削除済みである。通常のuser profileやvolume rootへACLを追加して
    いない。同じrunでJob Object収容、bounded output、timeout fixtureもPASSし、試験processが残っていないことを
    確認した。npm／Cargo、実Room workspace、製品runner、Windows 10は未検証のため、command gateはOFFのままである。
  - reusable Windows runner follow-up
    `A181EA9194925B36EB7EED9FEE4C1DFF41225D39E608595D856E82780A5C1E78`では、固定Git processの実行部分を
    `moe-windows-command-runner`へ切り出し、AppContainer tokenとJob Object所属を実行直前に再確認した。
    固定Git status、workspace境界、network拒否、出力上限、timeoutはすべてPASSし、temporary drive、固有SIDのACE、
    AppContainer profileも削除済みである。外側のAppContainer host launcherはまだspikeにあり、desktop製品経路、
    npm／Cargo、実Room workspace、Windows 10は未検証のため、command gateはOFFのままである。
  - bounded helper request follow-up
    `2A40C736BC9256477AC6AB9BE8DC5C26E7CBDEB537A4D4D2C899F61ACF94FA36`では、固定command ID、相対working
    directory、access区分だけのversion付きrequestをAppContainer childが匿名pipeから上限付きで読み、brokerで
    再分類してからWindows runnerへ渡した。hostだけがwrite端を持ち、childへはhandle listでread端一本だけを継承した。
    固定Gitと既存の全境界項目はPASSした。専用helper binaryとdesktop接続前なのでcommand gateはOFFのままである。
  - dedicated command helper process follow-up
    helper `0E25A57AA907F5C4357065AFBF06C178AD3F925BFC9A21ACCC470299D0C33B3B`とprobe
    `78AAB556C82E0314B9D8985C968C30CD1319E217D6AD37F54FCB099A2FF5EECC`では、固定requestをstdin pipeだけから受け、
    brokerで再分類してWindows runnerを呼ぶ独立`moe-command-helper.exe`を、AppContainer／Job Object内の別processとして
    実行した。固定Git status、workspace境界、network拒否、出力上限、timeoutと後始末はPASSした。desktop製品経路、
    npm／Cargo、実Room workspace、Windows 10は未検証であり、command gateはOFFのままである。
  - desktop bundled-helper identity follow-up
    Windows x64 NSIS検証installer `FC106C4AA93172F5A4F7719CA24C462BBEA0621F4DA696741E469A9B5F2504D6`では、
    build元とinstaller内helperのSHA-256が
    `8C0BEEF811CA2C7773C4F70E73B2893578E336C7F236B8BE0FF655C2213F62E3`で一致し、同じ値がdesktop EXEへ
    build時に埋め込まれた。desktop起動時はhelperを起動せず、同一directory、正規名、非reparse、単一hard-link、
    一致hashを満たす場合だけ内部状態をReadyとする。Codexへのcommand tool公開とcommand gateはOFFのままである。
  - desktop command preparation follow-up
    desktop内部の準備契約は、activeなhost-owned Room session、再取得したworkspace identity、検証済みhelperを照合し、
    brokerが許可した固定Git baselineだけをpathやshell文字列を含まないhelper requestへ変換する。inactive Room、
    helper欠落／置換、read-onlyでのbuild、未対応npm／Cargoはprocess起動前に拒否する。backendは未接続で、
    Codexへのcommand tool公開とcommand gateはOFFのままである。
  - desktop Git executable identity follow-up
    desktop hostは`ProgramW6432`／`ProgramFiles`／user-local Programs配下の正規`Git\\bin\\git.exe`だけを候補にし、
    PATH、Provider、frontendから実行fileを指定させない。install root、Git directory、bin directory、実行fileのreparseを
    拒否し、単一hard-link、file identity、起動時SHA-256を準備時に再検査する。processはまだ起動せず、command gateは
    OFFのままである。
  - Windows command host launcher contract follow-up
    独立crateが、検証済みの絶対workspace、正規名helper、正規名Git、上限内で再decodeできるGit requestだけを受ける。
    AppContainer固有profile、network capabilityなし、workspace限定ACL、一時drive、kill-on-close Job、bounded request／output、
    temporary grant完全cleanupをbackend必須条件に固定した。まだACL変更、profile生成、drive mapping、process起動は行わず、
    command gateはOFFのままである。
  - Windows isolation resource lifecycle follow-up
    host launcher内部に、一意AppContainer profile、workspace rootへの一時read／execute ACL、未使用drive mapping、
    kill-on-close Job ObjectをRAII資源として追加した。明示closeと途中失敗時Dropの双方がJob、drive、ACL、profileの順に
    cleanupへ入り、System32の正規`icacls.exe`／`subst.exe`以外を起動しない。helper processはまだ起動しない。
    通常のCodex sandbox内ではAppContainer profile作成前に拒否されたが、Owner承認後のsandbox外fixtureはPASSした。
    ACL付与前から存在するfileを一時drive経由で読取り、close後はdriveとSID grantが消え、profile削除とJob closeも成功した。
    Windows TEMP内のfixture directory残りは0件、`subst` mapping残りも0件だった。helper processはまだ起動しない
  - isolated helper process host follow-up
    helperをnetwork capabilityなしのAppContainer processとしてsuspendedで作成し、kill-on-close Jobへ割り当ててから開始する
    host起動部を追加した。継承handleはbounded request pipeとstdout／stderr pipeだけで、clean environment、host側output上限、
    helper上限より長い固定deadline、全resource cleanupを要求する。初回の実helper fixtureは、独自environment blockに
    AppContainer専用`LOCALAPPDATA`／`TEMP`／`TMP`がなく`CreateProcessW`のWin32 error 203でfail closedした。
    SIDからWindows APIで専用profile pathを取得し、作業driveのcurrent-directory entryと合わせて追加した後は、実helper
    `0E25A57AA907F5C4357065AFBF06C178AD3F925BFC9A21ACCC470299D0C33B3B`から固定Git statusがPASSした。
    hostのPATH、credential、通常user環境は継承していない。終了後はfixture directory、`subst` mapping、AppContainer profileが
    いずれも0件だった。desktop／Codexへのcommand tool公開は未接続で、command gateはOFFのままである
  - desktop pre-launch revalidation follow-up
    準備済みcommandを実行する直前に、Room sessionとworkspace identityを再取得し、helperとGitのfilesystem identity／SHA-256を
    再検査してからbackendを再構築するdesktop内部接続を追加した。workspace、helper、Gitのいずれかを準備後に差し替える
    単体fixtureはすべてprocess起動前に拒否した。最初の実desktop fixtureでは公式`Git\\cmd\\git.exe`が`git-lfs.exe`との
    hard-link共有により安全検査で拒否された。同じ公式install内で単一hard-linkの`Git\\bin\\git.exe`へ固定候補を変更し、
    desktop準備から実helper、固定Git statusまでsandbox外fixtureがPASSした。終了後のdesktop／launcher fixture directory、
    `subst` mapping、AppContainer profileはいずれも0件だった。Tauri command、Codex tool、frontendには公開しておらず、
    command gateはOFFのままである
  - host-owned Room command context follow-up
    provider-neutralな`TextTurnRequest`へoptional Room IDを追加し、Codex dispatch時にdesktop hostが現在のRoom IDを設定する。
    Room IDはprompt、workspace dynamic tool schema、Provider command lineへ含めず、adapter内部でのみ参照できる。
    command execution contextとCodex dynamic toolはまだ接続しておらず、command gateはOFFのままである
  - exact Room session turn context follow-up
    desktop command executionはhost-owned `TextTurnRequest`のRoom ID、workspace root、participant accessと、現在の
    Room command session／workspace identityから不透明なturn contextを作る。command準備APIからRoom IDとaccess入力を除き、
    contextだけを受けるようにした。準備後に同じRoom／workspaceでsessionを開始し直した場合も実行直前に拒否する。
    Codex dynamic toolにはまだ公開せず、command gateはOFFのままである
  - first product command tool follow-up
    Codex App Serverのworkspace turnに、activeなexact turn contextを作れた場合だけ`mio_command.git_status`を追加した。
    schemaは引数なしで、workspace rootの固定`git --no-pager -c core.fsmonitor=false status --short --branch
    --untracked-files=all`だけを既存の隔離helperへ渡す。workspace内Git設定からfsmonitor補助programを起動させない。
    thread／turn／call ID、namespace、tool名、空のargumentsを再検査し、sessionやworkspaceが変われば準備時または実行直前に
    fail closedする。任意command、path、argument、network、credential、Git変更操作、npm／Cargoは公開していない。
    2026-08-31の実`D:\desktop\M.O.E`製品試験では、準備28.6 ms後にtemporary ACLの付与・除去へ
    約18分を要し、helperはexit code 10、stderr 69 bytesの`not a git repository`で終了した。終了後にroot、`.git`、
    `apps`、`target`へAppContainer SID ACEが残っていないことは確認したが、実用性と実workspace境界は不成立である。
    一方、小さいC:／D: Git fixtureはPASSしており、D:一般の失敗とは扱わない。実workspace相当の境界がPASSするまで
    `COMMAND_DYNAMIC_TOOLS_ENABLED`をfalseとし、`mio_command.git_status`をCodexへ公開しない
  - 2026-08-31 root-only ACL follow-up
    再帰`icacls /T`を使わず、workspace rootへ継承可能なread／execute ACEを1件だけ一時付与するtest-only候補を作った。
    D:上の一時Git fixtureに既存子ファイル256件を置いた固定Git statusは947.3 msでPASSし、temporary SID ACEも0件だった。
    しかし実`D:\desktop\M.O.E`では60秒を超えて付与処理が完了せず中止した。中止後に遅れて反映された今回のroot ACEは
    exact SIDで回収し、一時profileもexact monikerで削除した。root DACL／Owner、Git index SHA-256、主要子path、drive mapping、
    helper／Git processに残存や変更がないことを確認した。root-only候補は製品経路へ採用せず、command dynamic tool製品gateは
    OFFのままとする
  - host-owned read-only Git status broker follow-up
    `git.exe`、shell、hook、project script、network、credentialを使わず、desktop process内のlibgit2で選択Roomの
    Git statusだけを読む専用brokerへ切り替えた。System／ProgramData／XDG／GlobalのGit設定探索を起動後の
    初回利用前に空へ固定し、local configのinclude、外部ignore／attributes、fsmonitor、filter、別worktree／objectsを
    拒否する。通常のporcelain相当としてuntracked directory内へ再帰せず、index refresh／更新も行わない。
    実`D:\desktop\M.O.E`では60.0 msでPASSし、Git index SHA-256とworkspace root ACLは前後一致した。
    active Room session、workspace root、filesystem identityを呼出し直前に再照合し、この`mio_command.git_status`だけを
    Codexへ公開する。任意command、build、test、Git変更操作と従来のAppContainer command経路は引き続きOFFとする
  - fixed Node entry follow-up
    read-write Roomだけに`mio_command.run_node({ directory })`を公開する。受け取るのは既存workspace相対directory 1件だけで、
    command文字列、argument、別entry fileは受け取らない。hostはdirectory内の`mio-main.mjs`を確認し、そのsubtreeだけへ
    一時modify ACEを付与する。Nodeは登録済みWindows layoutの`node.exe`をidentity／single hard link／SHA-256で検証・再検証し、
    1回限りのAppContainer data directoryへcopyしてから、network capabilityなし・clean environment・null stdin・bounded output／
    timeout・kill-on-close Jobで固定entryを実行する。終了時はworkspace ACE、仮想drive、staged Node、AppContainer profileを回収する
  - 実helper＋実Nodeの小さいTemp fixtureと、host user所有の
    `target/mio-node-host-smoke`はPASSした。後者はM.I.O.が先に作った5,691 bytesのHTMLを読み、SHA-256
    `BBEC06F7A58BBE73EF85390C9ADC008BB6F9E338890BA3D873C6A9B878C7A034`を結果JSONへ保存した。前後でOwnerは`CLUT\black`、
    temporary AppContainer ACEは0件だった。Codex file sandbox所有で作った別fixtureはWindows hostがACLを変更できず安全側に失敗したため、
    製品試験fixtureは通常のhost user所有で用意する
  - exact npm package install follow-up
    read-write Roomだけに`mio_command.install_npm_package({ directory, packageName, exactVersion })`を公開する。
    対象directoryには既存`package.json`を必須とし、lowercase npm名と厳密なSemVer 1件だけを受ける。tag、range、URL、
    local path、Git source、command文字列、追加optionはbrokerとhelper protocolの両方で拒否する。実行前にはM.I.O.の
    action-time confirmationで`package@version in directory`を表示する
  - fixed npm script follow-up
    read-write Roomだけに`mio_command.run_npm_script({ directory, script })`を公開し、`script`は`build`、`test`、
    `typecheck`の列挙値だけを受ける。対象directoryの既存`package.json`と現在のWindows user所有を準備時・実行直前に
    再検査し、script欠落は`--if-present`で成功扱いにしない。任意script名、追加argument、command文字列、read-only
    Room、Cargoはprocess起動前に拒否する。package内に定義済みのscriptは選択subtreeへ変更を行えるため、Node／npm
    と同じ一時AppContainer、network capabilityなし、clean profile／cache、Job Object、bounded output／timeoutで実行する
  - 正規Node layout内のnpm runtime 1,770 files／10,517,885 bytesを、reparse pointなし、single hard link、file／directory／
    total size上限、tree SHA-256で検証・再検証する。`npm.cmd`やshellは使わず、staged `node.exe`からstaged `npm-cli.js`を
    起動する。`--ignore-scripts`、`--save-exact`、`--package-lock=true`、local install、公式registryをM.I.O.側で固定する
  - package installのAppContainerに付けるcapabilityは外向き通信だけの`internetClient` 1件とする。private network、server、
    credentialは与えない。選択directoryとstaged toolは別々の一時driveへ限定し、終了時にACL、drive、npm cache、staged
    runtime、profileを回収する。実helper／実Node／実npmによる一時fixtureへの`is-number@7.0.0`導入は、exact dependency、
    `package-lock.json`、`node_modules/is-number/package.json`を確認してPASSした
  - Node／npmの実行directoryは、現在のWindows user所有であることを確認画面より前に検査し、許可後とhelper実行直前にも
    再確認する。別所有者、所有者取得不能、途中変更はそれぞれ専用errorで安全側に拒否する。通常user所有の製品UI fixtureでは
    `is-number@7.0.0`の導入と両manifest読戻しがPASSし、`CodexSandboxOffline`所有の旧fixtureは確認画面を出す前に拒否した
  - 任意Node argument、任意npm command／script／argument、直接shell tool、Cargo、Git変更操作は未公開のまま
  - 2026-08-31の現行source再確認では、command broker／helper protocol／command runner／Git status broker／Windows
    launcher／runner／workspace brokerのfocused test 55件とdesktop test 183件が成功、失敗0件だった。focused testでは
    実workspace読取り1件とtemporary ACL／downloadを伴うlauncher test 9件、desktopでは実Provider／製品fixture等14件を
    通常testから除外した。さらに実helper境界の固定npm `build`／`test`／`typecheck` fixture 1件を明示実行してPASSした

### 3. Providerごとのmodel選択を完成させる

- [x] 初期値を`Providerの既定`として、model固定を利用者へ強制しない
- [x] ProviderごとにM.I.O.で検証済みのmodel候補だけを表示する
- [x] 選択したmodelをdevice-localなparticipant profileへ保存する
- [x] model変更時は古いProvider continuityを再利用せず、新しいsessionを開始する
- [x] 契約外、廃止、利用不能なmodelを別modelへ黙ってfallbackしない
- [x] model名、利用Provider、課金／保持がProvider契約に従うことをUIへ表示する
- [x] Conductorと各workerがそれぞれの選択modelで動作することを確認する
- [x] Codex、Gemini、Claude CLI、Grok CLIのmodel指定／既定動作を個別に回帰試験する
  - participant profileに後方互換な`aiModel`を追加した。既存fileで未記録の場合は`providerDefault`へ移行し、CLIへmodel引数を
    送らない。明示候補はCodex `gpt-5.6-sol`／`gpt-5.6-terra`／`gpt-5.6-luna`、Claude Code `claude-fable-5`／
    `claude-opus-5`／`claude-sonnet-5`、Grok
    `grok-4.6`だけとし、Geminiは
    Provider既定だけを表示する。自由入力と別Providerのmodel IDはhost側で拒否する
  - 選択modelをprovider-neutral turn requestへ渡し、continuity environment keyにも含める。model変更後はRoom履歴を残したまま
    新規Provider sessionを開始する。Codex App Serverには明示modelと`allowProviderModelFallback=false`を渡し、Claude／Grokは
    明示選択時だけ`--model`を付ける。未許可IDはprocess起動前に拒否する
  - profile UIにはProvider既定と確認済み候補、Provider契約に従う課金／保持、silent fallbackなしを日英で表示した。
    frontend production build、SDK 5 tests、desktop 180 testsはPASS。2026-08-31にCodex実accountで`gpt-5.6-sol`、
    `gpt-5.6-terra`、`gpt-5.6-luna`を各1回明示指定し、期待したreplyを確認した。Provider既定とConductor、他Providerは
    後続の実機試験で確認した
  - 2026-08-31にM.I.O.が優先使用するClaude Code 2.1.220で`claude-fable-5`、`claude-opus-5`、
    `claude-sonnet-5`を各1回明示指定し、toolなし・session保存なしで期待した固定replyを確認した
  - 2026-08-31にConductorのCodex plan／synthesisへ同じ選択modelを渡し、`Providerの既定`ではmodel指定を
    送らないよう修正した。さらにCodex／Grok／Claude Codeの各workerへ本人の選択modelだけが届き、Geminiの
    `Providerの既定`はmodel指定なしとなることをdesktop unit testで確認した
  - 2026-09-01にcommit `fa93552`から正規のTauri release buildを生成し、Codex実accountのConductorで
    direct answer `CONDUCTOR_DIRECT_OK`を確認した。続いてparticipant ID `claude-code`へ1件だけ委任し、
    AI dispatch ledgerの`completed`、Room orchestration ledgerのdelegation 1件／`completed`、最終回答
    `CONDUCTOR_SYNTHESIS_OK: WORKER_MODEL_OK`を確認した。両profileは旧fileから移行した`Providerの既定`であり、
    この時点では明示modelを使うConductor総合試験とGrok明示model試験が残っていた
  - 同試験で、profile表示名を`Claude Fable`へ変更してもConductor planning packetはRoom snapshotの
    `Claude Code`を表示名として使う問題を確認した。2026-09-01にplanning packetの表示名をprofile優先、Room snapshot
    fallbackへ修正し、`Claude Fable`と安定したparticipant ID `claude-code`を同時に渡すdesktop unit testを追加した。
    profile未保存時に従来名`Claude Code`へ戻る試験を含むRoom orchestration 8試験がPASSした。正規のTauri release
    buildでも、ownerが内部IDを指定せず表示名`Claude Fable`だけで依頼し、delegation 1件の宛先`claude-code`、AI dispatch
    ledgerの`completed`、worker実返答`WORKER_ALIAS_OK`、最終回答`CONDUCTOR_ALIAS_OK: WORKER_ALIAS_OK`、Room
    orchestration ledgerの`completed`を確認した
  - 2026-09-01に同じ正規release buildで、Codex profileを`gpt-5.6-terra`、Claude Fable profileを
    `claude-opus-5`へ明示設定してConductor総合試験を実施した。保存profile、表示名によるClaude Fableへのdelegation 1件、
    AI dispatch ledgerの`completed`、worker実返答`WORKER_SELECTED_MODEL_OK`、最終回答
    `CONDUCTOR_SELECTED_MODEL_OK: WORKER_SELECTED_MODEL_OK`、Room orchestration ledgerの`completed`を確認した。
    これにより、この2者の選択設定でM.I.O.の実経路がfallbackエラーなく完了することは確認できた。一方、現在のledgerは
    Providerへ送ったmodel ID自体を保存しないため、Provider内部で実際に選択されたmodelの別証明とは扱わない。残る
    Geminiの既定動作とGrokの明示modelを含む他worker構成は後続試験で確認した
  - 2026-09-01に同じ正規release buildで、Gemini隊長のprofileを`Providerの既定`のままConductor workerとして
    1件だけ委任した。delegation宛先`gemini`、AI dispatch ledgerの`completed`、worker実返答
    `WORKER_PROVIDER_DEFAULT_OK`、最終回答`CONDUCTOR_PROVIDER_DEFAULT_OK: WORKER_PROVIDER_DEFAULT_OK`、
    Room orchestration ledgerの`completed`を確認した。Gemini既定modelへmodel IDを送らない契約の実経路は完了し、
    この時点で残る実account回帰は主にGrokの明示modelとなった
  - 2026-09-03にGrok Build CLI `1.0.13`をOAuth再認証し、source release本体のDirect modeで`Providerの既定`を使う
    固定reply試験を1回実行した。Room snapshotにはOwner送信1件とGrokの完全一致reply
    `GROK_1_0_13_OK` 1件だけが保存され、重複replyはなかった
  - 続いてGrok profileで`grok-4.6`を明示選択し、同じ製品UIのDirect modeから固定replyを1回送った。
    Room snapshotにはOwner送信1件と完全一致`GROK_4_6_EXPLICIT_OK` 1件だけが保存され、重複replyはなかった。
    Grok自身のsession `01a064ba-315a-7be0-a11e-25874cb85776`にも`current_model_id: grok-4.6`、
    update metadataの`modelId: grok-4.6`、`grok-4.6-build`のmodel call 1回、正常終了`end_turn`が記録されたため、
    silent fallbackなしの明示model実行まで確認済みとする
  - 同日、Codex `gpt-5.6-terra`をConductor、Grok `grok-4.6`を唯一のworkerとして製品UIから1件委任した。
    orchestration ledgerはGrokへのdelegation 1件と`completed`、AI dispatch ledgerも`completed`を記録し、worker返信
    `GROK_WORKER_4_6_OK`と最終回答`CONDUCTOR_GROK_4_6_OK: GROK_WORKER_4_6_OK`が保存された。Grok session
    `01a0651a-8eda-7ee2-8219-24510ec2ed20`は`current_model_id: grok-4.6`、model call 1回、正常終了`end_turn`を記録した。
    これをCodex／Claude／Geminiの先行証拠と合わせ、Provider別model回帰とConductor worker構成を完了とする

### 4. 配布installerを完成させる

- [x] Windows x64検証installerのbuild時に専用`moe-command-helper.exe`を同じtarget／static CRT設定でbuildし、
      Tauri sidecarとして同梱する。2026-08-30の展開確認ではbuild元とinstaller内のSHA-256
      `8C0BEEF811CA2C7773C4F70E73B2893578E336C7F236B8BE0FF655C2213F62E3`が一致した
- [x] 2026-08-31にinstaller build flavorだけWindows-safeな`productName: M.I.O`へ分離し、未署名NSIS
      `M.I.O_0.1.0-alpha.2_x64-setup.exe`を生成した。生成scriptの`PRODUCTNAME`、install先、uninstall表示、
      Start menu／desktop shortcut式がすべて`M.I.O`／`M.I.O.lnk`になることを確認した（ADR 0041）
- [x] code signing証明書とtimestamp方針を決める
  - ADR 0045で正式V1のmain executable／NSIS installerへAuthenticode署名を必須化し、SHA-256、
    RFC 3161＋SHA-256 timestamp、certificate subject完全一致、Default Authentication Policyでの
    chain検証を採用した。certificate issuerと実subjectはOwnerの購入・本人確認後に確定する
  - 2026-09-03にSignPath Foundation無料OSS枠を確認したが、commercial dual licensingを行わない
    条件がADR 0004／`COMMERCIAL-LICENSE.md`と衝突するため申請していない。商用license方針を
    勝手に削除せず、`docs/SIGNPATH-FOUNDATION-READINESS.md`へ代替案と再開条件を記録した
- [ ] V1のEXEとinstallerへ署名し、Authenticodeのpublisher、chain、timestampを確認する
  - 2026-09-03に公開段階をADR 0045へ追記した。commercial dual licensingを維持し、未署名版は
    RC／Previewと明示する。正式V1の第一候補はStore署名されるMSIXとし、実機適合性と審査結果を
    確認する。一度の不合格だけで商用方針を変更せず、修正・再申請を先に検討する
  - Store署名と自前署名がともに現実的でない場合だけ、未署名正式V1またはcommercial dual
    licensing終了後のSignPath申請をOwnerが別途決定する。現時点ではどちらも決定していない
  - 2026-09-03にTauri 2.11.4がWindows bundleとしてMSI／NSISだけを提供することを確認し、Windows SDK
    `MakeAppx.exe`によるfull-trust desktop MSIXの構造試作を追加した。preview identityでmanifest検証、
    package生成、再展開、本体／`moe-command-helper.exe`／license／third-party notices／ロゴ4種の存在確認が
    PASSした。最終構造試作は6,505,389 bytes、SHA-256
    `6A2A1DB92142347A3E3DDB770773F9E3CD463B06F00C8EAEC1DCAFFEA468E8EF`
  - 同日、Partner CenterでMSIX／PWAアプリとして`M.I.O`を予約した。Store IDは`9NS9B7T71XHN`、
    Package Identity Nameは`TINMOON.M.I.O`、Publisherは
    `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`、PublisherDisplayNameは`TINMOON`。MSIX生成scriptの
    既定値をこのStore identityへ合わせた
  - 同identityで`MakeAppx.exe`による生成と再展開を実行し、本体／`moe-command-helper.exe`／license／
    third-party notices／ロゴ4種の存在確認がPASSした。packageは6,505,409 bytes、SHA-256
    `06BDC3DAEE7D8AD7D94983E34806D8B30FC30F1471DA32668586AE12220867B4`。隣接`.sha256`も一致し、Store提出用の
    未署名local outputとして`.tools/store-msix/`に保持している
  - 2026-09-03にこのStore用MSIXをPartner Centerへuploadし、package検査`Validated`、英語（米国）／
    日本語（日本）のStore登録情報、価格と提供、プロパティ、年齢区分、申請オプションの完了を確認して、
    Submission 1（ID `1152921505701802472`）を認定へ正式提出した。2026-09-04 07:30 JSTの再確認では、
    製品は`認定中`、工程は`認定`が進行中で、`公開処理中`は未開始だった。認定合格、Store署名、公開、
    Storeからの取得／install／起動はまだ確認していない
  - 2026-09-04にOwnerの明示承認で、認定後すぐに公開する設定から、`今すぐ公開`を選ぶまで
    公開しない設定へ変更して適用した。Partner Centerは`今すぐ公開`をクリックした時点で公開開始と
    表示し、認定は継続中だった。認定合格とStore公開は別の確認・承認境界として扱う
  - 同日、Ownerの明示承認で旧`1.0.0.0` packageの提出を取り消して削除し、正式V1候補commit
    `8e3eb4e22c67a52007ca1890cb912b477314368a`から作成した`1.0.1.0` packageへ差し替えて再提出した。
    packageは6,479,052 bytes、SHA-256
    `279D7DFFBF36C06E7FF698C599C91530F567DEB8FFF605AA5E1AEEEA6BEAFDD5`で、Identity、Publisher、
    x64、desktop本体`1.0.0`、helper、再展開、Defender scanを確認済みである
  - 2026-09-05 12:51 JSTのPartner Center確認では、上部に`認定プロセスに合格しました`、
    `公開準備が完了しました`、`今すぐ公開`が表示された。下部の申請欄には`認定中`も残る混在表示だった。
    公開処理は開始しておらず、Ownerが`今すぐ公開`を明示承認するまで公開を保留する
  - 同日13:04 JST、保管済みの提出実体
    `.tools/public-release-prep/1.0.0/8e3eb4e22c67-store-20260904-104728/artifacts/`
    `M.I.O_1.0.1.0_x64_store-unsigned_20260904-120025.msix`を再照合した。sizeとSHA-256は上記記録および
    隣接sidecarと一致した。manifestはIdentity `TINMOON.M.I.O`、Publisher固定値、version `1.0.1.0`、
    x64、Windows.Desktop、minimum `10.0.22000.0`、`en-us`／`ja-jp`を保持していた。本体EXEの
    FileVersion／ProductVersionは`1.0.0`で、候補commit以降のruntime code変更は0件だった。
    このlocal packageはStore提出用のため`NotSigned`であり、Store署名済み配布物の実機確認は公開後に残る
  - 同日の未login browser確認では、`https://apps.microsoft.com/detail/9NS9B7T71XHN`は
    `製品が見つかりません`を返した。認定合格後も一般公開は開始していないことを確認した
  - Partner Centerの`Current packages`でもSubmission 1のfile名、version `1.0.1.0`、x64、
    Windows.Desktop minimum `10.0.22000.0`、`en-us`／`ja-jp`、`runFullTrust`、表示size `6.2 MB`を確認し、
    local提出実体のmanifestと一致した。Store側packageとの公開前照合はPASSとする
  - 2026-09-05、Windows実行file名を`mio-desktop.exe`／`mio-command-helper.exe`へ統一したcommit
    `dcb90b8a029146506eb5b638931e5b19384a7194`から、Store更新package
    `M.I.O_1.0.2.0_x64_store-unsigned_20260905-170824.msix`を生成した。sizeは6,520,238 bytes、
    SHA-256は`1605B2712FB1421D70C46450E70C9E7045E08F886433F83F0980E5CF331BC1CB`。
    再展開で新しい2実行fileと旧`moe-*.exe`不在、Identity、Publisher、x64、version `1.0.2.0`を確認した
  - 同日、Ownerのupload承認と再申請承認を別々に得てSubmission 2へ差し替えpackageを登録し、
    Partner Centerのpackage検査`Validated`、Windows 10/11 Desktop、x64、version `1.0.2.0`を確認した。
    最終確認時点は`更新プログラムの認定中`／`前処理中`であり、更新の認定完了・配信完了とは扱わない
  - 2026-09-07、Submission 2の`1.0.2.0`が公開済みであることをPartner Center実画面で確認した。
    現在のsourceから`M.I.O_1.0.3.0_x64_store-unsigned_20260907-080838.msix`（6,515,756 bytes、
    SHA-256 `B6B607C5802523E1C4DDCC97285E033DC0FC3EF1D98E2703F429D250C6DF58DD`）を生成・再展開検査した。
    Identity、Publisher、x64、version `1.0.3.0`、Windows.Desktop minimum `10.0.22000.0`、
    `en-us`／`ja-jp`、`runFullTrust`、本体とhelper、license類は一致した。Owner承認後にSubmission 3
    （ID `1152921505701823456`）へupload、保存、認定提出し、package `Validated`、最終確認時点
    `更新プログラムの認定中`／`前処理中`を確認した。公開中のSubmission 2は維持している
  - 同日、Partner Centerの実画面でSubmission 3が公開中になったことを確認した。読み取り専用の
    package詳細は`M.I.O_1.0.3.0_x64_store-unsigned_20260907-080838.msix`、version `1.0.3.0`、
    x64、Windows 10/11 Desktop、minimum `10.0.22000.0`を示していた。認定中の更新は残っていない
  - 同日、次期版commit `fbc81b89ee64a670f13128487c06a96bfabe3238`から、Store更新候補
    `M.I.O_1.0.4.0_x64_store-unsigned_20260907-220224.msix`（6,527,560 bytes、SHA-256
    `EFF886B23E261AAEBE758A27BD684CA08776C4F702EE5AC44D633C4E563F0192`）を生成した。
    package scriptへ4桁versionを渡さず、manifest `1.0.4`から`1.0.4.0`が自動採用された。
    MakeAppxによる生成と再展開、Identity、Publisher、本体、helper、license類、ロゴ4種の存在確認は
    PASSした。このlocal packageは未署名で、サイドロードは行っていない。Owner承認後にSubmission 4
    （ID `1152921505701829315`）へupload、保存し、Partner Centerのpackage検査`Validated`、x64、
    Windows 10/11 Desktop、minimum `10.0.22000.0`、`en-us`／`ja-jp`、`runFullTrust`を確認した。
    Ownerの認定提出承認後にMicrosoftへ送信し、最終確認時点は`更新プログラムの認定中`／`前処理中`。
    公開中のSubmission 3／`1.0.3.0`は維持している
  - 2026-09-08、Partner Center実画面でSubmission 4／`1.0.4.0`が公開済みとなったことを確認した。
    Owner承認後のStore app更新も成功し、Store署名と`Status: Ok`、本体version `1.0.4`を確認した。
    wingetでは更新なしと表示されたため、Store appの実際の更新結果を採用した。
    通常版の5 Room／316 messageとStore版の4 Room／7 messageはAppData仮想化で分離されていた。
    両側のsnapshotを保全し、Ownerの希望どおりStore側を継続使用する。履歴移行は行わない
- [ ] Windows 11 x64のclean環境でinstall、起動、終了を確認する。Windows 10はADR 0043に従いV1で保証しない
  - 2026-09-05、同じWindows 11実機で旧版を通常uninstallしてから`dcb90b8`系NSISを再installし、
    初回起動がほぼ即時に完了することをOwnerが目視確認した。`C:\Program Files\M.I.O`には
    `mio-desktop.exe`と`mio-command-helper.exe`が存在し、旧`moe-desktop.exe`／`moe-command-helper.exe`は
    存在しなかった。これは実機再install確認であり、別のclean環境試験は未完了のままとする
  - 2026-09-07、同じWindows 11実機のper-machine EXE版`1.0.3`へ、commit `fbc81b8`から生成した
    未署名`M.I.O_1.0.4_x64-setup.exe`を上書きinstallした。HKLM登録、起動中EXEのFileVersion／
    ProductVersionはすべて`1.0.4`、実行先は`C:\Program Files\M.I.O\mio-desktop.exe`だった。
    Ownerが既存Roomと会話履歴の保持、発言右クリックメニューの1行表示を目視確認した。右上の終了後は
    本体／helper processが残らず、開発用port 1420も解放された。これは同じ実機の更新試験であり、
    別のclean環境試験は未完了のままとする
- [ ] WebView2 Runtimeがない環境で公式bootstrapper経由の導入を確認する
- [x] 「設定 → アプリ」からinteractive uninstall画面と削除完了を人の目で確認する
  - 2026-09-02にcurrent-user RCのHKCU登録内容は正しい一方、このWindows 11実機の設定、
    「プログラムと機能」、`Get-Package`、`winget list`では列挙されないことを確認した。表示名の
    period有無は原因ではなかった。同じ実機で一時HKLM probeが両方のWindows画面へ表示され、削除も
    確認したため、ADR 0044で最終V1をper-machine installへ変更した
  - 変更後の未署名`1.0.0-rc.1`をUAC承認付きで実installし、Windows設定で`M.I.O`／`1.0.0-rc.1`／
    `moe`／18.4 MBが表示されることをRoom ownerが目視確認した。設定からinteractive uninstallし、
    `C:\Program Files\M.I.O`、HKLM uninstall登録、全ユーザーshortcutが削除されたことも確認した。
    Windows設定の検索はperiodを無視しないため`M.I.O`では一致し、`MIO`では一致しない
- [x] alpha.2の実データを保持したままV1へ更新できることを確認する
  - 2026-09-02に製品AppData 995 filesを別driveへ全件SHA-256付きでbackupし、その複製へ
    `0.1.0-alpha.2`をcurrent-user installして起動した。5 Rooms／257 messagesと
    `room-snapshot-v1.json`のSHA-256
    `385D5BE4876E5AE9B92D5F488AA57C75BBDB73266219CA676FEEDE2351F24F0A`を維持したまま、
    working treeから生成した`1.0.0-rc.1`を上書きinstallして起動できた
- [x] V1の上書きinstall、再起動、uninstall後にもRoom dataが意図どおり扱われることを確認する
  - 更新後の実行binaryとuninstall登録はともに`1.0.0-rc.1`だった。app data削除を選ばない
    interactive uninstall後はinstall directory、uninstall登録、Start menu shortcutが消え、
    5 Rooms／257 messagesとsnapshot hashは変化しなかった。試験後は製品AppDataを検証済みbackupから
    Roaming 24 files／Local 971 filesの全件SHA-256一致で復元し、試験結果も別名で保全した
- [x] Start menu、desktop shortcut、install先、uninstall登録の名称とversionを確認する
  - 同じ試験でinstall先`C:\Users\black\AppData\Local\M.I.O`、Start menuの`M.I.O.lnk`と
    そのtarget、uninstall登録の`M.I.O`／`1.0.0-rc.1`を確認した。desktop shortcutはこの試験では
    作成されなかったため、finish pageでの任意作成とuninstall時の削除確認が残る
  - 後続のcurrent-user RC再installでは`D:\desktop\M.I.O.lnk`を作成し、Start menuとdesktopの
    両shortcutが`C:\Users\black\AppData\Local\M.I.O\moe-desktop.exe`を指すことを確認した。
    最終判定はADR 0044のper-machine installerでinstall先と全shortcutの作成・削除を再確認して行う
  - per-machine RCでは実行中binaryが`C:\Program Files\M.I.O\moe-desktop.exe`／`1.0.0-rc.1`、
    HKLM登録が`M.I.O`／`1.0.0-rc.1`、Start menuが
    `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\M.I.O.lnk`、desktopが
    `C:\Users\Public\Desktop\M.I.O.lnk`だった。両shortcutはProgram Filesの同じbinaryを指し、
    interactive uninstall後にinstall directory、登録、両shortcutがすべて消えた
- [x] install完了後に自動でfinish pageへ進むことを確認する
  - Tauri 2.11.4標準NSIS templateの診断向け停止動作を、`NSIS_HOOK_POSTINSTALL`の
    `SetAutoClose true`で製品向けに変更した。working-tree installer buildは成功し、SHA-256
    `061B2C78BC33846985A981231E27AA9DE46E6C6EF1503F4E9D31F045A76280E8`の実上書きinstallで
    Room ownerが自動遷移を目視確認した。このartifactは未署名の試験用で、最終release artifactではない
  - per-machine変更後もbuildに成功し、試験artifactのSHA-256は
    `A50D4F5E124981CFFDF3D52B1BE57C23CD5FF5888C17B8DDBC495A62B5E88ACA`だった。生成NSIS定義で
    `RequestExecutionLevel admin`、`$PROGRAMFILES64`、HKLM uninstall、`SetAutoClose true`を確認した
- [ ] Windows Defenderでrelease artifactをscanする
  - 2026-09-02に未署名per-machine RC
    `M.I.O_1.0.0-rc.1_x64-setup.exe`（SHA-256
    `A50D4F5E124981CFFDF3D52B1BE57C23CD5FF5888C17B8DDBC495A62B5E88ACA`）を、定義version
    `1.457.447.0`のWindows Defender custom scanへ通した。antivirus／real-time protectionは有効で、
    threat detection数はscan前後とも0だった。最終署名release artifactで同じ確認を繰り返すまで完了扱いにしない
  - 2026-09-05、`dcb90b8`から再生成した直接配布用の未署名Preview候補installer
    `M.I.O_1.0.0_windows-x64_unsigned-preview_setup.exe`（4,512,406 bytes、SHA-256
    `17413C474EAD97D6673277E88C88CD5A562CA6DF9E041D97610B0B9F57925AF5`）とsource ZIPを含む
    artifacts directoryをWindows Defender custom scanへ通し、exit code 0／脅威なしを確認した。
    engineは`4.18.26080.3`、定義は`1.459.58.0`、antivirus／real-time protectionは有効だった。
    Authenticode署名付きの一般配布版またはStore署名済み更新実体の確認までは項目全体を未完了とする
  - 2026-09-07、commit `fbc81b8`の未署名`M.I.O_1.0.4_x64-setup.exe`（4,516,598 bytes、
    SHA-256 `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`）と、未署名Store候補
    `M.I.O_1.0.4.0_x64_store-unsigned_20260907-220224.msix`（6,527,560 bytes、SHA-256
    `EFF886B23E261AAEBE758A27BD684CA08776C4F702EE5AC44D633C4E563F0192`）を個別にDefender
    custom scanへ通し、exit code 0、両artifactの検出記録0件を確認した。engineは`1.1.26080.3`、
    定義は`1.459.93.0`、antivirus／real-time protectionは有効だった
- [x] 自動更新はADR 0043に従いV1へ含めず、署名済みinstallerによる手動updateとする
  - 英語／日本語のREADME、利用ガイド、support policy、Release notes Draftへ、backup、終了、hash・署名確認、
    既存installへの上書き、起動後のdata確認を含む手順を記載した

SmartScreenの警告が一度も出ないことは、新しい署名のreputationに左右されるため完全には
保証できない。署名付きV1の必須条件は、正しい発行者またはMicrosoft Storeにより署名され、
改ざん検出ができ、警告が出た場合の案内が正確であることとする。未署名RC／Previewは正式V1と
混同せず、署名なし、SHA-256、source commit、警告時の確認方法を明示する。

### 5. Release Candidateで機能回帰を通す

- [x] V1 RCを固定し、同一commitから全試験とinstallerを生成する
  - 2026-09-02に`1.0.0-rc.1`をcommit `577a9f18bb4f0dd76a112a7d09d015877d5dedb9`へ固定し、
    `prepare-public-release.ps1 -Commit HEAD`を実行した。typecheck、frontend production build、
    Rust format、offline／locked workspace test（331 passed／0 failed／24 ignored）、installer build、
    source snapshot、Gitleaksを同じcommitから完了した。installerは4,452,874 bytes、SHA-256
    `3E96C6A9E613680B01E3D87578771F3D1BD7E28C463736D19651CCDB973E660F`、`NotSigned`、
    source ZIPは359 files、SHA-256
    `FEA9B0F235F9AAA145DD6B8847B87A63A2B420ABE4C068FCD1AFAE0EBACB9449`だった。準備先は
    `.tools/public-release-prep/1.0.0-rc.1/577a9f18bb4f-20260902-111225/`で、外部公開、tag、Release作成は行っていない
- [ ] typecheck、frontend build、Rust format、workspace test、CIをすべて通す
  - 2026-09-07、次期版`1.0.4`の最終候補commit
    `fbc81b89ee64a670f13128487c06a96bfabe3238`でdesktop typecheck、frontend production build、
    `cargo fmt --all -- --check`、`cargo test --workspace --all-targets --locked`、NSIS buildを
    ローカル実行した。349 passed／0 failed／24 ignoredで、各buildもPASSした。clean runner CIは
    まだ実行していないため、項目全体は未完了のままとする
  - 2026-09-05、commit `dcb90b8`でdesktop typecheck、frontend production build、
    `cargo fmt --all -- --check`、`cargo test --workspace --locked`をローカル実行し、338 passed／
    0 failed／24 ignoredだった。同じcommitからNSISも再buildした。GitHub Actionsは今月の利用制限方針に
    従い起動していないため、clean runner CIを含む項目全体は未完了のままとする
  - 2026-09-03の`befd330`後のworking treeで、desktop typecheck、frontend production build、
    `cargo fmt --all -- --check`、`cargo test --workspace --locked`をローカル実行し、すべてexit code 0だった。
    Cargoは新しい4 Provider失敗回帰を含む337 passed／0 failed／24 ignored。GitHub Actionsは月間制限中の
    方針に従って起動していないため、項目全体は最終release候補のclean runner CIまで未完了とする
  - 2026-09-02の`a554f97`でGitHub Actionsを消費せずローカル検証した。desktop typecheck、
    frontend build、未署名per-machine installer build、evidence sanitization 2 tests、
    `cargo fmt --all -- --check`、Windows dependency boundary（1,135 entries／glib 0）、
    `cargo test --workspace`はすべてexit code 0だった。今月のActions利用制限方針に従い、
    手動`workflow_dispatch`によるclean runner CIは最終release候補まで実行せず、項目を完了扱いにしない
  - 2026-09-01にcode HEAD `c32cdf1`で`npm.cmd run typecheck`、`npm.cmd run build`、
    `cargo fmt --all -- --check`、`cargo test --workspace`をローカル実行し、すべてPASSした。Cargoは
    324 passed／0 failed／24 ignoredで、ignoredは実account、実helper、download、Windows資格情報等の
    明示実行専用試験である。GitHub Actionsは月間制限中の方針に従い実行していないため、この項目は未完了のままとする
  - 2026-09-02の`1.0.0-rc.1` working treeでtypecheck、frontend production build、Rust format、
    offline／locked workspace testを再実行し、331 passed／0 failed／24 ignoredだった。CIは同じ月間制限に
    従ってまだ起動していないため、この項目は未完了のままとする
- [x] fresh Roomと既存Roomの起動、保存、再起動復元を確認する
  - 2026-09-02に固定RC commit `577a9f1`から生成したrelease本体でfresh Room `New room 5`を作成した。
    新しいRoom IDと0 messagesのsnapshotが即時保存されたことをWindows上の製品dataで確認し、通常終了後に
    同じRCを再起動した。fresh Roomは一覧へ復元され、既存4 Roomも維持された。既存`M.I.O.. Dev Test`は
    226 messagesを保持して表示され、起動後の`moe-desktop`は1 processだった
- [x] Room作成、rename、削除、backup、正常restore、破損backup拒否を一巡する
  - backup保存先はユーザーが選択でき、現在の絶対pathを表示する。変更時も既存backupは移動しない。
    選択先が消えた場合は既定先へ黙って保存せず、失敗として知らせる。restoreは最新JSONを先に検証し、
    file名・作成日時・Room数を表示した後、同じfileを明示確認した場合だけ実行する実装とした
  - 2026-09-02にbackup境界test 6件、desktop Rust test 197 passed／0 failed／14 ignored、
    typecheck、frontend production build、Rust format、`git diff --check`を通過した。最新版UIでの一巡は未実施
  - 2026-09-02に最新版release UIで既定先`D:\Documents\M.O.E Backups`の絶対pathと保存先選択・
    folder表示・backup・restore previewを確認した。現在の5 RoomとID・message数が一致するfresh backupを作り、
    file名・13:09:29・5 Roomのpreview後に同じfileを正常restoreした。restore後も5 Roomとmessage数が一致し、
    `moe-desktop`は1 processだった。保存先選択dialogがM.I.O.を親窓として開くこととcancelも実画面確認し、
    cancel後にcustom設定fileが作られず既定先と既存4 backupが維持されることを確認した
  - 同日にper-machine RCで空の6番目Roomを作成し、`V1 回帰テスト`へrenameして即時保存を確認した。
    6 Roomsのbackupを作成後に一時Roomを削除し、previewのfile名・19:18:03・6 RoomsをOwnerが確認して
    restoreした。復元後のactive snapshotとbackupはSHA-256
    `67AE0413BA640C087E2136D79162800A223F615A26D9D3EFE83DE91ED08BCC35`で完全一致した。
    一時Roomを再削除して元の5 Roomsへ戻した
  - 36-byteの専用破損JSONを最新backupとして一時配置し、`Review restore...`が正常backupとして扱わず
    Room数5とsnapshot hashを変更しないことを確認した。試験fileは絶対pathを検証して削除し、既存backupには
    触れていない。最後に5 Roomsの正常backup
    `moe-room-backup-00000001788345788296.json`を作り、active snapshotとのSHA-256
    `74E7138023AEE854ADD29CC372C69499208956E32BA42D6A974AD6AB4D4F293A`一致を確認した
  - 初回画面試験でbackup成功・破損拒否messageが長い設定panelの最下部にあり、押しても変化が見えない
    UI欠陥を発見した。通知をpanel上部のsticky feedbackへ移し、修正版を同一versionのmaintenance installで
    実機へ上書きした。赤い破損拒否と緑の5 Rooms backup成功がスクロールなしで見えることをOwnerが確認した。
    typecheck、frontend build、未署名per-machine installer buildは合格し、試験installerのSHA-256は
    `B6AE0722C2F4D2CC0ED9FE4AD6A998763B776FEE404DC553E1CA564BE03F97D6`だった
- [x] 各対応ProviderへDirect modeで1件ずつ送り、実replyとRoom保存を確認する
  - Codexは2026-09-02のper-machine RCで長文Owner messageへ`V1_MESSAGE_PERSISTENCE_OK`を返し、送信台帳の
    `completed`、Room snapshot保存、通常終了後の再表示を確認した
  - Grokは2026-09-03のsource release本体で`Providerの既定`と明示`grok-4.6`のDirect固定replyを各1回保存し、
    同日の読取り専用reviewもDirectで`completed`となった。いずれも重複replyはなかった
  - 同じ2026-09-03のsource release本体で、Gemini隊長とClaude FableだけをDirect宛先にしたOwner messageを1件送り、
    Geminiは`GEMINI_DIRECT_RC_OK`、Claude Codeは`CLAUDE_CODE_DIRECT_RC_OK`を各1件だけ返した。両dispatchは
    `completed`で、2件のreplyが同じRoom snapshotへ保存された。これによりV1対象の4 CLI Providerを完了とする
- [x] Conductorでcompleted、partial、unknownの代表経路を確認する
  - completedは2026-09-03のsource release本体で、Codex ConductorがGrok `grok-4.6`へ1件だけ委任し、
    worker reply `GROK_WORKER_4_6_OK`を受けて最終回答を保存した。operation
    `conductor-operation-v1-eedf5449e4622d3cf5d8ce40810753c8`は`completed`で、Grok側も1 model call／
    `end_turn`だった
  - partial相当の混在worker結果は、Owner所有の一時的な非Git編集フォルダを使い、Claude Code
    `claude-opus-5`を`completed`、Grok `grok-4.6`をProvider起動前の`failed`にした。Codexは実statusだけから
    `CONDUCTOR_PARTIAL_OK: claude-code=completed; grok=failed`を1件保存し、operation
    `conductor-operation-v1-2c1db718ebb6bcb3b325e0caef5354cb`を`completed`で閉じた。Claude replyは
    `CLAUDE_PARTIAL_OK`、Grokの新規session／model callはなかった
  - unknownは既存の実記録`conductor-operation-v1-cd0c7526c3ec2e57a03d93c9cd9b9c33`が`unknown`のまま
    保存され、自動再送されていない。さらに`room_orchestration::tests`全8件を実行し、混在worker結果のsynthesisと
    `unknown_synthesis_is_not_retried`を含めて8 passed／0 failedを確認した
- [x] CLI未導入、未login、network断、timeout時にcrashや偽replyがないことを確認する
  - CLI未導入はpublic alpha時のWindows実画面で4 Providerを個別に利用不能へした記録を再利用した。対象Providerの
    replyを作らず、別recipientの成功replyとM.I.O.の操作継続を保ち、結果不明turnを自動再送しなかった
  - 2026-09-03に`provider_cli_failures_never_create_fake_replies_or_retry`を追加した。Codex、Grok、Gemini、
    Claude Codeそれぞれで、CLI未検出相当の`Unavailable`、未login／network失敗などCLI非0終了相当の`Rejected`、
    product deadline超過相当の`TimedOut`を注入する12経路を検証した。全経路でstatusは`Unknown`、reply messageは
    0件、同じsource messageの再dispatchでもAdapter呼出しは1回のままで`aiDispatchOutcomeUnknown`を返した
  - credentialの削除、実Providerからのlogout、Windows全体のnetwork切断は行っていない。ここで確認するrelease gateは、
    それらが同じAdapter失敗契約へ到達した後にcrash、偽reply、自動再送を起こさないM.I.O.側の処理とする
- [x] 日本語、英語、長文、改行、絵文字を含むmessageの保存と再表示を確認する
  - 2026-09-02にper-machine RCの空Roomへ364文字・複数段落のOwner messageを送り、日本語、英語、
    改行、`😀 🚀 🐑`と、Codexの完全一致reply `V1_MESSAGE_PERSISTENCE_OK`が各1件だけsnapshotへ
    保存されたことを確認した。通常終了後にProgram Files版を再起動し、同じRoomで長文の段落・絵文字と
    replyが再表示されることをOwnerが実画面確認した
- [x] Codexへ明示的に画像生成を依頼し、生成画像のRoom保存、縮小表示、原寸表示、download、
      選択済み編集フォルダへの新規保存を製品経路で確認する
  - 2026-09-01にsource実装とlocal回帰を完了した。Codex App Serverの同一turnで成功した
    `imageGeneration.savedPath`だけを受け、Codexの`generated_images`配下からPNG／JPEG／WebPを
    16 MiB以下でM.I.O.専用保管庫へhash検証付きで取り込む。Room messageが参照するartifactだけを
    UIへ返し、編集フォルダ保存はOwnerのbutton操作からWorkspace Brokerのcreate-only binary経路を使う。
    AI向けUTF-8 text write権限は拡張していない。typecheck、frontend build、Rust format、
    `cargo test --workspace`はPASSした
  - 2026-09-01の正規release buildで、Codexへの明示的な画像生成依頼から1254×1254 PNGを生成した。
    初回は画像resultを含むApp Serverの1行JSONが従来の1 MiB受信上限を超え、生成済み画像をRoomへ
    取り込めず`externalStarted`となった。受信上限を16 MiB画像のBase64表現と固定protocol余白に合わせて
    boundedに拡張し、再試験ではdispatch `completed`、Room reply、会話内縮小表示、原寸リンク、download、
    編集フォルダへのcreate-only保存を確認した。Codex生成元とM.I.O.保管庫の877,271-byte PNGはSHA-256
    `D05ABE02BC435E28270D2AC9ACDDC8958567BCB0197A34E4DF331B428740012E`で一致した
  - 即時保存で長いartifact名が使われた実画面結果を受け、保存buttonからファイル名確認dialogを開くように
    改善した。初期名は`MIO-image-YYYYMMDD-HHMMSS.<ext>`、Ownerが変更でき、拡張子変更、Windows禁止文字・
    予約名、空名をUIで拒否する。同じ画像のdownloadにも、画像読込み時に一度だけ生成した同じ初期名を使う。
    backendのworkspace境界と非上書き保存は変更していない。このUI追加後も
    typecheck、frontend build、Rust format、`cargo test --workspace`、`git diff --check`はすべてPASSした
  - CodexをDirect宛先にした時だけ、composerから画像のアスペクト比（1:1・4:3・3:4・16:9・9:16）を
    比率が見える四角iconで選び、品質（自動・低・中・高）の希望も選べる。選択は端末内に保存し、Room本文には混ぜず、Codexへのtrusted
    local guidanceとして画像生成を明示的に依頼したturnだけへ渡す。2026-09-01の実生成では固定画素数の
    希望が反映されず`2048x2048`希望に対して`1254x1254`が返ったため、固定解像度presetは撤去した。
    Codex内蔵toolにはアスペクト比・品質を明示するが、実結果の保証はせず、正確な画素数指定には対応しないことと、M.I.O.の
    受入れ上限が16 MiBであることをUIに明記した。正確な解像度は将来の直接Image API接続候補とする
- [x] app多重起動、window終了、OS再起動後の復帰を確認する
  - 2026-09-02にcommit `bd668aa`の正規release本体を、停止状態から起動、通常終了、再起動した。
    既存Room一覧、`M.I.O.. Dev Test`の226件の履歴、appearance、Codex宛先、Direct選択、結果不明の
    再送防止表示が復元された。同じEXEを起動中にもう一度起動しても、M.I.O.のprocessとwindowは
    どちらも1件だけだった。これまでの開発期間中にもOwnerが複数回Windowsを再起動し、その後も
    Room、履歴、設定を復元して継続利用できているため、累積した実機証拠を合わせて完了とする
- [x] update前後で既存Room、profile、appearance、dispatch記録が破損しないことを確認する
  - 2026-09-02の上書き更新前に別driveへ保存した全件SHA-256付きAppData backupと、2026-09-03の
    継続利用中AppDataを構造照合した。更新前の5 Rooms／257 messagesは現在の5 Rooms／280 messages内に
    全件存在し、Room metadata、各message本文、宛先、時刻、artifactを含めて欠落0件／変更0件だった
  - 更新前のdispatch 105件も現在の116件内に全件存在し、欠落0件／stateを含む変更0件だった。更新前からある
    Codex、Gemini、Claude Code、Ownerの4 profileは表示名、AI指示、avatar、model、access modeがすべて同一だった。
    appearanceは更新前後ともSHA-256
    `47ABEE57D11B3980102E68BA2B4AB2548B7DDA2A972B6EF734B3A49A1D555F04`で完全一致した
- [x] Room workspaceとmodel選択が再起動後も保持され、安全境界が変わらないことを確認する
  - 同じ2026-09-02の通常終了／再起動で、Room workspace `D:\desktop\M.O.E`、Codex
    `gpt-5.6-terra`／`workspaceWrite`、Claude Fable `claude-opus-5`、Gemini `Providerの既定`が
    保持された。画像生成設定も画面上で`9:16`／`中`のまま復元された。安全境界の再実行は
    この試験に含めていなかった
  - 2026-09-03に更新前backupと現在の`room-workspaces-v1.json`がSHA-256
    `18F9E71C802CCE4442B988DF0FA6A4000A263C32554AAFD505BF232055564622`で一致することを確認した。
    同じ照合で上記4 profileのmodel／access modeもすべて一致し、更新後に追加したGrokは`grok-4.6`／
    `workspaceRead`として保持されていた
  - 同日の`cargo test -p moe-desktop`は198 passed／0 failed／14 ignoredだった。Room workspace identityの
    永続化と差替え検知、Providerごとのmodel分離、Grok read-only要求、Codexのroot既定拒否・network無効・
    elevated Windows sandbox必須・brokered tool限定を含む安全境界が合格したため完了とする

Providerのloginや外部送信を伴う項目はCIでは代替できない。test用messageだけを使い、
実行日、CLI version、対象commit、結果をcredentialなしで記録する。

### 6. SecurityとdependencyをV1基準にする

- [x] GitHub Dependabot APIとWindows向けCargo treeを再確認する。2026-09-03現在のopen alertはRust
      `glib 0.18.5`のModerate 1件だけで、npm ecosystemのopen alertはない
- [x] 現在のGitHub Dependabot open alertでCritical／Highが0件であることを確認する
  - 2026-09-03にnpm `fast-uri 3.1.5`のHigh `GHSA-5jgf-p345-68v8`が新規検出された。
    dependencyはV1製品ではなく将来用`spikes/remote-mcp`の`@modelcontextprotocol/sdk 1.30.0`
    →`ajv 8.20.0`だけから到達する。修正版`fast-uri 3.1.7`へlockを最小更新し、local-only
    Streamable HTTP roundtripと全negative caseがPASSした。`7c3f853`をpush後、GitHub Dependabot
    APIで同alertが閉じ、Critical／High 0件へ戻ったことを確認した
  - 同日にnpm `qs 6.15.3`のModerate 2件（`GHSA-x5fp-wj9c-mxmx`、
    `GHSA-4mjr-xmp4-gh2g`）も検出された。同じ将来用`spikes/remote-mcp`だけから到達するため、
    修正版`qs 6.16.0`へlockを最小更新した。npm audit 0件となり、同じlocal-only roundtripと
    全negative caseがPASSした
- [x] Moderateの`GHSA-wrw7-89jp-8q8g`はLinux GTK／WebKit経路だけにあり、
      `cargo tree --locked --target x86_64-pc-windows-msvc -i glib`ではWindows配布treeに存在しない。
      修正版`glib 0.20.0`をTauri系から独立して強制せず、alertをdismissせずに監視する
  - 2026-09-08、`b3a8a2d`のcheckoutとprivate development repositoryのDependabot APIで再確認した。
    open alertは#1の`glib 0.18.5`／Moderate `GHSA-wrw7-89jp-8q8g`のみだった。
    `cargo tree --locked --offline --target x86_64-pc-windows-msvc -i glib`は正常終了し、依存なし
    （`nothing to print`）。Linux targetでは`gtk 0.18.2`／`webkit2gtk 2.0.2`などから
    `glib 0.18.5`へ到達した。製品sourceの`apps`／`crates`に`VariantStrIter`、`glib::`、
    `g_variant_`の直接使用も見つからなかった。
    [RustSec](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)では対象が`>=0.15.0, <0.20.0`、
    修正版が`>=0.20.0`で、文字列iteratorの未定義動作とNULL pointer参照によるcrashが説明されている。
    local registryの`gtk 0.18.2`は`glib = "0.18"`を要求するため、`glib 0.20`追加だけでは
    既存の脆弱な依存を置換できない。将来Linuxを対象にする場合やTauri／GTK系の更新時に再評価する。
    現行Windows x64配布の差し替えは不要と判断し、Cargo manifest／lockやalert状態は変更していない。
- [x] Gitleaks 0 findingsをV1 snapshotとartifact生成commitで再確認する
  - 2026-09-02にRC固定commit `577a9f18bb4f0dd76a112a7d09d015877d5dedb9`の359-file source snapshotを
    Gitleaks 8.30.1で検査し、0 findingsだった。検査reportとrelease planは同じRC準備directoryへ保存した
- [x] IPC command、filesystem path、credential、MCP tokenの境界回帰を通す
  - 2026-09-02にcommit `9e51ec4`で`cargo fmt --all -- --check`と
    `cargo test --workspace --locked`をローカル実行し、336 passed／0 failed／24 ignoredだった。
    ignoredは実account、network download、実helper、Windows資格情報などの明示実行専用試験。
  - 固定command分類と確認scope、helper IPCのsize／shape、workspaceの絶対path／`..`／junction／hard link、
    credential target injection、MCP bearer tokenの上限・完全一致・redactionを含む境界試験が合格した。
    GitHub Actionsや外部Provider送信は使用していない
- [x] Windows native binaryに実際に含まれるdependencyのlicenseとnoticeを再生成・確認する
  - 2026-09-02、Windows x64の`moe-desktop`／`moe-command-helper` normal dependency 315件、
    frontend production dependency 4件、Pixelify Sansを対象に`THIRD-PARTY-NOTICES.txt`を再生成した
  - license metadata欠落は0件。package rootにlicense／notice fileがない12件は、宣言済みlicense identifierと
    package情報を生成物へ明記した
  - `scripts/generate-third-party-notices.ps1 -Check`の再現性、installer buildでのstale拒否、
    NSISのlicense pageとinstall先へのnotice同梱・uninstall時削除を確認した
- [x] security policyの対応version、連絡経路、修正方針をV1向けに更新する
  - 英語版／日本語版に、最新V1安定版とRCの対応範囲、旧alphaの非対応、Private Vulnerability
    Reportingへの直接導線、利用不能時に詳細を書かず連絡を求める経路を明記した
  - 最新対応versionを基本とする修正方針、外部Provider／CLIとの切り分け、協調開示、SLAとbug bountyが
    ないこと、善意の調査境界を明記した

### 7. 利用者向け文書を安定版へ揃える

- [x] 英語READMEと日本語READMEを`v1.0.0`の実装・配布内容へ更新する
  - V1は準備中で未公開、sourceは`1.0.0`、ローカルinstallerは未署名検証用であることを冒頭に明記した
  - Codex workspace、固定commandと確認、model選択、turn停止、生成画像、backup、V1のProvider境界と
    非対応範囲を現在の実装へ更新した
  - installerのlicense／notice同梱と、`prepare-public-release.ps1`を使うV1準備手順へ更新した
- [x] install、初回起動、Provider CLI導入、login、update、uninstallを短い手順にまとめる
- [x] 各Providerへ送られるデータ、課金、保持、利用規約の境界を説明する
- [x] backup場所、復元方法、data保存場所、uninstall後に残るdataを説明する
- [x] 対応OS、必要なWebView2、既知の制約、非対応機能を明記する
- [x] troubleshootingにCLI未検出、未login、timeout、SmartScreen、WebView2を載せる
  - 2026-09-02、英語／日本語の`docs/USER-GUIDE*`へ、Windows 11 x64のV1利用手順、手動update、
    Room backupの対象と非対象、端末内data path、Provider送信・課金・保持境界、uninstall後に残るdata、
    代表的な問題の確認手順をまとめた
- [x] `CHANGELOG.md`、release notes、support policyをV1向けに更新する
  - `CHANGELOG.md`のRC残作業を現状へ更新し、英語／日本語のsupport policyとV1 Release notes Draftを追加した
  - Release notesの最終hash、署名者、size、download linkはRC値を流用せず、artifact固定後にだけ追記する
- [ ] レーベルサイトのM.I.O.ページにV1 download、checksum、署名確認方法を掲載する
- [x] BOOTH／itch.ioの配布方式をStore誘導または直接downloadからOwnerが選び、取得方法、hash、
      署名説明を最終確認する
  - 2026-09-05にBOOTH item `8807279`を0円・BOOST任意、itch.io game `4974122`を
    `$0 or Donate`／suggested donation `$2.00`として、それぞれ非公開Draftで保存した
  - BOOTHは日本語本文、itch.ioは英語本文、各ページへcoverと5 screenshotを登録済みである。
    この時点ではStore公開前のため実行file／正式Store linkは未登録で、両ページとも公開していなかった
  - 2026-09-05、Ownerの個別承認後に、BOOTHへ未署名Preview installerと日英README／checksumを
    含むZIP、itch.ioへ同じinstaller EXEをuploadした。BOOTHは対象check付きで下書き保存し、reload後も
    fileを保持した。さらにBOOTH本文へZIP／installerのhash、未署名PreviewとSmartScreen、source commit、
    backup／install手順を反映して下書き保存した。itch.ioはfile固有Windows platformを選択し、download
    非表示を解除して、同じ説明と英語install手順をDraft保存した。reload後もfile、設定、本文、Draft状態を
    保持していた。両pageは非公開で、公開直前の最終確認とOwner承認が残るため項目は未完了のままとする
  - 同日のBOOTH公開前previewで見つかった初期templateの4 dummy段落をOwner承認後に削除し、下書き保存した。
    reload後のpreviewでdummy文が消え、配布ZIP、未署名説明、ZIP／installer hashが保持され、非公開のまま
    であることを確認した
  - 2026-09-06、Ownerの明示承認後にBOOTHとitch.ioの公開ページを`v1.0.3`へ更新した。BOOTHは
    `M.I.O_1.0.3_windows-x64_unsigned-preview.zip`（4,504,270 bytes、SHA-256
    `C0F8AF6B135BA1CC9EA5E83586FE7617A1CAAD17F89A97863E5321412DB6FC9F`）、itch.ioは
    `M.I.O_1.0.3_windows-x64_unsigned-preview_setup.exe`（4,520,172 bytes、SHA-256
    `F9EF0CB56255216B82C92F7D9754FCF638A0B758BADEB9230C5BFD148ED034DE`）だけをdownload対象にした。
    旧`1.0.0` fileは削除せず非表示で保持した。両ページへ`v1.0.3`の更新履歴を追加し、無料または
    任意支援、未署名説明、source commit `7ff8b654cfa2a60c97c76e509b940d9993fdb8e6`を確認した。Microsoft Store
    packageやSubmissionには触れていない
  - 2026-09-07の`v1.0.4`公開記録を、2026-09-08に非login browserの公開ページで再確認した。
    BOOTHのdownloadは`M.I.O_1.0.4_windows-x64_unsigned-preview.zip`（4,500,699 bytes、掲載SHA-256
    `06D8F4262DBAE3BF1D43590D6AB1E38232EAF452528BF9163F1D1F32D218F9A1`）、itch.ioは
    `M.I.O_1.0.4_windows-x64_unsigned-preview_setup.exe`（4,516,598 bytes、掲載SHA-256
    `5BEBA29831175941B8977DC9E1F7FC0BB3B75B91A01922C88EEF7919330C7B19`）だった。
    両方ともsource commit `fbc81b89ee64a670f13128487c06a96bfabe3238`、無料／任意支援、未署名Previewを
    案内していた。この確認時点ではitch.io本文のRelease notesに`v1.0.4`が欠けていたが、
    同日の後続作業で7項目を追記・保存し、公開反映を確認した。既存の`v1.0.3`履歴とdevlogは保持した。
    実downloadと取得物の再hash照合は未実施である

### 8. V1を公開する

- [x] root、desktop、Tauri、Rustのversionを`1.0.0`へ揃える
  - 2026-09-04にroot／desktopの`package.json`、Tauri configuration、Cargo workspaceを`1.0.0`へ
    揃え、npm／Cargo lockfileをオフラインで更新した。現在のREADME、CHANGELOG、license notice表記も
    正式V1候補へ揃え、過去のRC install／試験記録と提出済みStore packageの`1.0.0-rc.1`証拠は変更していない
  - 版番号7か所の完全一致、third-party notices、desktop typecheck、frontend production build、
    `cargo fmt --all -- --check`、`cargo test --workspace --locked`を確認し、すべてexit code 0だった
  - bundleなしのTauri release buildと`moe-command-helper`のrelease buildも成功した。生成した
    `target/release/moe-desktop.exe`はFileVersion／ProductVersionとも`1.0.0`、SHA-256は
    `D1F6BE0290FC09C17AFA2BC330CDC4CFC7C6C106508F59B5E7F57491CFAC7147`だった。helperはCargo package
    `1.0.0`としてbuildされ、SHA-256は`2D29C49E2DA81934B5057E02507ADF6487CBAF1483F2B0C55A6A3BD0B28EBC90`だった
  - この確認はsource版番号と単体release buildの確認であり、helper hash埋込み、installer／MSIX作成、署名、
    install、Store package差替え、再提出は行っていない
- [ ] V1 source snapshot、source ZIP、installer、checksumを同一commitから生成する
  - Store提出候補については、上記commitから376 filesのsource ZIPと`1.0.1.0` MSIXを生成し、
    SHA-256一覧まで固定した。自前署名installerを含む一般配布一式は未確定なので、この項目は未完了のままとする
  - 2026-09-05、commit `dcb90b8a029146506eb5b638931e5b19384a7194`から385 filesのsource ZIP、
    未署名Preview NSIS、manifest、`SHA256SUMS.txt`を
    `.tools/public-release-prep/1.0.0/dcb90b8-direct-unsigned-preview-20260905-185037/`へ固定した。
    source ZIPは7,515,518 bytes／SHA-256
    `3D3B555A5C929D6282CA3E8FE10AA74DB058E3E3EEBDCF83167271C7612D5A2C`。これはBOOTH／itch.ioの
    直接配布を選んだ場合のPreview候補であり、自前署名済み正式installerではないため項目は未完了とする
- [ ] 公開snapshotのfile list、secret scan、license、READMEを確認する
  - Store提出候補のsnapshotではREADME、README.ja、LICENSE、THIRD-PARTY-NOTICESを確認し、
    Gitleaks 8.30.1は0 findings、未追跡fileと私用の基準画像は含まれていない。一般配布一式の最終確認は公開前に行う
  - 上記`dcb90b8` Preview snapshotはGit履歴と未追跡fileを含まず、Gitleaks 8.30.1で385 tracked files／
    約3.68 MBをscanして0 findingsだった。外部upload前にdownload表示文と署名説明を再確認する
- [ ] public repositoryへnamed-fileだけを同期し、diffを確認する
- [ ] GitHubで`v1.0.0` draft releaseを作り、署名済みinstallerとchecksumを添付する
- [ ] ownerがdraft本文、asset、署名、download後のhashと起動を最終確認する
- [ ] `v1.0.0` tagとreleaseを公開し、公開後は同じtag／assetを差し替えない
- [ ] レーベルサイトから正しいV1 Releaseへ到達できることを確認する
- [x] Store公開後の匿名page、Store取得、Windows 11実機、既存Room data、レーベルサイト掲載と、
      公開直後のIssue、security report、Provider側仕様変更を確認する担当手順を残す
  - `docs/MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md`へ、Store固有の固定情報、承認境界、
    中止条件、実機data保護、package identity確認、最小smoke、site掲載、完了記録をまとめた

## V1に必須としない項目

次は、V1で安全に無効化または非対応と説明できていれば、公開を止めない。

- token streaming UIとProvider turn途中のcancel
- Remote Relay、複数device、複数account
- background automation
- 無制限のConductor round、nested delegation
- macOS、Linux、32-bit Windows
- 自動更新（手動の安全な更新手順が完成している場合）

Fable、Gemini、Grokのworkspace read／writeは、Codex以外もV1で対応すると明示的に決めない限り
必須としない。CodexのRoom workspaceとProviderごとのmodel選択はV1必須とする。

## 推奨する実行順

1. installer smoke記録とこのV1 checklistをcommitして現状証拠を固定する
2. AppContainer workspace隔離spikeを行い、V1の安全な実装方式をADRで決める
3. Codex Room workspaceを実装し、境界試験と製品経路試験を通す
4. Providerごとのmodel選択を実装し、continuityと失敗表示を回帰試験する
5. Provider名、Windows上の製品名／shortcut名を決めて修正する
6. code signing方針を決め、その間にupdate／data保持試験を進める
7. V1 RCを作り、機能・Provider・securityの最終回帰を一巡する
8. 文書、site、release notesをRCの実物へ合わせる
9. 署名済みartifactをdraftで確認し、ownerの最終承認後に公開する

## 現時点の自己評価

製品のRoom、Direct／Conductor、fail-safe試験はV1にかなり近い。一方、M.I.O.を作った
中心目的であるRoom workspaceは、Windows native sandboxのnested junction read／write境界を
満たさなかったため、V1必須課題として隔離方式から完成させ直す。model選択もV1へ含める。

Remote Relayと複数deviceはV1に含めない。これによりcloud service運用を増やさず、local-firstの
workspace協働とWindows一般配布へ開発を集中する。未署名installerをそのまま安定版として配ることと、
alpha.2からのdata保持更新を未検証のままV1と呼ぶことは避ける。
