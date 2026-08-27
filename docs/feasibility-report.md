# M.O.E. Feasibility Report

Status: Windows personal V1 foundation complete; provider and deployment expansion remains

## Latest separate Claude connection status verification

- Date: 2026-08-12
- ADR: `docs/decisions/0018-separate-claude-connection-status.md`
- Rust status surface: Codex launcher availability、Claude Code CLI detection、Claude Web Remote MCP setup requirement、unsupported providerを別々のbounded stateとして返す。CLI検出だけでClaudeを接続済みにしない
- UI: product releaseでClaude Web `Web接続待ち`、Codex `利用可能`、Gemini `未接続`、追加menuでClaude Code `CLIあり・未接続`を確認
- Recipient guidance: Claude Webを選ぶと `未接続AI宛はRoomへ保存のみ · 現在は返信がありません` へ変化。message送信と永続Room membership変更は未実施
- Codex isolation: 新しいCodex turnは、他AIへの到達・可視性・返信・再送能力を断言しない。既存Room履歴は変更しない
- Automated verification: TypeScript typecheck/build PASS、Rust workspace 101 PASS・2 ignored、Clippy `-D warnings`、format、diff check PASS
- Remaining live boundary: Claude Code structured-stream responseとClaude Web Remote MCP / Relay reply-writeは別々の製品adapterが必要。最終認証済み応答試験は所有者のClaude再契約後

## Latest Room backup and recovery verification

- Date: 2026-08-12
- ADR: `docs/decisions/0017-room-backup-and-recovery.md`
- Persistence: 壊れたprimaryを `.corrupt-*` へ退避し、正常な直前backupからprimaryを再確立。壊れたbackupしかない場合は安全に起動拒否
- Tauri surface: `desktop_room_backup` と `desktop_room_restore_latest_backup`。filesystem pathはRust ownership、WebViewへはfile名とRoom数だけを返す
- Bundled catalog: 10参加者、M.O.E.開発室、回答くらべ部屋、MCP実験室。全Roomを同じread / write / dispatch / persistence経路へ統合
- Existing file upgrade: version 1 single-room snapshotへ不足しているbundled participant / Roomだけを追加し、既存ID・messageを上書きしない
- Failure path: export / restoreも同じtransaction mutex、snapshot validation、temp + sync + rename、失敗時memory rollbackを使用
- Automated verification: TypeScript production build PASS、Rust workspace 98 PASS・2 ignored（外部Codex live smokeとWindows credential integration）
- Tauri UI: 隔離backup directoryへ3室・10参加者のversion 1 JSONを生成し、file名付き成功表示と `最新を復元…` → `本当に復元` を確認。実画面の最終復元は未実行
- Recovery tests: primary破損退避、直前backup復旧、最新M.O.E. backupだけの選択、restore後process reload、不正file名拒否を確認
- Cleanup: debug app / Cargo watcher / Viteだけを終了。既存release PID 14448は継続
- V2 candidates: Claude / Gemini正式adapter、公開Relay、署名済みWindows配布。Room並べ替え、participant tombstone、任意import / retention、複数process writer、永続dispatch ledger、incremental databaseは必要時の拡張

この文書には推測ではなく、実際のCLI、API、Webサービス、MCP接続で観測した結果を記録します。

## Environment

- Windows: Windows 11 (x86_64)
- Node: `v24.14.1`
- npm: `11.11.0`
- Rust: `rustc 1.96.1`, `cargo 1.96.1`
- Codex Desktop: `26.803.10989.0`
- Codex standalone CLI: `codex-cli 0.145.0` (`@openai/codex@0.145.0`)
- Claude Code: `2.1.220` (Windows native install)
- Other relevant clients: Not checked in this tranche

## Connection probes

| Probe | Status | Evidence | Notes |
|---|---|---|---|
| Codex App Server | PASS | Handshake, restart/resume, localImage, interrupt recovery, approval gate, and seeded-fixture repair PASS; see `../spikes/codex-app-server/evidence/` | Test 0-A criteria satisfied on `codex-cli 0.145.0` |
| Claude Code structured stream | PARTIAL | CLI discovery, auth status, `system/init`, Fable model, session ID, and JSONL event framing observed; `spikes/claude-code-stream/evidence/handshake-latest.json` | Inference blocked by Anthropic `oauth_org_not_allowed` / HTTP 403; user choice required |
| Remote MCP roundtrip | PARTIAL | Local protocol、temporary Claude Web、local/public Room、local Relay→Desktop統合PASS; `../spikes/remote-mcp/evidence/` | Web入口・Room読取・ローカル全経路は実証。正式認証、公開Relay、Artifactは未実施 |
| Relay roundtrip | PARTIAL | Local Desktop outbound link、MCP integration、pairing、Rust product HTTPS、Room read、UI write後read、correlation、disconnect/reconnect PASS; `../spikes/relay-roundtrip/evidence/` と本書のproduction HTTPS統合節 | localhost TLS限定。公開Relay、Relay経由write、Artifactは未実施。Codex AI dispatchはDesktop内の別PASS |
| Windows credential storage | PASS | 製品候補crate経由でwrite/read/update/delete、別process復元、Rust pairing response直接保存、production HTTPS taskの2世代認証、metadata-only Tauri command PASS; `../spikes/windows-credential-store/evidence/windows-credential-manager-latest.json` / `../spikes/relay-roundtrip/evidence/rust-product-pairing-latest.json` / `../spikes/relay-roundtrip/evidence/rust-product-connection-latest.json` | rotation UI、非Windows backendは未実施 |
| Product Rust Relay client boundary | PASS | pairing、credential直接load、borrowed transport、lifecycle metadata、二重接続拒否、managed drop、production HTTPS Desktop task、Room read routerを固定 | 公開Relay、Room write、接続操作UIは未実施 |
| Desktop Room read UI hydration | PASS | bounded Tauri command、TypeScript response検査、browser preview fallback、実Tauri表示を確認 | 全Room catalog / readへ拡張済み |
| Desktop Room message write | PASS | idempotent Core store、bounded Tauri command、成功後UI、再読込、Relay readを確認 | 全Roomへ拡張済み。Codex dispatch / persistenceは別PASS |
| Codex Room AI dispatch | PASS | text-only permission profile、process内dispatch ledger、実App Server、Room reply、実Tauri表示・再読込を確認 | Codexのみ。永続ledger、継続session、Claude Web / Geminiは未実施 |
| Desktop Room persistence | PASS | versioned JSON、temp / backup切替、validation、rollback、破損退避、自動復旧、明示backup / restore、実process再起動後のUI復元を確認 | 単一Desktop writer。migration、任意import / retention、複数process writerは未実施 |
| Image artifact transfer | PARTIAL | Codex App Server localImage input PASS; `spikes/codex-app-server/evidence/image-latest.json` | Provider-neutral Artifact output/relay transfer remains |
| Long task / timeout | PARTIAL | `turn/interrupt` and immediate recovery PASS; `spikes/codex-app-server/evidence/interrupt-latest.json` | Provider timeout behavior remains |
| Generic MCP interoperability | NOT STARTED | | |
| Provider-neutral Adapter contract | NOT STARTED | | |

Status values: `PASS`, `PARTIAL`, `FAIL`, `NOT STARTED`.

## Exit Gate repetitions

