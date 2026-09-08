# Windows AppContainer workspace boundary spike

M.I.O. V1のRoom workspace候補として、Windows AppContainer processが明示した一時workspaceだけを
read／writeできるかを、外部AI、credential、実project dataを使わずに測る隔離PoCです。

```powershell
cargo run -p moe-windows-appcontainer-workspace-spike
```

probeはrepositoryのignored `.tools/appcontainer-workspace-spike/` 以下へ実行ごとに一意なfixtureを
作り、AppContainer profileも一意な名前で作成します。fixtureは証拠として削除せず、AppContainer
profileだけを終了時に削除します。

確認する境界:

- 許可workspace内のfile read／write
- workspace外への直接read／write拒否
- workspace内のnested junctionからworkspace外へのread／write拒否
- AppContainer内からjunctionを作成した場合の範囲外read拒否
- AppContainerから起動した`cmd.exe` child processによるworkspace内write
- network capabilityを持たないAppContainerからlocalhost listenerへ接続できないこと
- hostが解決した固定`git.exe`による`status --short --branch --untracked-files=all`
- AppContainerからの`git`、`node`、`npm`、`cargo` version process起動
- 明示したCodex CLI executableのfixture copyを使う、認証・通信なしの`--version`起動

このprobeは従来のAppContainer APIを使う。2026年6月公開の
`Experimental_CreateProcessInSandbox`はWindows 11限定かつ実験的であるため、V1の唯一の境界には
採用しない。probe合格だけでCodex CLI、認証、network、開発toolchainとの互換性を完成扱いにしない。

一時fixtureだけへAppContainer SIDのACLを付与する。利用者の実workspace、credential、Provider設定、
M.I.O.製品code、Windows Sandbox設定は変更しない。

## 2026-08-29 initial model-free result

- Environment: Windows display version 25H2 / build `26200.9168`
- Probe SHA-256: `FE5B2662C475CED28A6A714063FFBA4756FC4F2CFC5CD0208832F1B7DABEF8A8`
- Result: `BOUNDARY=PASS`
- Child process: `CHILD_PROCESS_COMPATIBILITY=PASS`
- Evidence: `.tools/appcontainer-workspace-spike/run-33492-1787971863028957200/`

AppContainer processは許可workspace内のread／writeと`cmd.exe` child processによるwriteに成功した。
workspace外への直接read／write、既存nested junction経由のread／writeはすべて拒否された。
AppContainer内からworkspace外を指すjunctionの作成も拒否され、範囲外write fileは存在しなかった。
一意なAppContainer profileは終了時に削除された。

これはmodel-freeの最初の境界合格である。Codex CLI、credential、outbound model通信、実toolchain、
Windows 10 compatibilityをまだ証明していないため、製品のworkspace controlは引き続き無効のままとする。

## 2026-08-29 toolchain discovery result

- Probe SHA-256: `8733C29051F0AB3BAEE891A7923C35F9964AEDCDD3630F962EADCA892F930FB7`
- Evidence: `.tools/appcontainer-workspace-spike/run-22320-1787971980378248000/`
- Boundary: `PASS`
- Generic `cmd.exe` child write: `PASS`
- `git --version`: `PASS`
- `node --version`: `FAIL`
- `npm --version`: `FAIL`
- `cargo --version`: `FAIL`
- Overall toolchain compatibility: `FAIL`

GitはAppContainerから起動できた。Node／npmは`C:\Program Files\nodejs`、Cargoは起動中PowerShellの
PATH外にあり、そのままではAppContainerから起動できなかった。AppContainerへ広いfilesystem権限を
与えず、必要なruntimeだけをread-onlyで見せる方式が必要である。これは境界破りではなく、
default-denyがtoolchainにも適用された結果である。

## 2026-08-30 fixed Git baseline and network result

- Probe SHA-256: `705FA57ED8D43AC3053DC4D5432294DF80DDBE091B26FDAEF764F7BC8F0D6013`
- Evidence: `.tools/appcontainer-workspace-spike/run-7148-1788062236113674600/`
- Runner contract applied: `PASS`
- Job Object process containment configured: `PASS`
- Bounded output fixture: `PASS`
- Timeout fixture: `PASS`
- Boundary: `PASS`
- Generic child process: `PASS`
- Network isolation: `PASS`
- Fixed Git baseline status: `PASS`
- Temporary workspace drive removal: `PASS`
- Temporary SID ACL removal: `PASS`
- AppContainer profile deletion: `PASS`
- Overall toolchain compatibility: `FAIL` (`node`／`npm`／`cargo`は未対応)

