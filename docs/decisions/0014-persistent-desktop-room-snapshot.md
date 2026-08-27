# 0014: Persistent Desktop Room snapshot

Status: Accepted

Date: 2026-08-12

## Context

ADR 0012 / 0013で開発室のユーザーmessageとCodex replyは同じRust Roomへ保存されるようになったが、sourceはprocess内 `RwLock` だけであり、Desktop終了時に履歴とmessage IDのidempotency基準が失われていた。最初の永続化でSQLiteなどの新依存、migration runner、複数process writerまで同時に導入すると、現在の単一Room / 単一Desktop writerに対して境界が大きすぎる。一方、単純なJSON上書きは書込み中の終了で履歴全体を壊す。

## Decision

- `RoomSnapshot`、`Room`、`RoomMessage`、`RoomParticipant`をprovider-neutralなversioned JSONへ直列化する。外側に `fileVersion: 1` を持ち、Core protocol versionと全参照整合性を起動時に再検証する。
- 製品pathはTauri `app_data_dir()` 配下の `room-snapshot-v1.json` とする。`MOE_ROOM_DATA_FILE` は隔離した製品経路試験だけの明示overrideとし、WebViewからpathを指定するcommandは作らない。
- fileは64 MiBを上限とし、unknown field、未知file version、不正JSON、不正snapshot、非file pathを拒否する。primaryが存在するのに不正な場合はbundled snapshotやbackupで黙って上書きせず起動をfail closedする。
- 保存は同一directoryに排他的temp fileを作り、全byteを書いて `sync_all` した後、既存primaryを1世代backupへrenameし、tempをprimaryへrenameする。primaryがなくbackupだけがある場合は、切替途中の終了としてbackupを読む。
- `DesktopRoomSource` がtransaction mutexを所有し、read / findとappend、snapshot取得、永続化を直列化する。永続化に失敗した場合は書込み前snapshotへmemoryを戻し、Tauri / AI dispatch / Relayへ未確定messageを見せず成功も返さない。
- bundled初期snapshotは永続fileが一度も作られていない時だけ使う。ユーザーmessageとAI replyは同じ `RoomStore::append_message` を通るため、両方が同じ永続境界へ入る。
- 現段階は単一Desktop processのfull-snapshot保存とする。複数process writer、部分更新、migration UI、履歴export / delete、暗号化、永続dispatch ledgerは別判断とする。

## Consequences

- Desktop processを終了しても、Room履歴、Codex reply、message IDとidempotent payloadが次回起動へ復元される。
- 自動試験で保存後のsource再生成、restart後のduplicate、primary欠落時のbackup復旧、破損primary拒否、file書込み失敗時のmemory rollbackを確認した。
- 実Tauri debug画面では、隔離fileへ `Room永続化の再起動テストです。短く返事してください。` をGemini宛で保存し、process終了後に同じfileで再起動して14:42のmessageが復元されることを確認した。未接続Gemini宛のため外部AI送信は発生していない。
- backupは直前の成功snapshot 1世代であり、primary自体が壊れて存在する場合の自動回復はしない。ユーザー向けrecovery / export UI、schema migration、複数Room mutationは後続trancheで扱う。