| Run | Required fault | Status | Evidence |
|---|---|---|---|
| 1 | Cold start | NOT STARTED | |
| 2 | Relay disconnect / reconnect | PASS | `spikes/relay-roundtrip/evidence/relay-roundtrip-latest.json` |
| 3 | Adapter process restart / resume | PASS | `spikes/codex-app-server/evidence/resume-latest.json` |

## Evidence index

- Commands and versions: `node --version`, `npm --version`, `rustc --version`, `cargo --version`, `codex --version`, `codex app-server --help`, `claude --version`, `claude --help`, `claude auth status`
- Generated schemas: `spikes/codex-app-server/generated/0.145.0/` (local, ignored); tracked manifest: `spikes/codex-app-server/schema-manifest.md`
- Sanitized event logs: `spikes/codex-app-server/evidence/handshake-latest.json`, `spikes/codex-app-server/evidence/resume-latest.json`, `spikes/codex-app-server/evidence/image-latest.json`, `spikes/codex-app-server/evidence/interrupt-latest.json`, `spikes/codex-app-server/evidence/approval-latest.json`, `spikes/codex-app-server/evidence/seeded-fixture-latest.json`, `spikes/claude-code-stream/evidence/handshake-latest.json`, `spikes/remote-mcp/evidence/local-roundtrip-latest.json`, `spikes/remote-mcp/evidence/public-claude-web-latest.json`, `spikes/remote-mcp/evidence/room-read-local-latest.json`, `spikes/remote-mcp/evidence/room-read-claude-web-latest.json`, `spikes/remote-mcp/evidence/relay-desktop-integration-latest.json`, `spikes/relay-roundtrip/evidence/relay-roundtrip-latest.json`, `spikes/relay-roundtrip/evidence/device-pairing-latest.json`, `spikes/relay-roundtrip/evidence/rust-product-pairing-latest.json`, `spikes/relay-roundtrip/evidence/rust-product-connection-latest.json`, `spikes/windows-credential-store/evidence/windows-credential-manager-latest.json`
- Diff artifacts: `sourceDiff` in `spikes/codex-app-server/evidence/seeded-fixture-latest.json`
- Test results: seeded fixture initial 2 PASS / 1 FAIL, final 3 PASS / 0 FAIL in `spikes/codex-app-server/evidence/seeded-fixture-latest.json`
- Screenshots and documents:
- Manual actions:

## Required architecture changes

- App Serverの初期Transportは公式仕様どおり `stdio` / JSONLとする。WebSocketは初期Coreの前提にしない。
- Codex Desktopの内蔵実行ファイルと独立CLIを同一視しない。2026-08-11のCodex Desktop内ターミナルでは、PATH先頭のアプリ内蔵版が通常プロセスから `Access is denied` となった一方、独立 `@openai/codex@0.145.0` は実行できた。
- CLIの場所は将来Adapter設定で上書き可能にし、検出失敗を認証失敗として扱わない。
- thread復旧は保存済みthread IDを第一候補にする。`thread/list` は狭いcwd/source filterだけに依存せず、全source kindを明示したページングとID照合をfallbackにする。
- 2026-08-11の最初の絞り込み付き `thread/list` では対象が返らず、App Server stderrにstate DBとrollout pathのread-repair警告が出た。同じthreadは全source・ページング一覧、`thread/read`、`thread/resume` では正常に復旧できた。
- file-change承認UIは `threadId`、`turnId`、`itemId` へ紐づけ、既定を拒否にする。回答後は `serverRequest/resolved` と同じitemの終端statusを確認するまで、UI上で適用済みと表示しない。
- `item/fileChange/requestApproval` 自体には変更pathが含まれないため、先行する `item/started` の同じ `itemId` からpathを取得して許可rootと照合する。対応itemやpathを特定できない承認要求は拒否する。
- Provider固有eventはAdapter内で順序を保持し、Core側では少なくとも `job.started`、`approval.requested`、`approval.resolved`、`job.completed` へ正規化できる。ただし今回の形式はPoC証拠であり、製品contractの決定ではない。
- Claude Code launcherはPATHだけに依存せず、Windows native installerの標準位置とユーザー指定pathを扱う。CLI未検出と認証拒否を別statusにする。
- Claude Codeの `--bare` はOAuth/keychainを読まないため、subscription login検証には使わない。製品Adapterではcustomization隔離とcredential sourceを別設定として扱う。
- Claude Code AdapterはClaude.ai OAuth credentialを抽出・再利用しない。第三者製品向けの公式guidanceに従い、production相当の接続は明示的なAPI keyまたは対応cloud providerを基本候補とし、credential sourceと課金境界をUIへ表示する。

## Manual actions

Record logins, connector registration, permission approval, browser interaction, and any other step that cannot be automated safely.

- 2026-08-11: 認証操作なし。既存の独立Codex CLIを使った環境確認とschema生成のみ実施。
- 2026-08-11: 既存ログインでbasic handshakeを実行。追加のログイン、承認、ブラウザ操作なし。
- 2026-08-11: 別App Server processからlist/read/resumeを実行。追加のログイン、承認、ブラウザ操作なし。
- 2026-08-11: localImageでPNG fixtureを送信。追加のログイン、承認、ブラウザ操作なし。
- 2026-08-11: 長いturnをinterruptし、同じthreadで回復turnを実行。追加のログイン、承認、ブラウザ操作なし。
- 2026-08-11: 専用fixtureでfile-change承認を自動試験。probeクライアントが `decline` / `accept` を返し、手動承認ダイアログや追加ログインはなし。
- 2026-08-11: seeded fixtureをCodexへ渡して実修正。対象ソース1本のfile-changeだけprobeが許可し、手動承認ダイアログや追加ログインはなし。
- 2026-08-11: Claude Code preflightを既存ログインで実行。ログイン操作、permission approval、課金はなし。programmatic access有効化または別credential選択が手動対応として残る。
- 2026-08-12: Claude Web無料プラン画面に「カスタムコネクタを追加」とRemote MCP URL入力欄があることを読み取り確認。登録、送信、設定変更はなし。
- 2026-08-12: Remote MCPをlocalhostだけで自動検証。公開URL、外部通信、認証登録、Claude Web tool呼び出しはなし。
- 2026-08-12: プロジェクト所有者の承認後、秘密pathとHost allowlistを付けた読み取り専用MCPをTryCloudflareで一時公開。`M.O.E. Local Test`をClaude Webへ登録し、`ping_moe`を「一度だけ許可」で1回実行した。成功後にTunnelとserverを停止し、port閉鎖と秘密URL入り一時ファイル削除を確認した。「常に許可」は選択していない。
- 2026-08-12: プロジェクト所有者が外部送信を明示承認した開発室デモsnapshotだけを新しい短時間Tunnelで公開。Claude Webの既存テストコネクタを一時URLへ差し替え、`moe_read_room`を「一度だけ許可」で1回実行した。`claude-web-room-probe` / `CLAUDE_WEB_ROOM_RUNTIME_OK`を取得後、Tunnel、server、port、一時snapshot、秘密URL入りsessionを削除した。「常に許可」は選択していない。
- 2026-08-12: Desktop outbound Relay spikeをlocalhostだけで自動検証。外部通信、公開URL、ログイン、手動承認、永続credentialはなし。probe専用tokenは実行中のmemoryだけで使用した。
- 2026-08-12: 公式MCPクライアントからRemote MCP、local Relay、Desktop役のRoom sourceを経由する全経路をlocalhostだけで自動検証。外部通信、公開URL、ログイン、手動承認、production credentialはなし。
- 2026-08-12: local Relayのdevice pairing contractを自動検証。pairing codeとdevice credentialはprobe内だけで生成・交換し、生値を証跡へ保存していない。外部通信、ログイン、手動承認、OS credential vaultへの書き込みはなし。
- 2026-08-12: プロジェクト所有者の進行指示後、一意な `M.O.E./probe/...` targetだけをWindows Credential Managerへ一時作成。別process読取、上書き、削除を確認し、終了時に同じtargetが存在しないことを再確認した。既存credential、ブラウザ、Provider loginには触れていない。
- 2026-08-12: プロジェクト所有者の進行指示後、Node localhost Relayが発行した短時間codeをRust pairing transportへprivate environment variableで渡し、製品Relay managerから一意な `M.O.E./relay-device/v1/rustpair-...` targetへdevice credentialを直接保存した。存在確認後にmanagerが削除し、finally相当の別cleanup processでもnot foundを確認した。公開network、WebView、既存credential、ブラウザ、Provider loginには触れていない。
- 2026-08-12: 続く進行指示後、一意な `M.O.E./relay-device/v1/rustconnect-...` targetへpairing credentialを保存し、製品Relay managerがloadした借用secretからRust localhost transportのAuthorizationへ直接書いた。Node Relayが`paired-device`としてhelloを承認し、Room marker往復、切断、manager削除、削除後の接続拒否、別cleanup processのnot foundがPASS。公開network、WebView、既存credential、ブラウザ、Provider loginには触れていない。

