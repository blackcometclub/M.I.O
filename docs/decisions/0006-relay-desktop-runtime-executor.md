# 0006: Relay Desktop runtime executor and cooperative cancellation

Status: Accepted

Date: 2026-08-12

## Context

ADR 0005でRelay lifecycleとretry規則をtransport非依存の状態機械として固定した。次にDesktop backendは、そのactionを実際のtimerと接続taskへ写像し、stopやアプリ終了時にbackground workを残さない必要がある。

production transport、接続先設定、pairing UXは未決定であるため、特定のHTTP / WebSocket libraryや公開networkにはまだ結合しない。

## Decision

- Desktop backendに`DesktopRelayRuntimeExecutor`を置く。後続ADR 0007で、Tauri managed stateはexecutorを内包するorchestratorへ昇格した。
- executorは状態機械のstart、close、schedule retry、cancel retry actionだけを受け取り、WebView commandとして公開しない。
- accountごとに同時に1つのbackground taskだけを所有し、二重startをtask生成前に拒否する。
- 接続taskはcooperative cancellation tokenを受け取り、connected、initial failure、unexpected disconnectを固定eventとして返す。
- retryは実timer taskで待機し、cancel tokenとcondition variableによりdeadline前でも即座に終了できる。
- taskとtimerには単調増加するgenerationを割り当てる。stopまたは新しいtaskの後に到着した古いeventは破棄する。
- terminal event、明示stop、shutdown、executor dropでtask handleをdropし、cancel後にworker threadをjoinする。
- runtime eventに含めるのは検証済みaccount ID、generation、固定event kind、固定error codeだけとする。credential、OS error、target、server messageは含めない。
- 最小executorはRust標準libraryのthread、channel、condition variableを使う。production transport導入時に内部実装をasync runtimeへ置き換えても、action / event / cancellation契約は維持する。

## Consequences

- 実clockとbackground threadを使って、retry発火、即時cancel、shutdown join、初回失敗と接続後切断の区別をcontract testできる。
- Rust threadを外部から強制終了しない。接続taskはcancel tokenを監視し、socketやIPC待機にもtimeoutまたはinterrupt可能なI/Oを使わなければならない。
- cooperative cancellationを無視するtransportはshutdownを止め得るため、production transportの採用条件を満たさない。
- start / stop Tauri command、接続先入力、credential操作、公開Relay通信は引き続き未実装である。
