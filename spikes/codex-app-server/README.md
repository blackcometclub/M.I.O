# Codex App Server spike

Phase 0-Aで、M.O.E.からCodex App Serverへ接続できることを製品Coreから切り離して検証します。

## 境界

- Transportは最初に `stdio` を使う。
- インストール済みCLIから生成した型だけを使い、手書きのProvider型をCoreへ持ち込まない。
- 最初の検証では `--experimental` を付けない。
- 生成物は `generated/` に置き、Gitにはバージョン、生成条件、ハッシュだけを記録する。
- PoCコードをそのまま製品Coreへ昇格させない。

## Schema生成

```powershell
codex app-server generate-ts --out ./spikes/codex-app-server/generated/<codex-version>/typescript
codex app-server generate-json-schema --out ./spikes/codex-app-server/generated/<codex-version>/json-schema
```

Codex Desktop内のターミナルでアプリ同梱版を先に拾って実行できない場合は、独立CLIの場所を設定から明示できるようにする。M.O.E.本体で探索ロジックを確定するのは、Phase 0-Aで独立CLI、認証、再起動復旧を確認した後とする。

## 次の試験

1. Node.jsクライアントからApp Serverを `stdio` で起動する。
2. `initialize` と `initialized` を完了する。
3. `thread/start`、`turn/start`、stream通知、`turn/completed` を観測する。
4. seeded fixtureを使い、許可範囲、approval、interrupt、再起動後resumeを検証する。
5. sanitized event log、diff、test結果をFeasibility Reportへ記録する。

## Handshake probe

依存追加なしのNode.jsクライアントで、最初の実往復を確認します。

```powershell
npm.cmd run probe:handshake --workspace @moe/spike-codex-app-server
```

通常はWindowsのglobal npmへ入った独立Codex CLIを探索します。別の実行ファイルを使う場合は `MOE_CODEX_BIN`、別のnpm CLI entrypointを使う場合は `MOE_CODEX_CLI_JS` で明示できます。

このprobeはread-only sandboxと `approvalPolicy: never` を指定します。予期しないcommand/file approvalが届いても拒否します。親プロセスの環境・権限制約は解除しません。成功時のsanitized summaryは `evidence/handshake-latest.json` へ保存します。再開試験にだけ必要なthread IDはGit管理外の `.moe/probe-state/` に分離します。

2026-08-11に `codex-cli 0.145.0` で実行し、`initialize`、thread/turn開始、stream通知、`turn/completed`、期待した最終文字列まで確認しました。次のprobeでは、このpersistent threadを別のApp Server processからlist/read/resumeします。

## Restart / resume probe

Handshake probeが保存したpersistent threadを、別のApp Server processから復元します。

```powershell
npm.cmd run probe:resume --workspace @moe/spike-codex-app-server
```

`thread/list` で対象を発見し、`thread/read` で元の応答を確認してから、`thread/resume` と新しい `turn/start` を実行します。成功時のsanitized summaryは `evidence/resume-latest.json` へ保存します。

公開証跡からはthread / turn / item ID、local repository path、server user-agentを自動的に伏せます。匿名化処理そのものは次で単体試験できます。

```powershell
npm.cmd run test --workspace @moe/spike-codex-app-server
```

## localImage probe

推測では分からない固有コードを入れたPNG fixtureを生成し、プロンプトには答えを書かずに画像だけから読み取れるか確認します。

```powershell
powershell.exe -ExecutionPolicy Bypass -File ./spikes/codex-app-server/generate-image-fixture.ps1
npm.cmd run probe:image --workspace @moe/spike-codex-app-server
```

App Serverへ `localImage` として絶対パスを渡し、画像下部のコードを余分な説明なしで返した場合だけPASSとします。threadはephemeral、sandboxはread-onlyです。sanitized summaryは `evidence/image-latest.json` へ保存します。

2026-08-11に `codex-cli 0.145.0` で実行し、プロンプトへ開示していない `NEKOMIMI-42` を画像から正確に読み取ることを確認しました。

## turn/interrupt probe

長い出力を要求するturnを開始し、`turn/started` の直後に `turn/interrupt` を送ります。そのturnが `interrupted` で完了し、同じthreadの次のturnが正常完了した場合だけPASSとします。

```powershell
npm.cmd run probe:interrupt --workspace @moe/spike-codex-app-server
```

threadはephemeral、sandboxはread-onlyです。sanitized summaryは `evidence/interrupt-latest.json` へ保存します。

2026-08-11に `codex-cli 0.145.0` で実行し、最初のturnが `interrupted`、直後の回復turnが `completed` になることを確認しました。

## File-change approval probe

拒否時に変更が適用されず、許可時だけ指定ファイルが変わることを、専用fixtureで連続確認します。

```powershell
npm.cmd run probe:approval --workspace @moe/spike-codex-app-server
```

probeは毎回 `spikes/fixtures/approval-sandbox/baseline/` から、Gitで無視される `runtime/` を作り直します。操作対象をこのruntimeへ限定し、`target.txt` のほかに、絶対に変えてはいけない `sentinel.txt` を置きます。

1回目は `item/fileChange/requestApproval` に `decline` を返し、対象ファイルが元のまま、fileChangeが `declined` になることを確認します。2回目は `accept` を返し、指定した内容だけが反映され、fileChangeが `completed` になることを確認します。両方でsentinel、ファイル一覧、command approvalが発生していないこと、および `item/started` → approval request → `serverRequest/resolved` → `item/completed` の順序を検証します。

2026-08-11に `codex-cli 0.145.0` でdeny/acceptともPASSしました。sanitized summaryは `evidence/approval-latest.json` へ保存します。

## Seeded fixture repair probe

Phase 0-Aの最終条件として、再現可能な不具合を入れた共通fixtureをCodex自身に調査・修正させます。

```powershell
npm.cmd run probe:seeded-fixture --workspace @moe/spike-codex-app-server
```

`spikes/fixtures/seeded-bug-app/baseline/` には、仕様、3件のtest、UI状態を表すSVG、sentinel、配送ロジックがあります。baselineは3件中1件だけ失敗します。probeはGitで無視される `runtime/` を毎回作り直し、Codexへ失敗再現、原因特定、修正、再試験を依頼します。

file-change approvalは `item/started` の `itemId` と変更pathを照合し、`runtime/src/delivery-plan.mjs` だけを許可します。それ以外の書込要求とcommand approvalは拒否します。完了後は初期failure、最終PASS、変更path、全保護ファイルのSHA-256、approval event順、Codexの説明を機械検証します。

2026-08-11に `codex-cli 0.145.0` で実行し、初期2 PASS / 1 FAILから最終3 PASS / 0 FAILへ修復されました。変更は許可したソース1本だけで、仕様、test、SVG、package.json、sentinelは不変でした。sanitized summaryとsource diffは `evidence/seeded-fixture-latest.json` へ保存します。
