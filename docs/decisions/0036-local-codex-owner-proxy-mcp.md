# ADR 0036: Local Codex owner-proxy MCP

- Status: Accepted
- Date: 2026-08-14
- Depends on: ADR 0012 (idempotent Room message writes), ADR 0023
  (participant identity), ADR 0027 (durable per-recipient dispatch ledger),
  ADR 0035 (Room-scoped conductor orchestration)

## Context

M.I.O.では、CodexをRoom参加AIまたは指揮者として動かす経路をすでに持つ。
これはM.I.O.がCodex App Serverを起動してAI応答をRoomへ保存する内部経路であり、
Codex Desktopで作業中のエージェントがM.I.O.を操作する経路ではない。

Codex DesktopからComputer UseでOwner UIを一時操作することはできるが、その間は
OwnerとCodexが同じ画面、マウス、キーボードを奪い合う。日常的なRoom操作では、
OwnerがPCを使い続けながらCodexが別経路でRoomを読み書きできる方が望ましい。

CodexはローカルMCP serverへ接続でき、Codex Desktop、CLI、IDE extensionは同じ
MCP設定を共有する。ローカル接続ではSTDIOとStreamable HTTPがサポートされ、
Streamable HTTPではBearer tokenまたはOAuthを利用できる。設定変更後はCodexを
再起動する必要がある。これらの製品境界はOpenAIの公式MCP documentationに基づく。

既存のbrowser bridgeはlocalhost transportと短命tokenの実験経路であり、MCPの
production driverではない。また、Room snapshot JSONを外部processから直接編集すると、
M.I.O. Coreの検証、排他制御、idempotency、dispatch ledgerを迂回してしまう。

ADR 0035は指揮者にOwner権限を与えない。今回検討する外部Codex Desktopからの操作は、
指揮者の権限拡大ではなく、Ownerが明示的に接続する別の「Owner代理操作」境界である。

## Decision

### M.I.O.をlocal-only MCP serverとして公開する

最初のproduct pathは、M.I.O. Desktop processがStreamable HTTP MCP endpointを
`127.0.0.1:38474/mcp` のみに公開する。既存browser bridgeのportやprotocolへMCPを
混在させない。

MCP serverはRoom snapshot JSONを直接読み書きしない。すべてのtoolは、Desktop UIや
既存Tauri commandと同じM.I.O. CoreのRoom service、message validation、保存境界を使う。
M.I.O.が終了している場合、toolはboundedな接続不能errorを返し、別processがRoom保存を
引き継がない。

最初のvertical sliceは次の4 toolだけに限定する。

- `mio_status`: M.I.O.とMCP endpointのversion、readinessを返す。
- `mio_room_list`: Ownerが利用できるRoomをboundedに列挙する。
- `mio_room_read`: 指定Roomのmetadataとmessageをpagingして読む。
- `mio_room_post_as_owner`: 指定RoomへOwner代理messageを一度だけ保存する。

任意shell、filesystem、browser、設定変更、Room削除、参加AI変更、AI dispatch、
conductor開始はこのsliceへ含めない。

### Owner代理であることを隠さない

`mio_room_post_as_owner` のauthor participant IDは既存Ownerとするが、通常のOwner入力と
区別できるimmutableなprovenanceを保存する。UIはmessageへ `via Codex` と表示し、
exportや将来の監査でも由来を失わない。Codexを同名の追加Room参加者として登録せず、
CodexがOwner本人の直接入力を装うことも認めない。

tool inputには呼出側が生成するrequest IDを必須とする。同じrequest IDと同じpayloadの
再試行は既存messageを返し、payloadが異なればconflictとする。message本文、Room ID、
paging、response sizeをboundedにし、未知fieldと不正なparticipant参照を拒否する。

### Localhostでも認証する

loopback bindだけを認証の代わりにしない。Codex clientからの接続には秘密token等の
credentialを必須とし、token、Room本文、秘密情報を通常logへ残さない。Web originからの
無許可呼出しはCORSだけに頼らず拒否する。