通常のuser profile配下にあるfixtureをAppContainerのcurrent directoryとして直接使うと、許可していない
ancestor directoryの走査でGit起動が失敗した。volume rootやuser profileへ広いACLを追加せず、未使用の
drive letterへ対象fixtureだけを一時的に`subst`してdrive rootとして見せた。割り当て済みdrive letterは
再利用せず、run終了時にmapping、固有SIDのACE、AppContainer profileをすべて削除した。

このrunでは`moe-command-broker`が分類し、`moe-command-runner`が生成した固定Git status planを使った。
hostが解決した固定`git.exe`だけを起動し、shell文字列、任意argument、credential、networkを渡していない。
Gitのread-only `status`は成功し、同じAppContainerからhostのlocalhost listenerへの接続は拒否された。
AppContainer childは停止状態で作成して`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`付きJob Objectへ収容してから
開始する。64 KiBを出す試験processでは合計4 KiBだけを保持し、超過分をmemoryへ蓄積せず最後までdrainした。
5秒sleepする試験processは100 msの試験用deadlineで停止し、終了後に試験processとtemporary driveが残って
いないことも確認した。

これは一時Git fixtureに対するrunner backend候補の証拠であり、実Room workspace、製品runner、Gitのwrite操作、
npm／Cargo build、Windows 10互換性を完成扱いにしない。

### Reusable Windows runner follow-up

- Probe SHA-256: `A181EA9194925B36EB7EED9FEE4C1DFF41225D39E608595D856E82780A5C1E78`
- Evidence: `.tools/appcontainer-workspace-spike/run-24780-1788063143228442200/`
- Reusable runner contract: `PASS`
- Fixed Git baseline: `PASS`
- Boundary／network isolation／Job Object／output limit／timeout: `PASS`

固定Git processの組み立て、AppContainer token確認、Job Object所属確認、環境非継承、bounded output、
timeoutを`moe-windows-command-runner`へ切り出し、同じspikeのAppContainer childから利用した。
任意のshell文字列は受けず、現時点では`git.exe`の固定baseline planだけを受け付ける。AppContainerの
profile生成、network capabilityなしの起動、workspace ACL、temporary driveは引き続き外側のhost launcherが
担当するため、このcrate単体を完全な隔離境界とは扱わない。desktop製品経路、npm／Cargo、実Room workspace、
Windows 10は未検証であり、command製品gateはOFFのままである。

### Bounded helper request follow-up

- Probe SHA-256: `2A40C736BC9256477AC6AB9BE8DC5C26E7CBDEB537A4D4D2C899F61ACF94FA36`
- Evidence: `.tools/appcontainer-workspace-spike/run-31532-1788064655743523800/`
- Anonymous helper request pipe／decode／broker reclassification: `PASS`
- Fixed Git baseline／boundary／network isolation／Job Object／output limit／timeout: `PASS`

desktop hostから隔離helperへ渡す要求を`moe-command-helper-protocol`として切り出した。version、列挙済みの
baseline command ID、相対working directory、host-owned access区分だけを16 KiB以下のJSONへencodeする。
workspace絶対path、executable path、shell文字列、任意argument、credential、環境変数はwireへ含めない。
helperは受信上限を先に適用し、未知fieldやversionを拒否してから`moe-command-broker`で再分類する。

requestはworkspace fileやcommand lineへ載せず、一回限りの匿名pipeで渡した。desktop hostだけがwrite端を持ち、
`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`でAppContainer childへread端1本だけを継承する。requestを書き終えたhostと
読み終えたchildはそれぞれpipe端を閉じる。証拠用JSON copyはworkspace外のrun directoryへ保存した。
専用helper binaryとdesktop接続は未実装のためcommand製品gateはOFFのままである。

### Dedicated command helper process follow-up

- Helper SHA-256: `0E25A57AA907F5C4357065AFBF06C178AD3F925BFC9A21ACCC470299D0C33B3B`
- Probe SHA-256: `78AAB556C82E0314B9D8985C968C30CD1319E217D6AD37F54FCB099A2FF5EECC`
- Evidence: `.tools/appcontainer-workspace-spike/run-12368-1788066490588421500/`
- Dedicated helper process／anonymous stdin pipe／broker reclassification: `PASS`
- Fixed Git baseline／boundary／network isolation／Job Object／output limit／timeout: `PASS`
- Overall toolchain compatibility: `FAIL` (`node`／`npm`／`cargo`は未対応)

