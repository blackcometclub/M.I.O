# 0007: Relay runtime orchestration and authoritative status metadata

Status: Accepted

Date: 2026-08-12

## Context

ADR 0005の状態機械とADR 0006のDesktop executorは個別に成立していたが、executor eventを状態機械へ戻し、retry actionから次の接続taskを生成する自動運転経路は未結合だった。また、blocking socket readを通常終了まで待つだけでは、手動stopの即時性を保証できない。

## Decision

- Tauri managed stateは、executorとaccountごとの状態機械・接続task factoryを所有する`DesktopRelayOrchestrator`とする。
- orchestratorは内部event pump threadを持ち、WebView、status command、integration probeによるpollがなくてもlifecycleを進める。
- orchestratorはconnected、initial failure、unexpected disconnect、retry elapsed eventを状態機械へ戻し、返されたactionを次のtaskまたはtimerへ自動的に写像する。
- runtime task生成失敗は固定code `runtimeUnavailable`のterminal errorとし、OS errorやthread生成messageをmetadataへ含めない。
- cancellation tokenはcancel hookを登録できる。socket transportはcloneしたsocketへのshutdownをhookに登録し、blocking readをstop時に中断してからworkerをjoinする。
- `relay_connection_status(accountId)`はruntimeがoffline以外の間、orchestrator statusをauthoritativeとする。responseへretry attemptとnext retry delayを追加する。
- Rust moduleはlocalhost integration probeから同じ製品orchestratorを利用できるよう公開するが、Tauri command surfaceにはstart、stop、pair、credential、接続先入力を追加しない。
- localhost probeでWindows Credential Managerからcredentialをloadし、認証接続、Room応答、unexpected disconnect、1秒retry、再認証接続、generation更新、socket cancel、offline復帰、credential削除を一本で検証する。

## Consequences

- lifecycleとexecutorがUI非依存の製品backend経路として自動運転され、再接続は設計だけでなくlocalhost上の実socketで確認済みとなる。
- status UIはsecretを受け取らずにretry waiting、stopping、retry回数、次の待ち時間を表示できる。
- localhost HTTP chunked NDJSON transportは検証driverであり、production transportの採用決定ではない。
- 次はTLS、hostname検証、cancel可能なI/O、connect/read/write timeout、frame上限を満たすproduction driverを選定・実装する必要がある。