## Phase 0-A progress

Status: PASS

### Basic stdio handshake — PASS

- Date: 2026-08-11
- Client: dependency-free Node.js probe
- Transport: `stdio` / JSONL
- CLI: `codex-cli 0.145.0`
- Sequence: `initialize` → `initialized` → `thread/start` → `turn/start` → streamed notifications → `turn/completed`
- Turn status: `completed`
- Expected marker: `MOE_APP_SERVER_OK`
- Observed final text: `MOE_APP_SERVER_OK`
- Agent message delta notifications: 5
- Elapsed: 12,919 ms
- Server-initiated requests: none
- Authentication: existing login; no manual action required

これは接続口の基本往復が実在する証拠であり、単独ではTest 0-A全体の合格ではない。localImage、interrupt、approval deny/approve、共通fixtureの実修正は後続probeでPASSした。

### Process restart / thread resume — PASS

- Date: 2026-08-11
- Source: persistent thread created by the basic handshake probe
- New App Server process: yes
- `thread/list`: target found with all-source pagination
- `thread/read`: original thread ID and `MOE_APP_SERVER_OK` history restored
- `thread/resume`: returned the same thread ID
- Continued turn status: `completed`
- Continued turn final text: `MOE_RESUME_OK`
- Elapsed: 4,960 ms
- Server-initiated requests: none
- Authentication: existing login; no manual action required

初回はcwdとsourceを絞った `thread/list` が対象を返さず、state DB read-repair警告が観測された。全source kindを明示してページングした再試験は成功した。復旧の正本は保存済みthread IDとし、一覧は索引修復やmetadata差分を考慮した探索手段として扱う。

### localImage content recognition — PASS

- Date: 2026-08-11
- Input type: `localImage`
- Image: deterministic 1200×800 PNG fixture
- Image detail: `original`
- Image SHA-256: `A9EF12D16F023CC10D01A6846739F650145B031C91283AABBA561256176F2BE9`
- Prompt disclosure: hidden code was not included in the text prompt
- Expected image-only code: `NEKOMIMI-42`
- Observed final text: `NEKOMIMI-42`
- Exact match: yes
- Turn status: `completed`
- Elapsed: 5,167 ms
- Thread: ephemeral / read-only
- Server-initiated requests: none
- Authentication: existing login; no manual action required

固定回答やファイル名からの推測ではなく、画像下部にだけ描画した固有コードと最終回答が完全一致した。これによりCodex App Serverの `localImage` が実際の画像内容へ利用されたと判定する。

### turn/interrupt and recovery — PASS

- Date: 2026-08-11
- Interrupted turn: long 10,000-item response request
- Interrupt timing: immediately after `turn/started`
- Interrupted turn final status: `interrupted`
- Interrupt request to completion: 5 ms
- Recovery: a new turn on the same thread
- Recovery turn final status: `completed`
- Expected recovery text: `MOE_AFTER_INTERRUPT_OK`
- Observed recovery text: `MOE_AFTER_INTERRUPT_OK`
- Total elapsed: 5,981 ms
- Thread: ephemeral / read-only
- Server-initiated requests: none
- Authentication: existing login; no manual action required

`turn/interrupt` 後に終了状態を明確に観測でき、同じthreadで新しいturnを安全に開始できた。Phase 0-Aのinterrupt条件は満たした。provider timeoutとUI上のキャンセル表示は後続試験で確認する。

### File-change approval deny / accept — PASS

- Date: 2026-08-11
- CLI: `codex-cli 0.145.0`
- Thread: ephemeral / read-only sandbox
- Approval policy: `on-request`
- Reviewer: `user`（probeクライアントが応答）
- Deny: `item/fileChange/requestApproval`へ `decline`、終端status `declined`
- Deny target: SHA-256が試験前後で一致、内容変更なし
- Accept: `item/fileChange/requestApproval`へ `accept`、終端status `completed`
- Accept target: `BEFORE_APPROVAL` から `APPROVED_CHANGE` へ変更
- Sentinel: deny / acceptの両方でSHA-256一致
- Scope: runtimeのファイル一覧は両方とも `sentinel.txt` と `target.txt` のみ
- Command approval requests: deny / acceptとも0件
- Event order: `item/started` → approval request → `serverRequest/resolved` → `item/completed` を両方で確認
- Authentication: existing login; no manual action required

拒否時は変更ゼロ、許可時だけ指定ファイルが変わり、対象外sentinelは不変だった。承認結果は回答送信だけで確定扱いにせず、`serverRequest/resolved` とfileChangeの終端statusまで追跡できることも確認した。Phase 0-Aのfile-change approval条件は満たした。

### Seeded fixture investigation and repair — PASS

- Date: 2026-08-11
- CLI: `codex-cli 0.145.0`
- Fixture: `spikes/fixtures/seeded-bug-app/`
- Initial test: 2 PASS / 1 FAIL
- Observed bug: 未選択の古いregistry行が、同じadapter IDの選択済み行を重複排除で隠す
- Root cause returned by Codex: 選択済みfilterより先にadapter IDの重複排除を行っていた
- Allowed change: `runtime/src/delivery-plan.mjs` のみ
- Actual changed paths: `src/delivery-plan.mjs` のみ
- Final test: 3 PASS / 0 FAIL
- Protected files: package.json、sentinel、SPEC、test、UI SVGのSHA-256が前後一致
- File approval: 1件、許可pathと一致、終端status `completed`
- Command approval requests: 0件
- Event order: `item/started` → approval request → `serverRequest/resolved` → `item/completed`
- Result explanation: root cause、変更path、test command、sentinel不変をCodexが返却
- Authentication: existing login; no manual action required