固定命令のdecode、再分類、Git起動を、製品候補の独立binary `moe-command-helper.exe`へ移した。helperの起動引数は
hostが解決した絶対`git.exe` pathだけに限定し、固定requestはstdinがWindows pipeである場合だけ受け付ける。
helper自身もAppContainer tokenとJob Object所属をrunnerで再確認し、固定Git statusのstdout／stderrと安定した
終了codeだけを返す。通常processからの実行、相対tool path、別名の実行file、余分な起動引数は拒否する。

実証ではAppContainer内のprobe childが匿名pipeから最初のrequestを読み、同じ境界内の専用helperへ別のpipeで
再送した。helper processのJob Object所属、固定Git status、既存の全境界項目と後始末はPASSした。desktop製品経路、
npm／Cargo、実Room workspace、Windows 10は未検証であるため、command製品gateはOFFのままである。

## Targeted Codex CLI compatibility mode

既存のCodex設定、認証、install directoryのACLを変更せず、指定したCodex CLI executableを
一時launcherへcopyして`--version`だけをAppContainer内で実行する。

```powershell
$env:MOE_CODEX_BIN = (Get-Command codex).Source
cargo run -p moe-windows-appcontainer-workspace-spike
Remove-Item Env:MOE_CODEX_BIN
```

このmodeでは`CODEX_HOME`を一意なAppContainer profileの専用directoryへ向け、network capabilityを
追加しない。
`CODEX_CLI_COMPATIBILITY=PASS`はbinary起動だけの互換性を示す。login、credential、model通信、
app-server、tool executionを証明しない。Codex指定時はboundary、generic child process、Codex CLIを
required observationとし、Git／Node／npm／Cargoの探索結果は同じrunへ参考記録する。

## 2026-08-29 targeted Codex CLI result

- Codex CLI: `0.149.0`
- Probe SHA-256: `E4042B34653326FC3F81CAB9913524401656F42419CAD6977B5CB3CC4CE8FEA4`
- Evidence: `.tools/appcontainer-workspace-spike/run-15376-1787973852860042900/`
- Boundary: `PASS`
- Generic child process: `PASS`
- Codex CLI compatibility: `PASS`

Standalone版Codex CLI executableをfixture launcherへcopyし、AppContainer内の専用`CODEX_HOME`で
`codex --version`を実行した。`codex-cli 0.149.0`が返り、同じrunでworkspace外への直接read／write、
nested junction経由のread／writeは拒否された。AppContainer profileは終了時に削除された。

この結果はCodex binaryのprocess起動だけを証明する。`app-server`起動、認証、Provider通信、
Codexが起動するtoolへの権限継承は未検証である。

## 2026-08-29 Codex App Server initialize blocker

- Probe SHA-256: `0DC2763A7A78CD9F31C32482DC08F61E3029C9E5226B68E5D34A84BC836D2208`
- Evidence: `.tools/appcontainer-workspace-spike/run-22964-1787974495393890700/`
- Boundary: `PASS`
- Codex CLI `--version`: `PASS`
- Workspace `canonicalize()`: `FAIL`
- AppContainer profile folder `canonicalize()`: `FAIL`
- App Server `initialize`: `FAIL`

Codex App Serverをmodel通信なしで起動し、JSON-RPC `initialize`だけを送るprobeを追加した。
`CODEX_HOME`を許可workspace内、相対path、Windowsが作成したAppContainer専用profile folderへ順に
向けたが、いずれもCodex起動直後のpath正規化が`アクセスが拒否されました (os error 5)`で停止した。
probe自身の`canonicalize()`もworkspaceと専用profile folderの両方で失敗し、Codex認証やProvider
通信より前のAppContainer path互換性blockerであることを確認した。

AppContainerへvolume ancestorの追加ACLや広いfilesystem capabilityを与える方法は、このrunでは
採用していない。通常のworkspace read／write、workspace外とnested junctionの拒否は同じrunで
PASSを維持した。直接AppContainerへCodex App Serverを収容する方式は、境界を広げずにpath正規化を
成立させる方法が見つかるまでV1実装へ採用しない。

## 2026-08-29 dedicated profile storage and ACL cleanup result

- Probe SHA-256: `E34FB652F391B988B9A0AA5340BEDA3BEF8D44080E7513A6E86CED64F1391ABF`
- Evidence: `.tools/appcontainer-workspace-spike/run-24676-1787974995535661100/`
- Boundary: `PASS`
- Generic child process: `PASS`
- Codex CLI `--version`: `PASS`
- AppContainer profile folder `canonicalize()`: `FAIL`
- App Server `initialize`: `FAIL`
- Temporary SID ACL removal: `PASS`
- AppContainer profile deletion: `PASS`

