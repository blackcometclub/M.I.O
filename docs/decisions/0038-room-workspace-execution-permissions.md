# ADR 0038: Room workspace execution permissions

- Status: Accepted
- Date: 2026-08-29
- Depends on: ADR 0019 (Room-scoped Codex workspace), ADR 0028
  (workspace reparse-point boundary), ADR 0034 (device-local AI access
  permissions)

## Context

M.I.O. V1では、選択したRoom workspaceの内側だけをAIが調査・編集できることを
必須とする。一方、workspace folderだけへアクセスを許可しても、`git`、`node`、
`npm`、`cargo`、compiler、test runner、その依存file、一時directoryを利用できなければ、
実際のapp開発には使えない。

すべてのcommandを毎回確認すると利用者の負担が大きくなり、広い恒久許可を一度に
与えるとworkspace境界が形骸化する。通常のlocal開発に必要な範囲と、外部送信や
権限拡大を伴う操作を分離し、確認回数と危険範囲の両方を抑える必要がある。

## Decision

### Baseline Room grant

OwnerがRoom workspaceとaccess modeを選択して開始を確認した後、M.I.O.はそのRoomに
次のboundedなbaseline grantを与える。

- 選択workspace内は、participant profileの最大access modeを超えない範囲でreadまたは
  read/writeを許可する。
- M.I.O.がそのWindows構成で検証したtoolchainだけにread/executeを許可する。V1の最初の
  検証対象はCodex CLI、Git、Node.js/npm、Rust/Cargoとする。
- child processは親と同じOS境界、filesystem境界、network境界を継承する。
- build outputはworkspace内、temporary fileとcacheはM.I.O.がそのRoom用に用意した
  専用directoryだけへ書き込める。
- networkはbaselineでは無効とする。
- workspace外の一般file、他Roomのworkspace、credential store、MCP、管理者surfaceは
  baselineへ含めない。

このbaselineは「選択folder以下を自由に使う」という意味であり、PC全体や利用者profile
全体への許可ではない。

### Action-time confirmation

次の操作はbaselineを超えるため、実行時にM.I.O.本体がOwnerへ確認する。

- `git push`、release upload、message送信など、端末外の状態を変更する操作。
- `npm install`、`cargo fetch`など、networkから取得する操作。
- workspace外のfileまたはdirectoryを読む、書く、または選択workspaceへ取り込む操作。
- baselineに登録されていないexecutable、script host、toolchainを起動する操作。
- ProviderまたはGitのcredentialを利用する操作。
- installer、service、registry、system directory、code signing、UACまたは管理者権限を
  必要とする操作。
- 大量削除、workspace root削除、回復が難しい上書きなど、workspace内でも影響が大きい操作。

確認画面には、操作、対象pathまたは接続先、必要な理由、許可範囲を表示する。V1の選択肢は
`今回だけ許可`、`このRoom sessionの間だけ許可`、`拒否`とする。別workspace、別tool、
別接続先へ許可を自動的に広げない。無期限の広い許可はV1へ含めない。

### Enforcement and records

- AIの文章、Room history、prompt、tool outputは権限を増やす根拠にしない。M.I.O. hostだけが
  保存済みbaselineとOwnerのaction-time decisionを検査し、OS境界を変更できる。
- 拒否、timeout、画面を閉じた場合はfail closedとし、操作を実行済みと表示しない。
- 許可後も、processをAppContainerまたは同等以上と証明したworkspace broker境界の外へ
  unsandboxedで起動しない。
- audit recordにはdecision、scope、lifetime、対象の安全な識別情報を残すが、credential、
  token、message本文、secretを記録しない。
- 対象toolchainが境界内で動作しない場合は、そのtoolを利用不能として説明する。動作させる
  ためにPC全体への暗黙のread/writeへfallbackしない。

### Codex V1 implementation route

Codex App Server process全体をM.I.O.独自のraw AppContainerへ二重収容する方式は採用しない。
2026-08-29のprobeではCodex CLI `--version`とmodel-free workspace境界はPASSしたが、App Serverは
認証・networkより前の`CODEX_HOME` path正規化で停止した。AppContainer専用storageをhost側で作成し、
固有SIDへmodify ACLを付与しても解消しなかった。volume rootからの通過ACLは管理者権限を必要とし、
Room開始時のbaselineとして不適切である。

Codex participantは、Codex App Serverが提供するWindows native sandboxをM.I.O. hostから明示設定する。

- thread／turnの`cwd`はcanonicalize済みのRoom workspaceに固定する。
- sandbox policyは`workspaceWrite`とし、writable rootをそのRoom workspaceとRoom専用TEMP／cacheに
  限定する。networkはbaselineでは無効とする。
- Windows sandbox setupは`elevated`を必須とする。`unelevated`は2026-08-29のlive試験で通常workspace
  read／writeには成功したが、nested junction経由のworkspace外writeを拒否できなかったため、
  V1 fallbackとして有効化しない。
- `elevated` setup成功後も、明示的な`workspaceWrite` contractではnested junction経由のworkspace外
  writeを拒否できなかった。比較したbetaの名前付きpermission profileはwriteを拒否したがreadを
  許した。どちらもこのADRの境界を満たさないため、Codex workspaceの製品gateはOFFのままとする。