最初のdevelopment pathはStreamable HTTPのBearer tokenを使う。M.I.O. serverと
Codex clientは同じ `MIO_MCP_TOKEN` environment variableを読み、Codex設定へtoken本体を
保存しない。credentialの安全な保存、rotation、UIからの設定は後続で固定する。

### Pull型接続と常時待機を分ける

MCP toolはCodex側が呼び出した時だけ動く。この接続だけでは、新着messageを契機に
現在のCodex taskを自動で起床させない。最初のsliceでは手動のread/postだけを提供する。

長時間待機が必要になった場合は、`mio_room_wait` のようなbounded long-poll toolと、
durableなCodex taskまたはmonitorのlifecycleを別ADRで設計する。M.I.O.内部のConductor
modeは引き続き自動AI応答の経路であり、外部Codex MCP接続とは混同しない。

### Computer Useは実機UI確認に残す

Computer Useは廃止しない。表示崩れ、クリック動線、keyboard操作など、実画面でしか
確認できないQAへ限定して利用する。通常のRoom read/postはMCPを優先し、OwnerのPC操作を
妨げない。

## Alternatives considered

### Computer Useだけを使う

新しいbackend境界は不要だが、Ownerと同時にPCを使えず、UI変更にも壊れやすい。
日常操作の主経路にはしない。

### Room snapshot JSONをCodexが直接編集する

実装は短いが、複数writer、schema migration、validation、idempotency、監査、AI dispatchの
不変条件を破るため採用しない。

### 独立STDIO serverがRoom保存を所有する

Codex起動時だけ動かしやすい一方、実行中のM.I.O.とRoom writerおよびlifecycleが分裂する。
最初のsliceでは採用せず、Desktop processが正本を所有するStreamable HTTPを先に検証する。

### 外部Codexを新しいRoom参加者として追加する

内部Codex参加AIと外部Codex operatorの見分けがつきにくくなる。最初はOwner代理messageと
明示的provenanceを使う。将来、外部agent固有の会話上の人格が必要になれば別途検討する。

## Security and product invariants

- endpointはloopbackだけにbindし、LANやInternetへ暗黙に公開しない。
- 全tool呼出しを認証し、認証失敗時にRoomの存在や内容を漏らさない。
- Room accessを毎回検証し、別Roomへの誤投稿を防ぐ。
- writeはrequest IDでidempotentにし、timeout後の再試行で二重投稿しない。
- `via Codex` provenanceを保存後に変更または削除できる通常fieldにしない。
- request、response、message、page、実行時間に上限を持たせる。
- MCP inputとRoom historyをuntrusted dataとして扱い、権限やsystem instructionに昇格させない。
- 内部Codex participant、Conductor mode、dispatch ledgerの既存挙動を変えない。
- token、認証header、Room本文、provider secretを通常logへ出さない。

## Rollout

1. transport-neutralなread-only tool contractとbounded Core呼出しを固定する。
2. MCP library候補、Streamable HTTP lifecycle、認証保存方式を小さなspikeで比較する。
3. `mio_status`、`mio_room_list`、`mio_room_read` のread-only product pathを追加する。
4. provenance schemaとidempotent write contractを先に固定し、
   `mio_room_post_as_owner` と `via Codex` 表示を追加する。
5. Codex Desktopへlocal MCPを登録して再起動し、Ownerが同時にPCを操作できる実機testを行う。
6. 必要性を確認してから、待機tool、monitor、追加mutationを別判断として検討する。

各段階は個別に承認、実装、検証する。このADRの採用だけでは後続段階を一括で開始しない。

## Acceptance criteria for the first product slice

