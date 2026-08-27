# 0011: Desktop Room read UI hydration

Status: Accepted

Date: 2026-08-12

## Context

ADR 0010でRust backendの読み取り専用Room sourceとHTTPS routerは成立したが、React画面は `useDemoRooms` のローカル初期値だけを表示していた。RelayとUIが同じ開発室を見ていることを保証できず、backend読取が壊れても画面上ではデモが正常に見える状態だった。

一方、現在の新規Room、参加者追加、message送信、ダミー応答はReactだけの操作である。読取と書込を一度にRustへ移すと、永続化・idempotency・AI dispatchまで同時に決める必要があるため、まず初期開発室のhydrationだけを製品commandへ結ぶ。

## Decision

- `moe-core::RoomReadQuery` はRoom ID、cursor ID、limit 1から30を構築時に検証する。HTTPS routerとTauri commandの両方が同じCore検証を通る。
- Desktop backendへ `desktop_room_read` commandを追加する。WebViewへfilesystem path、secret、任意source、無制限queryを受け取る面は作らない。
- Reactのhookを `useRooms` とし、Tauriでは起動時に `moe-dev-room` を最大30件で読み、成功responseをTypeScript側でも検査してから表示modelへ変換する。
- backend participantのID、表示名、kindを正本とし、service label、initial、accentはUI catalogから補う。未知participantには固定の安全な既定表示を使う。
- browser previewでは従来の3室デモを維持する。Tauri読取に失敗した場合も内容を消さず、footerを `Room offline` にして失敗を隠さない。
- footerはTauri hydration成功時だけ `Core + Room ready`、browser previewでは `Preview ready` と表示し、デモfallbackと製品読取を視覚的に区別する。
- この段階では開発室以外の2室、新規Room、参加者追加、message送信、ダミー応答をReactローカルstateのまま維持する。

## Consequences

- Tauri実画面の初期開発室は、Relayと同じRust Room sourceから読むようになり、初期messageの二重定義による表示driftを一段減らせる。
- invalid Room ID、path風ID、不正cursor、limit 0 / 31以上はRoom sourceへ到達する前に拒否される。
- browser previewの外観と既存操作は維持される。実Tauri debug画面で `Core + Room ready`、参加AI 3人、初期3 message、時刻を目視確認した。
- UI writeはまだbackendへ反映されず、Relayからも読めない。次はRoom mutation command、client-generated idempotency key、書込後snapshot更新、失敗表示を同じ境界で設計する。

## Follow-up

ADR 0012で、開発室のユーザーmessage write、client-generated idempotency key、書込後UI更新、失敗時の再試行表示を実装した。AI dispatchと永続化は引き続き未完である。