- App Serverのsandbox外process APIと`externalSandbox`はV1のworkspace実行経路に使わない。
- M.I.O.はapproval requestをhost-owned confirmationへ変換し、AIの文章だけで承認しない。
- App Server processとM.I.O. clientはtrusted control planeとして扱う。client側のfilesystem APIや
  process APIをpromptから直接呼べるtoolとして公開しない。

この経路も製品採用前に、workspace外の直接path、root／nested junction、sandbox内で作成するreparse
point、child process、network、credential、未登録toolを実Codex turnで検証する。公式contractを
指定しただけで境界合格とは扱わない。2026-08-29時点のCodex CLI `0.149.0`ではnested junction境界が
不合格であり、native sandbox単独をV1実装として採用できない。

### Workspace broker V1 implementation route

native sandboxが境界を満たさない間、M.I.O. hostがfilesystem操作を仲介するworkspace brokerを
段階的に実装する。

- Providerから受け付けるtargetはworkspace-relative pathだけとし、絶対path、prefix、root、`.`、`..`を
  拒否する。
- brokerはrootとtargetのfilesystem metadataを自分で検査し、junction、symbolic link、その他のreparse
  point、複数名を持つhard link fileを拒否する。resolved host pathをProviderやadapterへ返さない。
- file read／writeは検査後のpathを呼出側へ返して行わせず、将来のbroker API内部でbounded I/Oまで
  完結させる。
- arbitrary command、toolchain、network、credentialはfile brokerと別contractとし、境界を実証するまで
  有効化しない。

`moe-workspace-broker` crateはpath contractに加え、`cap-std 4.0.3`のopen directory capabilityを
broker内部に保持する。公開するfile operationは最大1 MiBのread、新規作成、同一directory内のtemporary
fileを使う既存file置換だけとする。Windowsではnested junction越しのread／createをpath preflightと
capability I/Oの両方で拒否する回帰試験を持つ。

path／file I/O tranche単独の時点ではProvider、Room、Codex、UIへ接続しなかった。arbitrary commandの
安全境界も未実装であり、これだけを理由にworkspace製品gateをONにしない。

2026-08-29の次段階では、Codex App Server `0.149.0`のthread-scoped `dynamicTools`を使い、
`item/tool/call`をM.I.O. hostが直接処理する接続contractを実装した。Codexへ渡すのは
`mio_workspace.list_directory`／`read_file`、read-write時だけ追加する`create_file`／`replace_file`と、
それぞれのstrict input schemaだけである。選択workspaceのhost pathはthread `cwd`、sandbox writable root、
prompt、tool resultのいずれにも渡さない。Codex processの`cwd`とnative sandboxは空のM.I.O. runtime
directoryをread-onlyで使い、workspace file I/Oはbroker内部だけで完結する。

dynamic toolで扱うUTF-8 textは128 KiBまでとする。これにより制御文字がJSON escapeで最大幅に膨らんでも、
App Serverの1 MiB JSONL framing上限を越えない。

実Codex turnで通常fileのread／create／再readが成功し、nested junction内のsecret readと外側へのcreateが
拒否されることを確認した。read-only accessからwrite toolを呼ぶ要求、未知tool、別thread／turn、余分な
argumentはfail closedする。現行Desktop Codexを古いnpm版より先に選ぶが、明示的な`MOE_CODEX_BIN`／
`MOE_CODEX_CLI_JS` overrideは引き続き最優先とする。

brokered dynamic tools経路は、folder消失、root link、broker初期化失敗、Windows ACL拒否を
Provider起動前の確定失敗として固定した後、Codex限定でsource上の製品gateへ接続した。Roomのfolder選択と
Codex participant profileの`会話のみ`／`読取り`／`読取り・編集`だけを有効にし、他Providerはchat-onlyを
維持する。実画面ではCodexの2つのworkspace modeが選択可能で、Geminiでは無効かつ未対応と表示され、双方で
command／Webが禁止と表示されることを確認した。2026-08-29の製品UI試験では、Codexが選択workspaceの
`seed.txt`を読み、`result.txt`を新規作成して再読取りし、host側でも両内容の完全一致を確認した。Roomと
participantの権限は試験後に`会話のみ`へ復元した。

同日のdirectory listing trancheでは、workspace rootを`.`、子directoryを相対pathで指定する1階層一覧を
追加した。最大256 entryかつ名前合計64 KiBまでに制限し、名前と`file`／`directory`／`blocked_link`／`other`
だけを整列して返す。linkは名前だけを`blocked_link`として示して追跡せず、link自体をdirectoryとして開く要求は
拒否する。実Codex App Serverの製品adapter試験で、root一覧、既存file読取り、新規file作成、再一覧、再読取りが
一巡して成功した。action-time confirmationと代表的なbuild／test commandを扱うcommand brokerは引き続き
別trancheで実証するため、V1完了とはまだ扱わない。

