# 0016: Persistent Desktop Room management

Status: Accepted

Date: 2026-08-12

## Context

ADR 0015で3つの標準Room、ユーザー作成Room、参加AI追加、全Roomのmessage writeを同じversion 1 JSON snapshotへ統合した。一方、名前変更、参加AIの取り外し、不要になったRoomの削除はまだできず、Room catalogを日常的に管理する操作が欠けていた。

既存snapshotではmessageのauthorとrecipientがRoom参加者を直接参照する。履歴が参照している参加者を単純に外すとload時の整合性検証に失敗する。また、標準Roomの誤削除や、AIが一人もいないRoomの作成も避ける必要がある。

## Decision

- `RoomStore` に `rename_room`、`remove_room_participant`、`delete_room` を追加し、Desktopの同じtransaction mutex、temp + backup永続化、失敗時memory rollbackを通す。
- Room名は標準Roomを含めて変更できる。空名、容量超過、不正ID、同値の再試行は既存のvalidation / duplicate契約で扱う。
- ownerの `owner` はTauri commandから取り外せない。AIは最低1人を残す。既存messageのauthorまたはrecipientが参照する参加者は、現在のsnapshot整合性を壊さないよう取り外しを拒否する。
- 標準Room ID `moe-dev-room`、`comparison-room`、`mcp-lab` は削除できない。削除できるのはユーザー作成Roomだけとし、UIは「削除…」から「本当に削除」へ切り替わる二段階操作にする。
- WebViewはRoom ID、変更名、取り外す参加者IDだけを送る。timestampと永続化はRustが所有し、responseのRoom / participant / status相関が一致した場合だけ画面へ反映する。
- browser previewは外部作用のないローカルmutationを維持し、Tauriではbackend障害時にローカル成功へfallbackしない。

## Consequences

- Room設定popoverから名前変更、未参照AIの取り外し、ユーザー作成Roomの削除を行え、再起動後も同じ状態へ復元できる。
- 履歴を保持したまま参加者を外す仕様は未導入である。将来それを必要とする場合は、履歴上のparticipant identityをRoom membershipから分離するschema変更またはtombstoneを別ADRで決める。
- 自動テストはrename / remove / delete、標準Room保護、最低AI数、履歴参照、永続化後のreloadを検証する。実Tauri画面では隔離fileを使い、名前変更、Gemini取り外し、二段階削除表示、process再起動後のRoom名とAI 1人の復元を確認した。実画面の最終削除は行わず、削除永続化はRust自動テストで確認した。
- Room並べ替え、履歴を伴うparticipant tombstone、recovery / export UI、複数process writer、永続dispatch ledger、incremental databaseは引き続き別判断とする。