固定応答ではなく、Codexが実際の失敗testと仕様を読み、原因に沿ってソースを修正し、同じtestを成功させた。許可外ファイルは内容だけでなくSHA-256でも不変であり、変更diffと説明も取得できた。これによりTest 0-Aの全合格基準を満たした。

## Phase 0-B progress

Status: PARTIAL — external access blocked

### Claude Code structured stream preflight — BLOCKED

- Date: 2026-08-11
- CLI: `Claude Code 2.1.220`
- Launcher: Windows native default path（PATH未登録）
- Authentication status: logged in via Claude.ai / first-party / Max
- Requested model alias: `fable`
- Observed model: `claude-fable-5`
- Transport: subprocess / newline-delimited `stream-json`
- Tool surface: empty
- Permission mode: `dontAsk`
- Observed before inference: `system/init`、`system/status`、session ID、model、tool list
- Inference result: HTTP 403 / `oauth_org_not_allowed`
- Marker response: not reached
- File changes: none
- Cost: USD 0
- Evidence redaction: email、organization name/ID、credential、raw tokenは未保存

CLI discovery、既存login、Fable alias解決、stream-json framing、session ID取得までは成立した。推論はAnthropic側のprogrammatic subscription access制限で拒否されたため、structured response、session継続、fixture修正、permission、cancel、image、resumeは未試験である。

続行には、プロジェクト所有者がClaude側でprogrammatic accessを有効化するか、M.O.E.用に明示したAnthropic API key / 対応cloud providerを選択する必要がある。既存OAuth credentialの抽出や、未承認のAPI課金への自動切替は行わない。

## Phase 0-C progress

Status: PARTIAL — Claude Web minimal roundtrip PASS

### Claude Web Remote MCP local preflight — PASS

- Date: 2026-08-12
- SDK: `@modelcontextprotocol/sdk 1.30.0`
- Transport: Streamable HTTP、stateless、JSON response
- Bind: `127.0.0.1`、ephemeral port
- Sequence: initialize → tools/list → `ping_moe` → `moe_status`
- Tool list: `ping_moe`、`moe_status`
- Tool annotations: read-only、idempotent
- File / shell / write capability: none
- Public HTTPS: not exposed
- Authentication: not implemented
- Claude Web registration and call: not performed

公式SDKクライアントとのローカル往復は成立した。これはRemote MCP transportの最小成立を示すが、Test 0-C全体の合格ではない。

### Claude Web temporary public roundtrip — PASS

- Date: 2026-08-12
- Claude plan observed in UI: Free
- Connector: `M.O.E. Local Test`
- Public transport: TryCloudflare Quick Tunnel、HTTPS、HTTP/2
- Public SDK preflight: initialize → tools/list → `ping_moe` → `moe_status` PASS
- Claude Web: Connector registration PASS、`Ping M.O.E.` tool recognition PASS
- Approval: 「一度だけ許可」; 「常に許可」は未選択
- Claude Web result: `{"ok":true,"service":"moe-remote-mcp-spike"}`
- Security: 64-hex secret path、exact Host allowlist、read-only tools only
- Observed guard: allowlist追加前のpublic Hostを公式SDK middlewareが403 `Invalid Host`で拒否
- Cleanup: Tunnel停止、server停止、port 3108閉鎖、秘密URL入り一時ファイル削除を確認
- Persisted evidence: public hostname、秘密path、完全URLは保存しない

これにより、Claude Codeを介さず、通常のClaude Web無料プランからRemote MCPを通じてM.O.E. toolを呼べることが実証された。次は本番相当の認証またはdevice pairing、RelayとDesktopのoutbound link、`moe_read_room`、`moe_get_artifact`、切断復旧、idempotencyを段階的に追加する。

### `moe_read_room` runtime snapshot preflight — PASS

- Date: 2026-08-12
- Source: host-selected `spikes/remote-mcp/runtime/room-snapshot.json`
- Input surface: `roomId`、`afterMessageId`、`limit`（1〜30）のみ
- Runtime mutation: probe起動時に `probe-runtime-message` / `REMOTE_ROOM_RUNTIME_OK` を追加
- Read: `afterMessageId=welcome-3`、`limit=1`でruntime追加メッセージを取得
- Unknown room: `room_not_found`
- Unknown cursor: `cursor_not_found`
- Missing / invalid snapshot: `room_snapshot_unavailable`
- Raw path input: tool surfaceになし
- Snapshot path boundary: `spikes/remote-mcp/runtime/`外をhost設定でも拒否
- Cleanup: probe終了時にruntime snapshotを削除

tool callbackの固定回答ではなく、起動時に追加したruntimeメッセージをcursor指定で取得できた。Room、Message、ParticipantはProvider固有形式やUIの`targetIds` / `sentAt`ではなく、中立の`recipients` / `createdAt` / `artifactIds`形式を使う。このローカル段階はDesktop接続ではなくhost-selected snapshotであり、Claude Webからの実呼び出し結果は次節に記録する。

### Claude Web `moe_read_room` temporary public roundtrip — PASS

- Date: 2026-08-12
- Claude plan observed in UI: Free
- Connector: `M.O.E. Local Test`
- Tool recognition: `Read M.O.E. Room`を含む読み取り専用tool 3件
- Input: `roomId=moe-dev-room`、`afterMessageId=welcome-3`、`limit=1`
- Approval: 「一度だけ許可」; 「常に許可」は未選択
- Result: `id: claude-web-room-probe`、`body: CLAUDE_WEB_ROOM_RUNTIME_OK`
- Other tool calls: none
- Approved data: 開発室のデモsnapshotのみ。実会話、任意ファイル、shell内容は送信していない
- Cleanup: Tunnel停止、server停止、port 3108閉鎖、一時snapshotと秘密session削除を確認
- Persisted evidence: public hostname、秘密path、完全URLは保存しない

これにより、通常のClaude WebがM.O.E.のRoom cursorを指定して、host-selected snapshotから最新メッセージを読み取れることまで実証した。まだDesktop本体からRelayへのoutbound linkではなく短時間fixture経路なので、Test 0-C全体はPARTIALのままとする。

### Desktop outbound Relay local roundtrip — PASS

- Date: 2026-08-12
- Transport: localhost上の永続・双方向HTTP NDJSON stream（production WSSではない）
- Direction: Desktop役からRelay役への外向き接続
- Authentication preflight: 実行中だけのprobe token。誤tokenは `device_unauthorized`
- Request correlation: 同時Room要求2件を別request IDで正しいresponseへ対応付け
- Runtime marker: `relay-runtime-message` / `DESKTOP_RELAY_ROOM_OK`
- Input boundary: `roomId`、`afterMessageId`、`limit`以外を拒否。raw filesystem pathは `invalid_request`
- Relay retention: Room snapshotと本文を保持せず、観測値 `retainedRoomCount=0`
- Disconnect: Desktop切断中のreadは即座に `desktop_offline`
- Reconnect: 新しい外向きlink確立後の同じRoom readがPASS
- Cleanup: Relay停止、Desktop link終了、runtime snapshot削除
- Public network / login / manual approval: none