同日のpreflight trancheでは、保存済みworkspaceが送信直前に消失、link化、またはbrokerからopen不能に
なった場合をworkspace固有の確定失敗とした。M.I.O.はCodex processを起動せず、dispatch ledgerを
`failed`で終端し、`unknown`や自動再送候補へ移さない。UIはfolderの再選択または会話のみへの復帰を案内し、
providerへ届いた可能性があるとは表示しない。workspace検査とCodex起動の間に状態が変化してbroker初期化が
失敗した場合も、adapter contractの`WorkspaceUnavailable`で同じpreflight経路へ戻す。

Windows ACLによる権限拒否は、Windows一時directoryの継承を外してEveryoneへのfull-control deny ACEを
設定する実fixtureで固定した。deny前はbrokerを初期化でき、deny後はdirectory listingが
`PermissionDenied`、broker初期化が`WorkspaceUnavailable`となる。fixtureはtest終了時にdeny ACEを除去して
削除する。dispatch側ではこのadapter preflight errorを`failed`で終端し、`unknown`や自動再送へ送らない。

同日のcommand contract trancheでは、`moe-command-broker`をfile brokerとは別のprovider-neutral crateとして
追加した。AIからshell文字列、実行file path、任意argumentを受け取らず、Gitのstatus／diff／logと、標準名の
npm／Cargo build・test・typecheck等だけを列挙したintentとして分類する。baseline planはnetwork無効、shell
command文字列不可、出力最大256 KiB、最長300秒を必須条件とし、read-only workspaceではbuild outputを伴う
intentを拒否する。`git push`、package取得、未登録tool、credential、管理者操作、破壊的操作はbaseline planを
生成せずaction-time confirmation対象として分離した。

続くconfirmation trancheでは、M.I.O. hostが発行するrequest IDと、Room、session、workspace、操作種別、
credentialを含まない対象表示を完全一致scopeとする確認状態contractを同じcrateへ追加した。`今回だけ許可`は
実行直前に一度だけ消費し、scope不一致ではauthorizationを失効させる。Room session許可も同一scopeだけへ適用し、
session終了時にpending request、未消費authorization、session grant、同sessionの終端記録を除去する。
拒否、dismiss、timeout、不正なID、不正な対象表示、未登録tool、credential、管理者操作、破壊的操作への
session許可はfail closedとする。未登録tool、credential、管理者操作、破壊的操作は今回だけの許可に限定する。
pendingと未消費authorizationは合計32件、終端request IDは1024件までに制限し、上限到達時は新しい許可を
生成しない。

このcrateは分類とhost-owned確認状態だけを担当し、executable解決、process起動、確認UI、turn待機、永続保存は
行わない。安全な隔離runnerとhost-owned確認UIが接続されるまではCodexへcommand toolを公開せず、command製品
gateはOFFのままとする。

desktop hostには、hostがregistryへ登録したpending requestをRoom別に読むcommandと、Ownerの回答を返すcommand
だけを追加した。UIへ返すのはrequest ID、Room ID、操作種別、理由、credentialを含まない対象表示、Room session
許可を選べるかどうかに限定し、内部のsession IDとworkspace keyは返さない。回答時はrequest IDが指定Roomの
pending requestであることをhost側で再検査する。UI側からrequest、scope、timeout、authorization消費を作成する
commandは公開しない。このbackend境界はまだturn、確認画面、runnerへ接続しておらず、製品動作は変わらない。

frontend trancheでは、AI応答待ちの間だけ選択Roomのpending requestを読み、完全一致するJSON shape、列挙済みの
操作と理由、actionとreasonの対応、Room ID、重複しないrequest ID、対象表示の上限を再検査するbridgeを追加した。
pending requestがある場合は背景のRoom画面を`inert`にし、拒否へ初期focusした`alertdialog`で操作、理由、安全な
対象表示を示す。Room session許可はhostがeligibleと返した操作だけに表示し、Escまたはcloseはdismissとしてhostへ
回答する。回答に失敗した場合はdialogを閉じず、操作は許可されていないと表示する。

このfrontend tranche単独では内部runnerのrequest登録、turnのwakeup、timeoutへ接続していない。実requestを
使う製品経路試験も未実施であり、command製品gateはOFFのままである。

host内部のwaiting trancheでは、内部callerだけが確認requestをregistryへ登録し、condition variableでOwnerの
回答を待つ境界を追加した。frontendからの回答は同じhost stateでresolutionを記録してwaiting callerを起こし、
許可時はcallerへ返す直前に完全一致scopeのauthorizationを一度だけ消費する。拒否、dismiss、host-owned timeoutは
authorizationを返さず、Room session終了はpendingまたは回答直後のwaiterを起こしてfail closedする。同じrequest ID
で複数waiterを作ることは拒否する。製品callerへtimeout値は渡させず、host内部の120秒へ固定する。

このwaiting境界はまだCodex dynamic tool、Room turn、runner、process起動へ接続していない。Room session IDの
生成・終了契約もhost側で固定する必要があるため、command製品gateはOFFのままである。

