# 0012: Idempotent Room message write

Status: Accepted

Date: 2026-08-12

## Context

ADR 0011で初期開発室のreadはReact UIとRelayが同じRust sourceを見るようになったが、送信はReactローカルstateへ追加して650 ms後にダミーAI返答を出すだけだった。UIからRustへwriteを追加する際、保存成功後にcommand responseだけ失われると、単純な再送は同じ発言を二重追加する。

AI dispatch、課金、永続DBはまだ未実装である。今回はユーザーmessageを一度だけRoomへ追加し、UI再読込みとRelay readの両方から観測できる境界を固定する。

## Decision

- `InMemoryRoomSource` のsnapshotを `RwLock` で保護し、`RoomStore::append_message` と既存 `RoomSource::read_room` が同じ正本を共有する。
- WebViewが生成するmessage IDをidempotency keyとして使う。同じID、Room、author、宛先、本文、Artifact IDの再試行は既存messageを `duplicate` として返し、追加しない。同じIDで内容が異なる場合は `messageConflict` を返す。
- `desktop_room_write_message` commandが受け取るのはRoom ID、message ID、宛先ID、本文だけとする。authorは `owner`、Artifactは空、生成時刻はRustがUTC RFC 3339で付与する。
- Coreはidentifier、重複宛先、空本文、最大4,000 bytes、最大100宛先、Room参加者参照、Room最大10,000 messageを検証する。UIはさらに本文1,000文字で制限する。
- Tauri開発室では保存成功後だけRustが返したmessageをReactへ追加する。保存失敗時は本文を入力欄へ残し、入力欄直下に再試行可能と表示する。同じ本文・Room・宛先の再試行は同じmessage IDを使う。
- WebViewはwrite responseのshapeに加え、message ID、Room、author、宛先、本文、Artifactが送信requestと一致することを確認してから表示する。Tauri Roomがloading / offlineの場合はローカル送信へfallbackせず、送信buttonを無効化する。
- Tauri開発室ではダミーAI返答を出さない。browser previewと未移行の他2室だけは従来のローカル送信・ダミー返答を維持し、hintで区別する。
- in-memory sourceのため、WebView再読込みではmessageを保持するが、Desktop process終了後の永続化は保証しない。

## Consequences

- UI送信 → bounded Tauri command → Rust Room write → UI表示 → WebView再読込み → Rust Room readが一本になり、Relay routerも同じ追加messageを読める。
- command response消失後の再試行で二重messageを作らない。将来のAI dispatchでは、このmessage IDとは別にdispatch / turn idempotencyを追加する必要がある。
- 実Tauri debug画面で `Rust Room write UI PASS` を保存し、ダミーAI返答が出ないこと、入力欄が成功後に空になること、WebView再読込み後もRust sourceから再表示されることを確認した。
- Room永続化、process restart後の復元、他2室の移行、Room / participant mutation、AI dispatch、書込みのRelay protocol化は未完である。

## Follow-up

ADR 0013で、Codex宛messageの別dispatch key、実Codex App Server turn、最終応答の同じRoomへの保存、UIの応答待ち・失敗表示を実装した。ADR 0014でRoom snapshotとmessage idempotency payloadをprocess restart後も復元する。ほか2室、Claude Web / Geminiは引き続き未完である。