- OwnerがPCを操作している間も、Codex DesktopがComputer UseなしでRoomを一覧・閲覧できる。
- Codex Desktopからの投稿がOwner messageとして一度だけ保存され、UIに `via Codex` が見える。
- 同じrequest IDの再送でmessageが増えず、異なるpayloadはconflictになる。
- tokenなし、不正token、許可されないRoom IDは安全に失敗する。
- Room snapshot JSONを外部processが直接変更しない。
- M.I.O.終了中は短時間で明確な接続不能errorになる。
- 内部Codex参加AI、Direct mode、Conductor modeの既存testが退行しない。

## Remaining open questions

- credentialを安全なOS保存領域へ置き、Codexの環境変数参照へどう安全に渡すか。
- read resultの既定page sizeとbyte budgetをいくつにするか。
- 次のtoolを `mio_room_wait` にするか、別の明示mutationにするか。

## Consequences

- Codex DesktopとOwnerが同じWindows desktopを奪い合わず、M.I.O.で協働できる。
- UI自動操作より安定した、versionedでtest可能な操作境界を持てる。
- Owner代理投稿は便利になる一方、provenance、認証、idempotencyをproduct schemaとして
  維持する責任が増える。
- M.I.O. processの起動が接続条件になり、完全なbackground serviceにはならない。
- MCP接続だけでは常時自動応答にならないため、待機とautomationは後続設計が必要になる。

## Implemented first read-only path

2026-08-14時点で次を実装、検証した。

- transport-neutralな `moe-mcp` crateが3つのread-only tool contractを所有する。
- Desktopは公式Rust SDK `rmcp` 3.1.2とAxumでStreamable HTTP endpointを提供する。
- `MIO_MCP_TOKEN` が未設定または不正な場合、serverは起動しない。
- endpointはloopbackだけにbindし、Host、Origin、Bearer token、64 KiB request上限を検証する。
- unauthenticated requestの401、initialize、session、tools/list、実Room catalog/readを確認した。
- Codex global MCP設定にはURLと `bearer_token_env_var` だけを登録し、token本体を含めない。
- `mio_room_post_as_owner`、`via Codex` provenance、secure credential UI、待機toolは未実装である。

2026-08-17の実Codex Desktop follow-upでは、再起動後のtask inventoryに
`mio_status`、`mio_room_list`、`mio_room_read` が公開され、3 toolをCodex自身から
呼び出した。statusはready、Room catalogは4件、bounded Room readは1 message pageと
6 participantsを返した。診断出力にはtoken、Room ID、Room名、message本文を含めていない。

## Implemented Owner-proxy write path

2026-08-17時点で、最初のwrite product pathを次の境界で実装した。

- `RoomMessage`へoptionalな `codexOwnerProxy` provenanceを追加した。既存snapshotはfieldなしで
  読み込め、通常messageではfield自体を保存しない。
- `mio_room_post_as_owner` は `requestId`、`roomId`、`recipientIds`、`body` だけを受け取り、
  author、timestamp、message ID namespace、provenanceをM.I.O.側で固定する。
- 同じrequest IDと同じpayloadは既存messageを返し、本文、宛先、Room、author、artifact、
  provenanceのいずれかが異なる場合はconflictとする。
- tool annotationはread-only false、destructive false、idempotent true、open-world falseとする。
  Codex側はMCP serverの `default_tools_approval_mode = "writes"` を使い、read toolを妨げず
  write toolだけOwner承認を要求する。
- 新規保存後は本文をevent payloadへ載せずRoom IDだけをDesktop UIへ通知し、UIが既存の
  bounded Room readを再利用して対象Roomを再読込する。duplicate時も通知を再送し、最初の
  通知をUIが受信できなかった場合に安全に回復する。
- UIは保存済みprovenanceだけを根拠に `via Codex` badgeを表示する。author名、message ID、
  表示名から推測しない。
- このwrite pathはmessage保存だけを行い、AI dispatch、Direct mode、Conductor modeを
  自動開始しない。

## References

- OpenAI, [Model Context Protocol](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