続くworkspace identity trancheでは、保存済みのcanonical pathだけをRoom sessionのscopeとして再利用しない。
Windowsではdirectory handleからvolume serial numberとfile index、Unixではdevice IDとinodeを読み、host内部だけの
ASCII workspace keyを生成する。同じpathのfolderを別objectへ置換するとkeyが変わる回帰試験を固定した。keyは
Provider、frontend、prompt、Room historyへ公開せず、file内容やcredentialも含まない。このtranche単独ではまだ
Room sessionを開始せず、runner、process、command tool、製品gateも有効化しない。

host-owned Room session trancheでは、desktop hostがprocess内sequenceからsession IDを生成し、選択Roomと上記
workspace keyへ結び付ける。同じRoomでsessionを開始し直すと旧sessionのpending request、未消費authorization、
session grantを終了させ、待機中callerもfail closedで起こす。終了入力は内部session IDではなくRoom IDとし、
frontendやProviderにsession ID／workspace keyの生成を許さない。このcontractはまだ製品のRoom open／close、
dynamic tool、runner、processへ接続しておらず、command製品gateはOFFのままである。

確認requestの製品caller向け入口も、activeなhost-owned Room session、列挙済み操作、credentialを含まない対象表示
だけを受け、request IDと完全一致scopeをdesktop hostが生成する形へ限定した。request直前にも保存済みworkspaceを
filesystem identityで再検査し、古いsession、別Room、別workspace key、同じpathのfolder置換はpending requestを
登録する前にfail closedとする。request ID、session ID、workspace key、timeoutのいずれも
frontend、Provider、prompt、Room historyから指定できない。この入口もまだdynamic toolやrunnerへは接続しない。

active Room lifecycle trancheでは、frontendがactive Room IDだけをdesktop hostへ通知する。hostはRoomの存在と保存済み
workspaceを再検査し、既存の全command sessionを先に終了してから、workspace modeのRoomだけに新しいsession IDと
workspace keyを内部生成する。chat-only、workspace消失、link化、open不能、Room切替では旧sessionをfail closedで
終了する。workspace再選択とchat-only化の完了後にも再同期する。frontendへ返すのはRoom IDとactive booleanだけで、
内部IDやpathは返さない。この接続もcommand tool、runner、process起動は有効化しない。

runner contract trancheでは、`moe-command-runner`を独立crateとして追加し、command brokerが生成したbaseline plan
だけをGit／npm／Cargoの固定argumentへ変換する。host環境を継承せず、stdinは閉じ、出力256 KiB、timeout 300秒を
維持する。npmとCargoのoffline設定も固定するが、環境変数だけでnetwork拒否を証明したとは扱わない。実行backendには
workspace filesystem境界、OS-level network拒否、clean environment、child process containmentの全条件を要求する。
このtrancheはexecutableを解決せずprocessも起動しない。AppContainerまたは同等以上のbackendと実fixtureが通るまで
command tool、runner製品経路、command製品gateはOFFのままである。

2026-08-30のAppContainer runner fixtureでは、一意なtemporary Git repositoryだけを対象に、command brokerが
分類し、runner contractが生成したGit status planを実際のchild processへ接続した。hostが解決した固定
`git.exe`を使い、shell文字列、任意argument、stdin、credential、継承環境は渡さない。同じAppContainerから
host localhost listenerへの接続、workspace外の直接read／write、nested junction越しのread／writeも拒否された。

通常のuser profile配下ではAppContainerがcurrent directoryのancestorを走査できずGit起動に失敗したため、
volume rootやuser profileのACLは広げず、未使用drive letterへ対象fixtureだけを一時的に割り当ててdrive rootとした。
既存drive letterは再利用せず、run終了時にmapping、固有SIDの一時ACE、AppContainer profileの削除を確認した。
この結果は固定Git read-only baselineのOS隔離backend候補を証明するが、製品runnerには未接続であり、npm／Cargo、
Git write、実Room workspace、Windows 10をまだ証明しない。command製品gateはOFFのままとする。

同fixtureの次段階では、AppContainer childをsuspendedで作成し、kill-on-close付きWindows Job Objectへ収容してから
開始することで、収容前にchild processを作られるraceを閉じた。stdoutとstderrは共有上限までだけ保持し、超過分を
memoryへ追加せずpipeをdrainし続ける。64 KiB出力に4 KiBの試験上限を与えるfixtureと、5秒sleepに100 msの
試験deadlineを与えるfixtureがともにPASSし、終了後に試験processとtemporary driveが残っていないことを確認した。
製品用AppContainer host launcherとdesktop経路へ同backendを接続する作業は未完了である。

同日の次trancheでは、固定Git processの組み立て、AppContainer token確認、Job Object所属確認、環境非継承、
256 KiB出力上限、300秒timeoutを`moe-windows-command-runner`へ切り出した。外側のspike hostが生成した
AppContainer childからこのcrateを使用し、固定Git status、workspace境界、network拒否、Job Object、出力上限、
timeoutがすべてPASSした。証拠binary SHA-256は
`A181EA9194925B36EB7EED9FEE4C1DFF41225D39E608595D856E82780A5C1E78`である。

この内部crateは任意のshell文字列を受けず、現時点ではhostが解決した`git.exe`と固定baseline planだけを受け付ける。
ただしAppContainer profile生成、network capabilityなしの起動、workspace ACL、temporary driveは外側のhost launcherが
担当しており、crate単体は完全な隔離境界ではない。desktop製品経路、npm／Cargo、実Room workspace、Windows 10を
まだ証明していないため、command製品gateはOFFのままとする。