これでRelayがRoomデータを抱えず、接続状態と処理中requestの相関だけを持つ境界がローカルで成立した。transportは捨てられるPoCであり、製品化前にTLS/WSSまたは同等の常設transport、device pairing、credential lifecycle、公開Relay、MCP endpointとの結合を別途検証する。

### Remote MCP → Relay → Desktop local integration — PASS

- Date: 2026-08-12
- Client: 公式 `@modelcontextprotocol/sdk` Streamable HTTP client
- Full path: MCP client → Remote MCP → local Relay → Desktop Room source → Relay → MCP result
- `moe_status`: `relay=local-outbound-link`、`desktop=connected`
- `moe_read_room`: `afterMessageId=welcome-3`、`limit=1`
- Result: `mcp-relay-desktop-message` / `REMOTE_MCP_RELAY_DESKTOP_OK`
- Desktop disconnect: MCP tool resultは `desktop_offline`
- Desktop reconnect: statusが `connected`へ戻り、同じRoom readがPASS
- Relay retention after completion: Room 0件、pending request 0件
- Relay URL boundary: credentialなしのloopback HTTP originだけを許可
- Public network / login / manual approval / production credential: none
- Cleanup: Remote MCP、Relay、Desktop link停止、runtime snapshot削除

これにより、これまで別々に検証していたWeb向けMCP surfaceとDesktop outbound linkが、ローカルでは一本のRoom読取経路として成立した。次の外部境界は、このRelay pathを短時間HTTPS入口へ接続してClaude Webから1回読む試験、または先にdevice pairing契約を定義することである。公開試験はデータと送信先を再提示し、改めて明示承認を得てから行う。

### Local device pairing contract — PASS

- Date: 2026-08-12
- Pairing code: 判別しやすい英大文字・数字8文字、表示は `XXXX-XXXX`
- Lifetime: 既定60秒、最大10分
- Exchange: codeを一度だけ32-byte device credentialへ交換
- Wrong attempt: 残回数を減らし、5回で `pairing_code_locked`
- Expiry: `pairing_code_expired`
- Reuse: `pairing_code_used`
- Credential connection: paired DesktopからRoom marker `DEVICE_PAIRING_ROOM_OK` を取得
- Device binding: credentialと異なるdevice IDのhelloを拒否
- Revocation: 接続中deviceを切断し、旧credentialは `device_unauthorized`
- Re-pairing: 新codeと新credentialで接続・Room read復旧
- Secret storage: 生code・生credentialを保持せず、process-memory keyによるHMAC-SHA-256のみ
- Persistence: none。Relay停止でpairing/credential stateは消える
- Public network / production credential / login / manual approval: none

これでpairingの状態遷移と拒否条件はローカルで成立した。製品化にはDesktop側のOS credential vault、Relay側credential recordの安全な永続化、rotation、複数device管理、user-presenceを伴う承認UI、秘密を含めないaudit logが残る。短いpairing code自体を長期credentialとして使用してはならない。

### Windows Credential Manager lifecycle — PASS

- Date: 2026-08-12
- Backend: Windows Credential Manager / Generic Credential
- API: `CredWriteW`、`CredReadW`、`CredDeleteW`、`CredFree`
- Persist: `CRED_PERSIST_LOCAL_MACHINE`、現在のWindows user用
- New write: PASS
- Separate-process exact read: PASS
- Update / old-value mismatch: PASS
- Delete / not found after delete: PASS
- Secret transport to child: command lineではなくprivate environment variable
- Child behavior: environmentから読んだ直後に変数を除去し、stdout / stderrへ値を出さない
- Evidence / repository: secret、fingerprint、完全target名を保存しない
- Scope: 実行ごとに一意な `M.O.E./probe/...` targetだけ
- Cleanup: 正常・異常時のguardと最終not found確認
- Existing Windows credentials / browser login / Provider login: untouched

Windows Desktopでdevice credentialを平文JSONではなくOS credential vaultへ置けることは実証できた。次はprobeコードをそのままUIへ露出せず、Rust backendの小さな抽象へ昇格し、stable target naming、metadata version、rotation rollback、複数Relay accountを定義してからTauri commandへ接続する。

### Product Rust credential boundary — PASS

- Date: 2026-08-12
- Crate: `crates/moe-credential-store/`
- Stable target schema: `M.O.E./relay-device/v1/<account-id>`
- Account ID boundary: 小文字英数字で開始、`.` / `_` / `-`のみ追加可、最大64 bytes
- Path / target injection: `../other-target`、slash、uppercase、空文字を拒否
- Secret type: `SecretBytes`。Serialize / Cloneなし、Debugは `[REDACTED]`
- Memory cleanup: drop時にvolatile writeとcompiler fence
- Unsafe boundary: Win32 Credential API呼び出しをcredential crateだけへ隔離
- Rust backend internal API: store / load / delete / contains
- Tauri command: credential境界は`relay_credential_status(accountId)`のみ。後続Relay serviceは別のmetadata-only connection statusを追加
- WebView response: account ID、stored、backendだけ。secret読取・保存commandはなし
- OS error response: WebViewへ数値やtargetを出さず、固定code / messageへ変換
- Actual Windows roundtrip: 製品候補crate経由で別process read、update、delete PASS
- Tests: credential crate 3 PASS、Tauri backend 1 PASS

React/WebViewはcredentialの存在を表示できるが、値を受け取れない。次の結合ではRust内部のRelay connection managerだけが `SecretBytes` をloadし、そのままoutbound transportへ渡す。UI入力からsecretを保存するcommandは作らず、pairing responseをRust backendが直接保存する。

### Product Rust Relay client boundary — PASS

- Date: 2026-08-12
- Crate: `crates/moe-relay-client/`
- Identity: account IDは`RelayCredentialId`へ検証とtarget生成を委譲し、device IDは小文字英数字開始・`.` / `_` / `-`・最大128 bytesに限定
- Pairing response: device credentialを`SecretBytes`に閉じ、Serialize / Cloneを持たせずDebugをredact
- Pairing code: Node PoCと同じ判別しやすい`XXXX-XXXX`形式だけを受け、Serialize / Cloneを持たない短時間secretとしてDebugをredactし、交換後にbufferを消去
- Pairing transport: codeとdevice IDを寿命付きの借用値としてRust transport traitへ一度だけ渡し、responseをmanagerへ直接返す
- Device binding: 期待したdevice IDとresponseのdevice IDが一致しない場合は保存前に拒否
- Pairing storage: Rust managerがcredential storeへ直接保存し、呼出元へ返すのはaccount / device metadataだけ
- Connection: Rust managerがcredential storeから直接loadし、寿命付きの借用値としてtransport traitへ渡す
- Failure boundary: 未保存、保管backend失敗、device不一致、transport拒否・停止・protocol・cancelをsecretなしの型で区別
- Re-pairing: 保存成功時はcredentialを置換し、保存失敗時は既存credentialを保持
- Deletion: 明示削除はidempotentで、削除後の接続はtransportを呼ぶ前に拒否
- Desktop integration: `RelayClientService<PlatformCredentialStore>`をTauri managed stateとして保持
- Tauri surface: `bootstrap_status`、metadata-only `relay_credential_status`、metadata-only `relay_connection_status`。secret保存・読取・接続開始commandの追加なし
- Tests: Relay client 18 PASS、Desktop backend 3 PASS、Rust workspace全体PASS、Desktop frontend typecheck / production build PASS
- External effects: fake store / fake transportだけを使用し、実Windows credential、公開network、Provider loginには触れていない