host側で一意なAppContainer専用folderと`codex-home`／`codex-temp`を先に作成し、そのprofile SIDだけへ
一時的なmodify ACLを付与した。Codex `--version`は引き続き成功したが、probe自身とApp Serverの
`canonicalize()`は`アクセスが拒否されました (os error 5)`のままで、専用storageを事前作成する
だけではblockerを解消しなかった。

同じrunでworkspace外への直接read／write、nested junction経由のread／writeを拒否し、境界はPASSを
維持した。終了時にはworkspace、launcher、専用profile folderへ付与した固有SIDのACEを逆順で削除し、
profileも削除した。事後確認でもworkspaceとlauncherに対象SIDのACEは残らず、profile folder自体も
存在しなかった。

volume rootからfixtureまで固有SIDへ通過権だけを付ける試験も開始したが、最初の`D:\` ACL変更が
管理者権限を必要として拒否されたため、child process起動前に停止した。対象SIDのACEが全対象pathに
存在しないことを確認した。通常利用時にvolume root ACLの変更やUACを要求する方式は、Room開始時の
baselineとして採用しない。

この結果により、Codex App Server process全体をM.I.O.独自AppContainerへ二重収容する経路はV1候補から
外す。Codexについては、App Serverが公開するWindows native sandbox setup、`workspaceWrite`、
`writableRoots`、network policy、approval policyをM.I.O. hostが明示して利用する経路を次の候補とする。
`externalSandbox`やsandbox外で動くprocess APIを代替として使わない。製品採用前に、workspace外と
junction境界を実際のCodex commandで再証明する。

## 2026-08-29 Codex native sandbox follow-up

- Codex CLI: `0.149.0`
- Configured Windows sandbox: `unelevated`
- Explicit thread sandbox: `workspace-write`
- Explicit turn policy: `workspaceWrite`
- Writable roots: selected temporary workspace only
- Network access: disabled
- Workspace read／write smoke: `PASS`
- Nested junction outside write: `FAIL`
- Junction fixture cleanup: `PASS`

App Server自身が生成したJSON Schemaに合わせ、Windows workspace turnへthreadの`workspace-write`と
turnの`workspaceWrite`／`writableRoots`を明示した。通常の一時workspaceではinput read、output write、
内容一致、cleanupがPASSした。

同じcontractでworkspace内junctionからworkspace外のfixtureを指す試験を行うと、外側へのwriteが
成功した。fixtureとescaped fileは試験後に削除した。したがって`unelevated` Windows sandboxは
M.I.O. V1のworkspace境界を満たさず、利用者への注意表示だけで製品fallbackとして有効化しない。
製品のWindows workspace gateは引き続きOFFとし、`elevated` setup完了後に同じ2本のlive試験を
再実行する。

## 2026-08-29 Codex elevated sandbox result

- Codex CLI: `0.149.0`
- Setup request: `windowsSandbox/setupStart` with mode `elevated`
- Setup completion: `success=true`
- Codex Doctor: backend `elevated`, provisioning `complete`
- Explicit `workspaceWrite` workspace read／write: `PASS`
- Explicit `workspaceWrite` nested junction outside write: `FAIL`
- Beta named permission profile nested junction outside write: denied
- Beta named permission profile nested junction outside read: `FAIL`
- Fixture cleanup: `PASS`

推奨の`elevated` setup自体は正常に完了し、通常の選択workspace内ではread／writeとcleanupが成功した。
しかし、明示的なthread／turn sandbox contractでは既存nested junction経由のworkspace外writeが成功した。
比較のため同じ`elevated` backendでbetaの名前付きpermission profileを使うとwriteは拒否されたが、外側の
marker readは成功した。どちらの経路もread／writeを両方拒否するV1境界を満たさない。

両試験は一意なTEMP fixtureだけを使い、junction、workspace、outside folder、escaped fileを終了時に
削除した。Codex native sandbox setupは端末に残してよいが、M.I.O.のWindows workspace製品gateは
引き続きOFFとする。既存reparse pointの事前scanだけではturn中に作成されるreparse pointを防げないため、
それだけを境界修正として採用しない。次の候補は、canonical pathを毎操作で検査するhost-owned workspace
broker、または同等以上の境界を実証できる別process隔離方式である。