helper request protocol trancheでは、desktop hostから隔離helperへ渡せる内容をversion、列挙済みbaseline command ID、
相対working directory、host-owned access区分だけに限定した。requestは16 KiB以下で、workspace絶対path、executable、
shell文字列、任意argument、credential、環境変数を含めない。helper側は未知field、未知version、上限超過を拒否し、
`moe-command-broker`で同じ要求を再分類できた場合だけrunner planを得る。

AppContainer fixture `2A40C736BC9256477AC6AB9BE8DC5C26E7CBDEB537A4D4D2C899F61ACF94FA36`では、このrequestを
一回限りの匿名pipeで渡した。desktop hostだけがwrite端を持ち、`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`でchildへ
read端一本だけを継承した。requestはworkspace fileやcommand lineへ載せず、childが上限付きdecodeと再分類をしてから
固定Git statusを実行し、既存のworkspace境界、network拒否、Job Object、出力上限、timeoutをすべて維持した。
専用helper binaryとdesktop接続は未実装であるため、command製品gateはOFFのままとする。

次のtrancheでは、固定requestのdecode、broker再分類、Windows runner呼び出しを独立binary
`moe-command-helper.exe`へ移した。helperはstdinがWindows pipeの場合だけrequestを受け、起動引数はhostが解決した
絶対`git.exe` pathだけに限定する。相対tool path、別名の実行file、余分な引数、通常processからのrunner実行は拒否する。

AppContainer fixtureでは、probe childから専用helperを同じJob Object内の別processとして起動し、匿名stdin pipeで
固定requestを渡した。helper `0E25A57AA907F5C4357065AFBF06C178AD3F925BFC9A21ACCC470299D0C33B3B`、probe
`78AAB556C82E0314B9D8985C968C30CD1319E217D6AD37F54FCB099A2FF5EECC`で、専用helper process、固定Git status、
workspace境界、network拒否、Job Object、出力上限、timeoutと後始末がPASSした。desktop製品経路、npm／Cargo、
実Room workspace、Windows 10は未検証であるため、command製品gateはOFFのままとする。

製品同梱trancheでは、Windows x64 NSIS検証installerのbuild前に専用helperを同じtarget tripleとstatic CRT設定で
release buildし、Tauriのexternal binaryとして同梱する経路を追加した。2026-08-30のinstaller
`2C4144FEB2551341046DAD83BFB822D1198DD6EADD2FCADA4A7A68B86893E7DD`を展開して得たhelperのSHA-256は、build元と同じ
`8C0BEEF811CA2C7773C4F70E73B2893578E336C7F236B8BE0FF655C2213F62E3`だった。同梱だけではprocessを起動せず、
Codexへcommand toolも公開しない。desktop host launcherと製品経路試験が完了するまでcommand製品gateはOFFとする。

続くbundled-helper identity trancheでは、installer buildが計算したhelper SHA-256をdesktop EXEへcompile-time値として
埋め込む。desktopは自分自身のcanonical install directoryにある正規名`moe-command-helper.exe`だけを調べ、reparse
point、複数hard-link、32 MiB超過、hash不一致、確認後のfile identityまたはhash変更を拒否する。PATH、環境変数、
Provider、frontendからhelper pathや期待hashを指定できない。

最終検証installer `FC106C4AA93172F5A4F7719CA24C462BBEA0621F4DA696741E469A9B5F2504D6`を展開したhelperとbuild元は
SHA-256 `8C0BEEF811CA2C7773C4F70E73B2893578E336C7F236B8BE0FF655C2213F62E3`で一致し、desktop EXEにも同じ期待値が
埋め込まれた。desktop起動時は`Ready`、`NotBundled`、`Rejected`の内部状態を作るだけでprocessを起動せず、
欠落または改変時にも会話機能は起動できる。command toolとcommand製品gateはOFFのままとする。

desktop command preparation trancheでは、activeなhost-owned Room sessionと保存済みworkspaceを実行準備のたびに
再取得し、filesystem identityがsession開始時と一致することを確認する。検証済みhelperを再検証し、brokerが許可した
列挙済みbaseline planを固定helper requestへencodeする。現時点の製品準備対象はread-only Git baselineだけで、
npm／Cargo、read-only workspaceでのbuild、inactive Room、helper欠落／置換は準備段階で拒否する。

この契約はworkspace絶対pathとhelper pathをdesktop内部のbackend入力として保持するが、Provider、frontend、prompt、
Room historyへ返さない。まだAppContainer backend、process起動、Codex dynamic toolへ接続していないため、
command製品gateはOFFのままとする。

desktop Git executable identity trancheでは、PATH検索やProvider／frontend指定を使わず、`ProgramW6432`、
`ProgramFiles`、user-local Programs配下の正規`Git\\bin\\git.exe`だけをhost内部候補とする。install rootから実行fileまでの
directoryとfileにreparse pointがなく、fileが単一hard-linkである場合だけ、filesystem identityとSHA-256を起動時に保持する。
command準備時に同じpath、identity、hashを再検査し、欠落、置換、別名はAppContainer起動前に拒否する。