このPASSは製品用の安全な接続境界と、test transportによるRust内部のpairing code交換から保存までを固定したものである。後続のlocalhost統合試験で実HTTP clientからWindows Credential Managerまでの結合もPASSしたが、HTTPS / WSS transportを完成させたものではない。

### Rust product pairing → Windows Credential Manager integration — PASS

- Date: 2026-08-12
- Path: Node localhost Relay `/pair` → Rust loopback HTTP transport → `moe-relay-client` → `moe-credential-store` → Windows Credential Manager
- Endpoint boundary: `http://127.0.0.1:<nonzero-port>`だけを許可し、hostname、path、HTTPS、公開hostを拒否
- Pairing code: private environment variableからRustが読み、直後にprocess環境から除去。command line、stdout / stderr、証跡へ出さない
- Rejection: 誤codeをRust transportが`InvalidCode`へ対応付け、credential保存へ進まず終了。その後に正codeで成功
- HTTP body: requestと最大2048 bytesのresponseを消去対象bufferへ閉じ、response内credentialをborrowして製品`PairingResponse`へ移す
- Product path: `RelayConnectionManager::pair`がresponseを受け、`PlatformCredentialStore`へ直接保存
- Web boundary: credentialはTauri command、event、React state、WebView、Node orchestratorへ返さない
- Windows persistence: probe専用のversioned targetへ保存済みであることを確認
- Cleanup: managerによる削除後not found、さらに別cleanup processでもdelete / not foundを再確認
- Relay storage: 生pairing code、生device credentialの保持なし
- Public network / login / manual UI: none
- Evidence: `spikes/relay-roundtrip/evidence/rust-product-pairing-latest.json`

これでlocalhostに限り、pairing code発行から製品Rust境界を通ったOS credential保存・削除までが一本になった。次はcredential storeからloadした値を使うRust Desktop connection transportをNode Relay `/desktop-link`へ結び、認証済みhello、Room read、切断、削除後の再接続拒否を統合試験する。常設HTTPS / WSSや公開Relayはその後に別trancheで扱う。

### Rust stored credential → authenticated Desktop connection integration — PASS

- Date: 2026-08-12
- Path: Windows Credential Manager → `RelayClientService::connect` → Rust HTTP chunked NDJSON transport → Node localhost Relay `/desktop-link` → Room response
- Credential load: 製品service内部のmanagerだけが`SecretBytes`をloadし、transportへ寿命付き借用値として渡す
- Authorization: secretを`String`やheader mapへ複製せず、借用bufferからloopback socketへ直接write
- Handshake: paired device credentialと同じdevice IDでhelloを送り、Relayが`paired-device`として`hello_ack`を返した
- Room: Relayの`moe_read_room` requestへRustがresponseし、`RUST_PRODUCT_CONNECTION_OK`を取得
- Stream boundary: response header / chunk / NDJSON frameを各8 KiB以下に制限し、未知method・Roomを拒否
- Lifecycle: serviceがconnectedを報告し、managed connection drop後にofflineへ戻ることを確認
- Deletion: managerがcredentialを削除し、削除後の`connect`はnetworkへ触れる前に`CredentialNotStored`で拒否
- Cleanup: 別processでもprobe targetのdelete / not foundを再確認
- Web / public boundary: credentialはWebView、Tauri command、Node orchestrator、証跡へ出ず、公開networkは不使用
- Evidence: `spikes/relay-roundtrip/evidence/rust-product-connection-latest.json`

これでlocalhostでは、pairing、OS保管、OSからのload、認証Desktop link、Room応答、切断、削除までが製品serviceを通って一本になった。残るtransport上の大きな境界は、TLS/WSSまたは同等のproduction transport、常設Relay認証record、rotation、複数device / account、公開運用である。

### Product Relay lifecycle service / metadata-only Tauri status — PASS

- Date: 2026-08-12
- Service: `RelayClientService<CredentialStore>`がaccountごとのoffline / connecting / connected / errorを保持
- Managed connection: 接続handleのdrop時にtransport connectionを先にdropし、statusをofflineへ戻す
- Concurrency: connecting / connected中の同一accountへの二重接続をnetwork前に拒否
- Safe errors: credential missing、secure storage、pairing、Relay拒否・停止、protocol、cancelを固定enumへ分類。OS code、target、server messageはstatusへ含めない
- Credential status: `contains`を使い、secretをRust bufferへloadせず保存有無だけを取得
- Tauri state: Desktopが`RelayClientService<PlatformCredentialStore>`を保持
- Tauri command: `relay_connection_status(accountId)`
- WebView response: account ID、`offline|connecting|connected|error`、credential stored、固定last error codeだけ
- Not exposed: connect、pair、store、load、delete、secret input command
- Contract tests: Relay client 18 PASS、Desktop backend 3 PASS
- Actual Windows integration: serviceがconnected、managed drop後offline、credential削除後error / stored falseを報告し、別cleanup processもPASS

### Product Relay runtime lifecycle / Desktop ownership — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0005-relay-connection-lifecycle.md`
- State machine: clock / thread / network I/Oを持たない`RelayLifecycle`がoffline、connecting、connected、retry waiting、stopping、errorを管理
- Actions: start connection、close connection、schedule retry、cancel retryをtransport runtimeへ要求する値として分離
- Retry: 1秒、2秒、5秒、10秒、30秒の5回まで。成功、手動stop、新しい明示startでreset
- Retry refusal: credential不足、secure storage障害、Relay認証拒否、cancelは自動再試行しない
- Unexpected disconnect: unavailable / protocol failureだけを同じbounded retryへ移す
- Desktop owner: accountごとの型消去handleを保持し、二重所有を拒否。stop、shutdown、owner dropで確実に破棄
- Safe metadata: phase、retry回数、次の待機時間、固定error codeだけ。secret、OS code、target、server messageは含めない
- Not exposed: start / stop Tauri command、timer、network thread、production transport
- Contract tests: Relay client 26 PASS、Desktop backend 7 PASS

### Desktop Relay runtime executor — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0006-relay-desktop-runtime-executor.md`
- Managed state: `DesktopRelayRuntimeExecutor`をTauri backendだけで保持
- Action mapping: start connection、close connection、schedule retry、cancel retryを実background taskへ写像
- Connection events: connected、initial failure、unexpected disconnectを固定error codeと共に区別
- Real timer: condition variableによる待機で、30秒retryも手動cancel時にdeadlineを待たず終了
- Ownership: accountごとに1 task。二重startは2本目のthread生成前に拒否
- Generation: stop / restart後に遅れて届いた古いtimer・connection eventを破棄
- Shutdown: 全taskへcancelを通知し、worker threadをjoin。executor dropも同じfallbackを持つ
- Safe event: account ID、generation、固定event kind、固定error codeだけ。secret、OS code、target、server messageは含めない
- Not exposed: start / stop Tauri command、production network driver、接続先入力
- Contract tests: Desktop backend 12 PASS、Relay client 26 PASS

