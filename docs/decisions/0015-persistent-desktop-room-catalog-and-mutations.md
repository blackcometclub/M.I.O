# 0015: Persistent Desktop Room catalog and mutations

Status: Accepted

Date: 2026-08-12

## Context

ADR 0011から0014までで、M.O.E.開発室の読み取り、ユーザーメッセージ、Codex応答、再起動後の履歴復元はRust Room sourceへ統合された。一方、回答くらべ部屋とMCP実験室、新規ルーム作成、参加AI追加はReactのローカルstateだけに残っていた。このままでは同じTauri画面内に永続Roomと一時的なデモRoomが混在し、再起動でユーザー操作が失われる。

永続ファイルはすでに `fileVersion: 1` のfull snapshotである。ここで別DBや別catalogファイルを追加すると、単一Desktop writerという現行境界に対して同期点と障害経路を増やしてしまう。また既存ユーザーのsingle-room snapshotを単純にbundled snapshotへ置き換えると、履歴を失う。

## Decision

- Coreにmessage本文を含まないboundedな `RoomSummary` と `RoomCatalogSource::list_rooms` を追加する。Tauri UIはcatalogを取得した後、各Roomを既存の最大30件 `desktop_room_read` で読む。WebViewへunbounded snapshotやfilesystem pathを公開しない。
- `RoomStore` にidempotentな `create_room` と `add_room_participant` を追加する。同じRoom ID・名前・初期参加者の再試行、すでに参加済みAIの再試行は `duplicate` とし、異なる内容で同じRoom IDを使う要求は拒否する。
- 公開するTauri mutationは `desktop_room_create(roomId, name)` と `desktop_room_add_participant(roomId, participantId)` に限定する。作成時の初期参加者はRust側で `owner` と `codex` に固定し、時刻もRustが付与する。WebViewから任意の参加者定義やmessage履歴を注入させない。
- bundled snapshotは、画面catalogに存在する10参加者と3室（M.O.E.開発室、回答くらべ部屋、MCP実験室）を正本として持つ。既存のversion 1 snapshotを読むときは、欠けているbundled participant / Room IDだけを追加して同じversion 1へ保存する。既存IDの内容と既存messageは上書きしない。
- list / read / find / append / create / participant addは同じDesktop transaction mutexを通す。mutation後のtemp + backup永続化に失敗した場合は直前snapshotへmemoryを戻し、成功を返さない。
- Tauri版では全Roomの送信をRust write / dispatchへ通す。Codexだけが現在の実AI dispatch対象であり、未接続Providerには偽の返信を作らず、ユーザーメッセージを保存したうえで `unsupported` を表示する。ブラウザpreviewは従来どおりローカル3室デモとする。

## Consequences

- 3つの標準Room、追加Room、参加AI、全Roomのユーザーメッセージが同じJSON snapshotから再起動後に復元される。
- 旧single-room fileは初回起動時に不足catalogだけが補われ、既存履歴を保持する。schemaの意味を変更しないためfile versionは1のままとする。
- Core、Tauri command、永続化についてcatalog、冪等作成、参加者追加、競合、未知参加者、rollback、旧file補完の自動テストを持つ。
- 隔離したTauri debug実画面で、3室表示、回答くらべ部屋へのGemini宛て保存、新しいルーム4の作成、Gemini追加、process終了、同じfileでの再起動、本文・Room・AI人数の復元を確認した。Geminiは未接続のため外部送信は発生していない。既存release processは停止していない。
- Room名編集、Room削除、参加者削除、Room並べ替え、履歴export / recovery UI、複数process writer、永続dispatch ledger、incremental databaseは後続の別判断とする。