この検査は既知pathのfileがGit for Windowsの正規配布物であることやpublisher署名を単独で証明するものではない。
V1の検証toolchain／署名方針は別途固定する。現段階ではprocessを起動せず、Codexへのcommand tool公開とcommand gateは
OFFのままとする。

Windows command host launcher contract trancheでは、外側の隔離launcherが受けられる入力を、検証済みの絶対workspace、
正規名`moe-command-helper.exe`、正規名`git.exe`、上限内で再decodeできるGit helper requestだけへ限定した。
AppContainer固有profile、network capabilityなし、workspace限定一時ACL、一時drive、kill-on-close Job、bounded anonymous
request pipe、bounded output、全temporary grant cleanupを全て必須条件として列挙する。

このcrateはまだACL変更、AppContainer profile生成、drive mapping、pipe生成、process起動を実装しない。desktop内部の
prepared commandから同contractを生成するだけであり、Codexへのcommand tool公開とcommand gateはOFFのままとする。

Windows isolation resource lifecycle trancheでは、host launcher内部に一意なAppContainer profile、workspace rootだけへの
一時read／execute ACL、未使用drive mapping、kill-on-close Job ObjectをRAII資源として追加した。外部toolはcanonicalな
`SystemRoot\\System32\\icacls.exe`と`subst.exe`だけをclean environmentで起動し、PATH検索やshellを使わない。明示closeと
途中失敗時DropはいずれもJob、drive、ACL、profileをcleanupする。

この段階ではprofileへnetwork capabilityを付けず、helper process、request／output pipeもまだ作成しない。既存fileへの
ACL継承とcleanupは一時fixtureで確認してから製品実行経路へ接続し、command gateはOFFのままとする。

live cleanup fixtureは通常のCodex sandbox内では`CreateAppContainerProfile`以前に`ProfileUnavailable`となったが、Ownerが
一時ACLとdrive mappingの残留riskを明示承認したsandbox外実行ではPASSした。ACL付与前から存在するfixture fileを一時drive
経由で読取り、明示close後にdrive pathが消え、workspace ACLから固有SID grantが除去され、profile削除とJob closeも成功した。
test終了後のWindows TEMPにfixture directoryは0件、`subst` mappingも0件だった。

このPASSはresource lifecycleだけの証拠であり、AppContainer process、network capabilityなしのtoken、request／output pipe、
helper実行はまだ検証していない。command製品gateはOFFのままとする。

isolated helper process host trancheでは、検証済みhelperをnetwork capabilityなしのAppContainer processとしてsuspendedで
作成し、kill-on-close Job Objectへ割り当てた後だけ開始するhost起動部を追加した。childへ継承するhandleはbounded request
pipeのread端、stdout pipeのwrite端、stderr pipeのwrite端だけをhandle listで指定する。requestはprocess開始前にhostが
上限内で書き終えて閉じ、host環境は継承しない。Unicode environment blockは`SystemRoot`、`SystemDrive`、`NO_COLOR`に加え、
temporary workspace driveのcurrent-directory entryと、SIDからWindows APIで取得したAppContainer専用`LOCALAPPDATA`、
`TEMP`、`TMP`だけを渡す。

hostも共有256 KiB output上限とhelper上限より10秒長い固定deadlineを持ち、timeout時はJob全体を終了する。process、pipe、
Job、drive、ACL、profileは成功／失敗の双方でRAII cleanupへ入る。初回の実helper fixtureは、AppContainer専用環境項目が
不足し`CreateProcessW`のWin32 error 203でfail closedした。専用profile pathとdrive current-directory entryを追加した後、
helper `0E25A57AA907F5C4357065AFBF06C178AD3F925BFC9A21ACCC470299D0C33B3B`から固定Git statusを実行するfixtureはPASSした。
終了後のWindows TEMP fixture directory、`subst` mapping、AppContainer profileはいずれも0件だった。Codexへのcommand tool公開と
command製品gateはOFFのままとする。

desktop pre-launch revalidation trancheでは、準備済みcommandの実行直前にactive Room sessionとworkspace identityを再取得し、
準備時の値と一致する場合だけ続行する。bundled helperとGit executableもfilesystem identityとSHA-256を再検査し、backendの
prepared launchを現在値から再構築してから実行する。workspace、helper、Gitを準備後に差し替える単体fixtureは、いずれも
AppContainer process作成前にfail closedした。

実helperと固定Gitまでを通すdesktop ignored fixtureの初回実行では、公式`Git\\cmd\\git.exe`が`git-lfs.exe`とのhard-linkを
共有していたため単一hard-link検査でfail closedした。安全条件を緩めず、同じ公式install内で単一hard-linkの
`Git\\bin\\git.exe`だけを候補とするよう変更した後、desktop準備から実helper、固定Git statusまでPASSした。終了後の
desktop／launcher fixture directory、`subst` mapping、AppContainer profileはいずれも0件だった。この内部接続は
Tauri command、Codex dynamic tool、frontendから呼べず、command製品gateはOFFのままとする。