### Product Relay orchestrator / automatic reconnect integration — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0007-relay-runtime-orchestration.md`
- Path: Windows Credential Manager → `RelayClientService` → `DesktopRelayOrchestrator` → Rust chunked NDJSON → Node localhost Relay → disconnect → automatic retry
- Orchestration: 内部event pumpがUI / status pollなしでexecutor eventを状態機械へ戻し、unexpected disconnectから1秒timerと次のconnection taskを自動生成
- Authentication: retry後もWindows credentialを製品serviceが直接loadし、借用secretからAuthorizationへwrite
- Generation: 初回接続と再接続が異なるgenerationであり、古いeventは次のtaskへ作用しない
- Cancellation: cancel hookがblocking socket readへ`Shutdown::Both`を発行し、stopから1秒未満でjoin
- Room: 初回接続で`RUST_PRODUCT_CONNECTION_OK`をRelayへ応答後、切断と再接続を確認
- Status command: runtime phase、retry attempt、next retry delay、固定error codeをmetadata-onlyで返す
- Cleanup: stop後service / orchestratorがoffline、probe credential削除、削除後の接続拒否、別cleanup processもPASS
- Secret boundary: pairing code / device credentialはcommand line、stdout、stderr、Node orchestrator、WebView、証跡へ出ない
- Network: `127.0.0.1`だけ。公開networkは不使用
- Evidence: `spikes/relay-roundtrip/evidence/rust-product-connection-latest.json`
- Contract tests: Relay client 27 PASS、Desktop backend 15 PASS、Rust probe 3 PASS

### Production Relay HTTPS transport boundary — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0008-production-relay-tls-transport.md`
- Product crate: `crates/moe-relay-transport/`
- TLS: `rustls` safe protocol defaults + ring provider + `rustls-platform-verifier`。Windowsの信頼済み証明書、hostname、失効方針を使う
- Endpoint: `https://<DNS hostname>[:port]/desktop-link`だけ。HTTP downgrade、IP literal、userinfo、query、fragment、別pathをnetwork前に拒否
- Authentication: device credentialを`String`やheader mapへ複製せず、借用sliceからTLS streamへ直接write。header unsafe byteはnetwork前に拒否
- Handshake: HTTP/1.1 chunked NDJSONでhello / `hello_ack`を検証し、protocol versionは`moe-protocol`の単一定義を参照
- Bounds: response header / trailer、chunk、NDJSON frameは各8 KiB以下。401 / 403だけを固定`Rejected`へ分類し、server bodyやTLS詳細を返さない
- Timeout: resolved addressごとのTCP connect timeout、TLS handshakeと各read/write操作のdeadlineを実装。標準DNS resolver自体の全体deadlineは未決定
- Cancellation: 同じsocket handleのnonblocking I/Oが5 ms以内にcancel flagを観測し、`Shutdown::Both`と合わせてblocking TLS readを1秒未満で解除
- Local TLS fixture: trusted certificate成功、untrusted certificate拒否、hostname mismatch拒否、401分類、oversized header / frame拒否、stalled response timeout、blocking read cancelがPASS
- Public network / real credential / UI: none。生成CAと固定test credentialだけを使用
- Contract tests: Relay HTTPS transport 10 PASS

これで公開Relayへ接続するためのTLS client境界は製品crateになった。この時点ではDesktop orchestratorのtask factoryへの結合と公開Relayが未実装だったため、次の統合で前者を扱った。

### Windows credential → product HTTPS → Desktop runtime integration — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0009-desktop-product-relay-bootstrap.md`
- Product path: build-time Relay metadata → `RelayClientService<PlatformCredentialStore>` → `RelayHttpsTransport` → `DesktopRelayOrchestrator`
- Configuration: endpoint、account ID、device IDの3項目がそろったbuildだけが起動時に自動start。全項目未指定はnetworkなし、不完全・HTTP・不正identityは起動前に拒否
- Secret boundary: build metadata、environment、command line、Tauri command、WebViewへcredentialを渡さず、各connection taskがWindows Credential Managerから直接load
- Authentication: 生成CAのlocalhost TLS Relayが2世代とも同じtest credentialとdevice IDのAuthorization / helloを確認
- Reconnect: 初回TLS接続 → server切断 → `RelayUnavailable` → 1秒retry → 別generationの再TLS接続をUI pollなしで確認
- Cancellation: TLS shutdown handleをDesktop cancellationへ登録し、2本目のblocking readをstopから1秒未満で解除・join
- Service status: TLS handshake後connected、stopとmanaged connection drop後offline
- Real Windows store: 隔離した一時targetへstoreし、製品serviceから2回load、TLS認証、stop後delete、not foundを確認
- Cleanup: test targetは削除済み。公開network、実credential、WebView、UI操作なし
- At ADR 0009 completion: `hello_ack`後のframeはRoom router未実装のためprotocol errorとして切断。未検証payloadを破棄・転送しない

この段階でDesktopは、信頼済みbuild設定がある場合にproduction HTTPS connectionを自動所有できるようになった。直後のADR 0010で `moe_read_room` の製品routerを追加した。公開Relayとpairing UIはその後も別境界である。

### Product HTTPS Room read router — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0010-product-room-read-router.md`
- Protocol: strictなrequest envelope、`moe_read_room` params、success / error responseを `moe-protocol` に固定。未知field、不正ID、範囲外limitを拒否
- Correlation: responseは元のrequest IDを保持。1接続256 IDまで、重複は `duplicate_request`、未知methodは `unsupported_method`、不正paramsは `invalid_request`
- Core model: provider-neutralなRoom、participant、message、宛先、生成日時、Artifact ID、read query / result、`RoomSource` traitを追加
- Read semantics: cursorなしは末尾から最大30件、cursorありは次のmessageから取得。Room / cursor not foundは安全な結果として返す
- Desktop source: 現行Reactデモと同じ初期参加者・3 messageを持つ、不変のbackend bootstrap snapshot
- HTTPS integration: 生成CAのlocalhost TLS Relayが2世代目の接続で実 `moe_read_room` を送り、`welcome-3` とnext cursorを持つ相関済みresponseを受信
- Lifecycle: 初回切断、1秒retry、別generationでの再認証、Room response、stop時cancel / joinを同じ試験で確認
- Real Windows store: 隔離targetへ保存したcredentialを製品serviceがloadし、同じHTTPS Room response経路を通過後にdelete / not foundを確認
- Rejection: malformed requestはconnection-level protocol error。未知method、不正params、重複IDは固定response。cancel観測後はresponseを書かない
- External effects: 公開network、実credential、WebView、UI操作なし

この時点のPASSは、RelayからDesktopの初期Room snapshotを安全に読めることを示した。直後のADR 0011で初期開発室のreadだけをReact UIへ結合したが、UI送信内容のlive同期、message write、永続化、Artifact、8 KiBを超える結果のbyte-budget paging / backpressureは次の境界である。

