# 0017: Room backup and recovery

Status: Accepted

Date: 2026-08-12

## Context

ADR 0014から0016でRoom catalog、message、Room管理操作は単一Desktop processのversion 1 JSONへ永続化された。書込み中断に備える直前snapshot 1世代もあるが、壊れたprimaryが存在する場合は起動を拒否し、ユーザーが自分で持ち出せるbackupと復元UIはなかった。

個人利用のWindows V1を完成扱いにするには、通常の保存失敗をrollbackするだけでなく、データを別の場所へ明示保存でき、primary破損から安全に戻れ、復元操作を誤って即実行しない境界が必要である。

## Decision

- 正常なprimaryが読めず、同じ場所の1世代backupが正常な場合、壊れたprimaryを `room-snapshot-v1.json.corrupt-*` へ退避してからbackupをprimaryへ再確立する。壊れた証拠を削除・上書きしない。
- Room設定から全Room snapshotを `Documents/M.O.E Backups/moe-room-backup-<20桁UTC epoch milliseconds>.json` へ書き出す。既存のversion 1形式、64 MiB上限、snapshot validation、temp + sync + renameを再利用し、新形式や依存は追加しない。
- 復元対象はM.O.E.が生成した20桁timestamp名の通常fileだけを同じdirectoryから列挙し、最新1件を選ぶ。symlink、異なる名前、壊れたJSON、不正snapshotは対象外または拒否する。
- 復元前に全snapshotをload / validationし、bundled catalogを非破壊補完する。memoryとprimaryの置換はDesktop transaction mutex内で行い、永続化失敗時はmemoryを復元前へrollbackする。
- UIは「最新を復元…」から「本当に復元」へ切り替わる二段階操作とする。復元成功後はRust catalogを再hydrateし、画面だけ古い状態に残さない。
- `MOE_ROOM_BACKUP_DIR` は隔離テスト用の絶対path overrideとし、通常製品ではDocuments配下だけを使う。filesystem pathそのものはWebViewへ返さず、生成file名とRoom数だけを返す。

## Consequences

- 個人利用者は全Roomを一操作で持ち出し、最新backupから戻せる。primary破損時は直前の正常世代へ自動復旧し、破損fileも調査用に残る。
- 自動テストはexport / latest選択 / restore / process再読込 / 不正名拒否 / primary破損退避を確認する。隔離したTauri実画面では3室・10参加者のbackup生成、成功表示、二段階復元表示を確認した。実画面の最終復元はデータ置換を伴うため実行せず、Rust自動テストで検証した。
- 任意file chooserからのimport、backup一覧・削除・retention、暗号化、複数process writer、incremental databaseはV1完成条件に含めず、必要になった時点で別判断とする。