host-owned Room command context trancheでは、provider-neutralな`TextTurnRequest`へoptional Room IDを追加し、Codex dispatchを
作るdesktop hostだけが現在のRoom IDを設定する。Room IDはprompt、dynamic tool引数、Provider command lineへ含めず、将来の
command contextがactive Room sessionを再照合するためだけにadapter内部で使う。この段階ではcommand execution contextや
dynamic toolへ接続せず、command製品gateはOFFのままとする。

exact Room session turn context trancheでは、desktop command executionがhost-owned `TextTurnRequest`のRoom ID、workspace
root、participant accessを、現在のRoom command sessionとworkspace identityへ結び付け、不透明なturn contextを作る。
command準備APIはRoom IDやaccessを個別入力として受けず、このcontextだけを受ける。準備時と実行直前にsession ID、
workspace identity、workspace rootを再検査するため、同じRoomとworkspaceでsessionを開始し直しても古いcontextや
準備済み命令はfail closedする。dynamic toolへの公開はまだ行わず、command製品gateはOFFのままとする。

first product command tool trancheでは、Codex App Serverのworkspace turnにactiveなexact turn contextを作れた場合だけ、
`mio_command.git_status`を追加する。input schemaは空objectだけを許し、working directoryはworkspace rootの`.`、commandは
固定`git --no-pager -c core.fsmonitor=false status --short --branch --untracked-files=all`とし、workspace内Git設定から
fsmonitor補助programを起動させない。hostはthread ID、turn ID、call ID、namespace、tool名、空のargumentsを再検査し、
既存の隔離helper／networkなしAppContainer経路だけで実行する。任意command、argument、path、
credential、Git変更操作、npm／Cargoは公開しない。

2026-08-31の実`D:\desktop\M.O.E`製品試験では、turn準備は28.6 msで終わったが、workspace全体へのtemporary ACL付与と
除去を含む実行は約18分かかり、helperはexit code 10、stderr 69 bytesの`not a git repository`で終了した。終了後のroot、
`.git`、`apps`、`target`にAppContainer SID ACEが残っていないことは確認した。小さいC:／D: Git fixtureはPASSしているため
D:一般の不成立ではないが、実workspace相当の安全境界と実用時間は証明できていない。`mio_command.git_status`を含む
command dynamic tool製品gateはOFFへ戻し、同等の実workspace試験がPASSするまで再公開しない。

同日のroot-only ACL follow-upでは、再帰`icacls /T`を使わずworkspace rootへ継承可能なread／execute ACEを1件だけ
一時付与するtest-only候補を作った。D:上の一時Git fixtureへ既存子ファイル256件を置いた固定Git statusは947.3 msでPASSし、
temporary SID ACEも0件だった。しかし実`D:\desktop\M.O.E`では60秒を超えて付与処理が完了せず中止した。中止後に遅れて
反映された今回のroot ACEはexact SIDで回収し、一時profileもexact monikerで削除した。root DACL／Owner、Git index
SHA-256、主要子path、drive mapping、helper／Git processに残存や変更がないことを確認した。root-only候補は製品launcherへ
採用せず、command dynamic tool製品gateはOFFのままとする。

host-owned read-only Git status broker follow-upでは、AppContainerへworkspaceを公開せず、desktop process内の
libgit2からstatusだけを取得する別経路を追加した。`git.exe`、shell、hook、project script、network、credentialは使わない。
libgit2のSystem／ProgramData／XDG／Global設定探索は、desktop内で他にgit2利用がないことを前提に、最初のrepository open
より前に一度だけ空へ固定する。local `.git/config`は事前検査し、include、external ignore／attributes、fsmonitor、filter、
別worktree／objectsを参照できる設定を拒否する。`.git` file、commondir、alternates、config worktreeも非対応とする。
statusはindexをrefresh／更新せず、untracked directoryへ再帰しない。

2026-08-31の実`D:\desktop\M.O.E`試験は60.0 msでPASSした。前後の`.git/index` SHA-256は
`CEE164E5E094DF1B894BB86FF86475F3B4F8A03F8248CFE13B703FEC796AD9A1`で一致し、workspace root ACLも一致した。
Codexへ公開するのはactive Room session、workspace root、filesystem identityを直前に再照合した引数なし
`mio_command.git_status`だけとする。任意command、argument、path、build、test、Git変更操作と従来のAppContainer
command経路は引き続きOFFとする。

fixed Node entry follow-upでは、read-write Roomだけに`mio_command.run_node({ directory })`を追加する。これは上記の
AppContainer command経路全体を再公開する判断ではなく、`mio-main.mjs`だけを実行する狭い例外である。Providerから受けるのは
既存workspace相対directory 1件だけで、command文字列、argument、別entry fileを受け取らない。hostはactive Room session、
workspace identity、directoryとentryの非link境界を準備時と実行直前に再照合する。AppContainerへ見せるworkspaceは選択directoryの
subtreeだけであり、Room workspace root全体ではない。

