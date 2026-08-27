# 0013: Codex Room AI dispatch

Status: Accepted

Date: 2026-08-12

## Context

ADR 0012でユーザーmessageはRust Roomへidempotentに保存されるようになったが、Tauri開発室はAIへ配送せず、応答も存在しなかった。単純に保存command内で外部turnを開始すると、command response消失やtimeout後の再試行で同じ利用枠を二重消費する。また、通常のCodex coding sessionをそのままRoomへ使うと、会話本文からローカルfileやtoolへアクセスできる境界になる。

Claude WebとGeminiの公式製品接続は未確定である。最初の実participantには、Phase 0-AでWindows上のhandshake、stream、interrupt、approval、resumeを確認済みのCodex App Serverを使う。

## Decision

- provider-neutralな `TextTurnAdapter` 契約を `moe-adapter-sdk` に追加し、Codex JSON-RPC型やeventをCore / UIへ流さない。
- Codex製品driverはstdio App Serverを子processとして起動する。`MOE_CODEX_BIN`、`MOE_CODEX_CLI_JS`、global npm版、最後にPATHの順でlauncherを選ぶ。stdoutは1行1 MiB、request 30秒、turn 180秒、response 4,000 bytesで制限し、終了・失敗・timeout時は子processを停止する。
- App Serverへexperimental permission-profile capabilityを宣言し、利用可能な `:read-only` profileをpreflightする。threadには動的な `moe-room-text-only` profileを渡し、`:minimal` runtimeと空の専用workspaceだけをread、network無効、writeなしとする。approval policyは `never`、serverからapproval requestが来ても拒否する。
- threadはephemeralとし、base / developer instructionsでRoom本文をuntrusted textとして扱い、tool、command、file、MCP、browser、networkを使わず日本語の会話本文だけを返すよう固定する。
- dispatch keyはsource message IDとrecipient IDの組とし、process内ledgerを `inProgress` / `completed` / `failed` で管理する。completed再試行は保存済みreplyを返し、in-progressとfailedは新しい外部turnを開始しない。process restartをまたぐexactly-onceは保証しない。
- `desktop_room_dispatch_message` が保存済みのhuman messageだけを読む。Codex宛は実turnを開始し、最終agent messageを決定的なreply ID、author `codex`、recipient `owner`で同じRoomへappendする。Claude Web / Geminiは `unsupported` とし、ダミー返信を作らない。
- UIはユーザーmessageのRust保存成功時点で入力をclearし、その後を `Codex が考え中` / `応答待ち` と表示する。dispatch失敗時も保存済みmessageを再送せず、入力欄直下に「自動再送していない」と表示する。
- TypeScriptはdispatch responseのsource ID、recipient順、status、replyのRoom / author / recipient / body / Artifactを相関検査する。初期welcome AI messageだけを `UI DEMO` とし、実Codex replyへdemo表示を付けない。

## Consequences

- Tauri開発室で、ユーザー送信 → Rust保存 → 実Codex App Server → Codex最終応答 → Rust Room保存 → UI表示が一本になった。同一processのWebView再読込み後も両messageをRust readから復元できる。
- 実Codex live smokeで固定markerを受信した。実Tauri debug画面では `M.O.E.から実Codex応答テストです。短く返事してください。` に対し、Codexから `[OWNER_DISPLAY_NAME]、受信できています。実Codex応答テスト成功です！` を受信し、応答待ち解除、demo札なし、再読込復元を確認した。
- 外部turnが開始された可能性のある失敗を自動retryしないため、無言の二重利用を避ける一方、同じmessageの手動retry UIはまだない。
- ADR 0014でRoom履歴とmessage idempotency payloadはDesktop再起動後も復元される。Codex dispatch ledger自体はprocess内のため、継続session、streaming表示、interrupt UI、token / 利用枠表示、Claude Web / Gemini、複数同時dispatchとともに後続trancheで扱う。
- ADR 0027でこのprocess内ledgerをdevice-localな永続台帳へ置き換え、外部送信開始後の結果不明状態を自動再送しない契約を追加した。