### Desktop Room read command / UI hydration — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0011-desktop-room-read-ui-hydration.md`
- Core input: Room / cursor IDとlimit 1から30を `RoomReadQuery::try_new` で検証。HTTPS routerとTauri commandが同じ境界を使用
- Tauri surface: `desktop_room_read(roomId, afterMessageId, limit)` だけを公開。filesystem path、secret、任意source、無制限queryなし
- Web validation: unknownをsuccess shape、participant、message、string arrayまで検査してからUI modelへ変換
- Participant mapping: backendのID / 表示名 / human・AIを正本とし、UI固有のlabel / initials / accentだけをcatalogから補完
- Failure behavior: browserでは従来デモ、Tauri読取失敗は内容を維持しつつ `Room offline`。成功時だけ `Core + Room ready`
- Browser preview: 3室表示、Room切替、送信、650 ms後のダミー応答を実画面で確認
- Tauri debug UI: `target/debug/moe-desktop.exe` を起動し、`Core + Room ready`、参加AI 3人、Rust source由来の初期3 messageと時刻を目視確認
- Process isolation: 既存の `target/release/moe-desktop.exe` は操作・停止せず、検証用debug / Cargo watcher / Viteだけを明示PIDで終了
- Public network / credential / Provider login: none

この段階で初期開発室のreadはUIとRelayが同じRust sourceを見るようになった。直後のADR 0012で開発室のユーザーmessage writeを結合した。開発室以外の2室、新規Room、参加者追加、ダミー応答はまだReactローカルstateである。

### Idempotent Desktop Room message write — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0012-idempotent-room-message-write.md`
- Core store: `RwLock<RoomSnapshot>` を共有する `RoomStore::append_message` と既存 `RoomSource::read_room`
- Idempotency: client-generated message ID、Room、author、宛先、本文、Artifactが同じ再試行は `duplicate` で既存messageを返す。内容変更は `messageConflict`
- Validation: restricted ID、空本文拒否、Core 4,000 bytes / UI 1,000文字、宛先重複拒否、Room参加者参照、最大100宛先、Room最大10,000 message
- Ownership: WebViewはRoom、message ID、宛先、本文だけを送る。authorは `owner`、Artifactは空、UTC RFC 3339時刻はRustが付与
- UI success: Rust response後だけmessageを追加し、入力欄をclear。Tauri開発室ではダミーAI返答なし
- UI failure: 本文を残し、入力欄直下に再試行可能と表示。同じpayloadの再試行は同じmessage ID
- Response correlation: WebViewはresponseのmessage ID、Room、author、宛先、本文、Artifactがrequestと一致する場合だけ表示。Tauri Room loading / offline時はローカル送信へfallbackしない
- Browser preview: `Preview ready`、ローカル送信、Codexダミー応答を実画面で確認
- Tauri debug UI: `Rust Room write UI PASS` を保存し、13:42のRust時刻で表示。ダミーAI返答なし、成功後入力欄clear
- Rehydration: 同じdebug processでWebViewを再読込みし、保存messageがRust Room readから再表示されることを確認
- Relay observation: write後に `welcome-3` cursorから製品Room routerを読み、同じ追加messageを取得するcontract test PASS
- Cleanup: debug app / Cargo watcher / Viteだけを停止し、既存release appは維持、port 1420閉鎖
- Public network / credential / Provider login: none

このPASSで開発室のユーザーmessageは画面だけの追加ではなくなった。直後のADR 0013でCodex AI dispatchを別idempotency境界として開始し、responseを同じRoomへ追記した。process restart後の永続化と他2室の移行は別trancheで扱う。

### Codex Room AI dispatch — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0013-codex-room-ai-dispatch.md`
- Neutral contract: `moe-adapter-sdk::TextTurnAdapter` がdispatch ID、text prompt、final text、固定errorだけを扱い、Codex固有eventをCore / UIへ流さない
- Launcher: `MOE_CODEX_BIN`、`MOE_CODEX_CLI_JS`、global npm Codex、PATH fallback。WindowsApps版の直接起動拒否後、既存Phase 0 probeと同じglobal npm経路で製品live smoke PASS
- Protocol: initialize、permission profile list、ephemeral thread、turn start、stream / completed agent message。stdout 1 MiB / line、request 30秒、turn 180秒、reply 4,000 bytes
- Permission: 動的 `moe-room-text-only` profileで `:minimal` と空の専用workspaceだけをread、network無効、writeなし。approval `never`、server requestはdeny
- Prompt boundary: base / developer instructionsでRoom本文をuntrusted textとし、tool、command、file、MCP、browser、networkを禁止。日本語会話本文だけ、800文字以下を要求
- Idempotency: source message ID + recipient IDのprocess内ledger。in-progress拒否、completedは同じ保存replyを返し、failedは外部turnを自動再試行しない
- Unsupported: Claude Web / Geminiは `unsupported`。実返信やダミー返信を作らず、UIへ未接続を表示
- UI: Rust保存後に入力clear、`Codex が考え中` / `応答待ち`、完了後に同じRoomへreply。失敗時は保存済みと自動再送なしを入力欄直下へ表示
- Live adapter smoke: 実Codex App Serverが固定 `MOE_CODEX_ROOM_LIVE_OK` を返しPASS
- Tauri debug UI: 14:12に `M.O.E.から実Codex応答テストです。短く返事してください。` を送信し、`[OWNER_DISPLAY_NAME]、受信できています。実Codex応答テスト成功です！` を受信
- Rehydration: WebView再読込後、user / Codex両messageをRust Room readから復元。実Codex replyに `UI DEMO` 札なし
- Cleanup: debug PID、Vite親・port ownerだけを停止。既存release PID 14448を維持し、port 1420閉鎖
- Persistence boundary: ADR 0014でRoomは永続化。dispatch ledger、external refs、継続session、streaming表示、interrupt UI、利用枠表示は未実装

### Persistent Desktop Room snapshot — PASS

- Date: 2026-08-12
- ADR: `docs/decisions/0014-persistent-desktop-room-snapshot.md`
- Format: `fileVersion: 1` + provider-neutral `RoomSnapshot`。Core protocol version、participant / Room / message参照、ID、件数、body上限をload時に再検証
- Product path: Tauri `app_data_dir()` / `room-snapshot-v1.json`。WebViewへfilesystem path指定commandを公開しない
- Bounds: primary / backupとも64 MiB。unknown field、未知file version、破損JSON、不正snapshot、非file pathを拒否
- Commit path: 同一directoryの排他的tempへwrite + `sync_all`、primaryを1世代backupへrename、tempをprimaryへrename。primary欠落 + backup存在は切替途中としてbackupを読む
- Failure path: Desktop transaction mutexでread / find / append / persistを直列化。永続化失敗は書込み前snapshotへmemory rollbackし、Tauri / AI / Relayへ未確定messageを見せず成功も返さない。破損primaryは黙って上書きせず起動失敗
- Automated restart: source再生成後にuser message復元、同じID / payloadが `duplicate`。primary欠落時のbackup復旧、破損primary拒否、書込み不能pathでmemory rollbackを確認
- Tauri restart UI: 隔離 `MOE_ROOM_DATA_FILE` で `Room永続化の再起動テストです。短く返事してください。` を14:42にGemini宛で保存。debug process終了後、同じfileで再起動し、message / To Gemini / `Core + Room ready` を実画面で確認
- External effect: Geminiは未接続のため外部AI送信なし。検証後debug / Vite / port ownerを停止し、隔離data / log 7点を削除。既存release PID 14448を維持、port 1420閉鎖
- Superseded by ADR 0015-0017: 全Room catalog / mutation / delete、primary recovery、明示backup / latest restoreまで実装済み。残りはschema migration、任意import / retention、複数process writer、永続dispatch ledger、incremental database