Node executableは登録済みWindows layoutの`node.exe`をidentity、single hard link、SHA-256で検証・再検証する。Program Filesの
ACLは変更せず、検証済みNodeを1回限りのAppContainer data directoryへcopyする。helperはnetwork capabilityなし、clean environment、
null stdin、bounded output／timeout、kill-on-close Jobの下で固定entryを実行する。終了時はworkspace ACE、temporary drive、staged Node、
AppContainer profileを回収する。

実helper＋実Nodeの小さいTemp fixture、およびhost user所有の`target/mio-node-host-smoke`はPASSした。後者は5,691 bytesの
`mio-mini-app.html`を読み、SHA-256
`BBEC06F7A58BBE73EF85390C9ADC008BB6F9E338890BA3D873C6A9B878C7A034`を結果JSONへ保存した。前後でOwnerは
`CLUT\black`、temporary AppContainer ACEは0件だった。Codex file sandbox所有のfixtureはWindows hostがACLを変更できず安全側に
失敗したため、製品fixtureはhost user所有で用意する。package install、任意Node argument、npm／Cargo、shell、Git変更操作は
未公開のまま残す。

exact npm package install follow-upでは、read-write Roomだけに
`mio_command.install_npm_package({ directory, packageName, exactVersion })`を追加した。対象は既存`package.json`を持つ
workspace相対directory、lowercase npm package名、厳密なSemVer 1件だけである。tag、range、embedded version、URL、local path、
Git source、command文字列、追加optionはdynamic tool schema、broker、16 KiB以下のhelper protocolで重ねて拒否する。
実行直前にはhost-owned confirmationへcredentialを含まない`package@version in directory`を表示し、許可待ちの前後でRoom session、
workspace identity、directory、`package.json`、helper、Node、npm runtimeを再検査する。

Windowsの`npm.cmd`とshellは使用しない。登録済みNode layoutの`node_modules/npm`をfile 4,096件、directory 1,024件、
各file 16 MiB、合計64 MiB以下へ制限し、reparse point、複数hard-linkを拒否してtree hashを保持する。実機のnpm runtimeは
1,770 files、444 directories、10,517,885 bytesで境界内だった。検証済みNodeとnpm runtimeは一回限りのAppContainer data
directoryへcopyし、そのdirectoryにも別のtemporary driveを割り当てる。これによりvolume rootやuser profileのACLを広げず、
Nodeのrealpath検査をstaged tool drive内で完結させる。

package installだけはWindows AppContainerの`internetClient` capability SID 1件を付ける。private network、server、他capabilityは
付けず、既存baselineとfixed Node entryは引き続きcapability 0件である。npm引数は`install`、`--ignore-scripts`、
`--save-exact`、`--package-lock=true`、`--global=false`、`--registry=https://registry.npmjs.org/`、audit／fund無効、
選択directoryをhostが付ける`--prefix`、検証済み`package@version`だけへ固定する。clean environment、null stdin、256 KiB output、
300秒timeout、kill-on-close Jobも維持する。

2026-08-31のsandbox外実境界試験では、host user所有の一時fixtureだけへ`is-number@7.0.0`を導入した。終了コード0、exact dependency、
`package-lock.json`、`node_modules/is-number/package.json`を確認してPASSした。試験用fixture、temporary ACL、workspace／tool drive、
staged Node／npm、npm cache、AppContainer profileはrun終了時に回収した。任意Node argument、任意npm command、Cargo、shell、
Git変更操作は引き続き公開しない。

追加のowner境界では、fixed Node entryとexact npm package installの実行directoryを現在のWindows user所有に限定する。
`GetNamedSecurityInfoW`で取得したowner SIDとprocess token user SIDを比較し、確認画面より前、確認完了後、helper実行直前に再検査する。
owner取得不能、別owner、途中変更は専用errorで安全側に拒否する。通常user所有の製品UI fixtureでは`is-number@7.0.0`導入と
`package.json`／`package-lock.json`読戻しがPASSした。一方、`CLUT\CodexSandboxOffline`所有の旧fixtureは確認画面より前に拒否した。
これにより、Windows error 267としてしか見えなかった所有者境界不成立を、実行や再送の前に区別できる。

同日のturn cancellation trancheでは、provider-neutralな`TextTurnCancellation`をDirect turn、Conductorのplan／synthesis、
各native workerへ渡す。AI応答待ちの送信buttonは同じ位置の停止buttonへ変わり、Ownerが停止するとM.I.O.はactiveなRoom turnへ
停止信号を送り、CLI／App Server childを終了する。送信済みRoom messageは削除せず、Providerへすでに届いた可能性を表示して
自動再送しない。停止信号が対象を見つけられない場合も未送信とは断定せず、結果待ちとしてfail closedする。

## Consequences

- 一般的な調査、編集、build、testは、Room開始時の一度の確認後に繰り返し実行できる。
- 外部送信、download、credential利用、権限拡大は、影響が分かる時点でOwnerが判断できる。
- 対応toolchainごとに実行file、依存file、cache、temporary pathを調査し、境界回帰を持つ
  必要がある。
- 任意のWindows開発環境をV1で保証せず、検証済みtoolchainと既知の制約を公開文書へ明記する。
- AppContainerとの互換性を理由に境界を緩めることはできない。不成立の場合は、同じpermission
  contractを保ったworkspace broker方式を採用する。
