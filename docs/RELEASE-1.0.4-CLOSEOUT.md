# M.I.O. 1.0.4 公開後確認の整理

更新: 2026-09-08 16:35 JST。対象はStore `1.0.4.0`、直接配布 `1.0.4`、GitHub `v1.0.4`。

今回合意した公開・配布・実機確認の作業は完了した。下記の省略と次期対応は完了と区別する。
包括的な旧V1チェックリストは全項目PASSに変更しない。この文書を現在の作業一覧とし、
過去の日付に書かれた「未確認」「残り」を再試験の指示として扱わない。

## 完了

| 項目 | 確認した結果 |
| --- | --- |
| Store公開・実機更新 | `1.0.4.0`、Store署名、Status Ok、起動・通常終了・再起動を確認 |
| TINMOON日英・BOOTH・itch.io | 1.0.4の案内と更新履歴を公開。既存1.0.3履歴を保持 |
| GitHub Release | `v1.0.4`を通常Releaseとして公開。匿名取得した3添付物のhash一致を確認 |
| BOOTH／itch.ioの配布内容 | BOOTH ZIP、itch.io受信bytes、ZIP内installerが公開原本と一致 |
| クリーンなWindowsへの導入 | 新規Windows 11 x64 SandboxでNSIS版1.0.4のinstall・起動・通常終了・再起動を確認 |
| WebView2未導入環境 | 未導入を事前確認し、公式bootstrapperの自動取得・有効なMicrosoft署名・Runtime導入を確認 |
| Store版Codex実返信 | 専用Roomから1回だけ送信し、`MIO_STORE_104_OK`を受信・保存・再表示 |
| 既存設定と会話 | `gpt-5.6-sol`／会話のみ／workspace未設定を保持。再起動前後の設定・会話fileのhash一致 |
| 履歴の保全 | 通常版とStore版の保存領域の違いを確認。backup保持、通常側snapshot不変 |
| 試験環境の終了 | Owner依頼で専用Sandboxを終了。16:04 JSTの`wsb list`は0件。hostのM.I.O.は終了対象外 |

詳細な日時・hash・試験の制限は[公開後チェックリスト](MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md)に残す。
画面にあった`UI DEMO`の初期サンプルを実Provider返信の証拠に流用しない。

## 省略・今回の範囲外

| 項目 | 方針・制限 |
| --- | --- |
| Sandbox内のAI試験 | Owner指定によりProvider CLI導入・認証・AI送信を省略。再設定を要求しない |
| 通常版からStore版への履歴移行 | Ownerは現在のStore側を継続使用。復元・移行をしない |
| 未設定Provider／workspace権限の全組合せ | 今回の設定保持試験は既存Codex設定の範囲。他Providerやread／write動作までPASSとしない |
| itch.ioのEdge保存警告解除 | 受信bytes一致と警告文言は確認済み。通常名での保存完了は未確認。警告解除・一時fileのrename／実行を追加しない |

## 次期対応・継続確認

| 項目 | 次の扱い |
| --- | --- |
| 最後に選択したRoomの自動復帰 | 次期版source `2606469`に実装済み。14ケースの回帰確認と専用native appでの既存／新規Roomの実機再起動がPASS。[結果と制限](NEXT-VERSION-NOTES.md)を記録。公開1.0.4は変更せず、正式な次期版の作成・公開は別作業 |
| GitHubの警告表示 | 下記の公開lockfileと依存関係一覧の不一致を残す。警告が解消したとは報告しない |
| `glib`の既知警告 | Windows targetには不在。上流Tauri／GTK系の対応を確認する際に再評価し、独立した強制更新やdismissはしない |
| 自前Authenticode署名 | 現在の直接配布は未署名Preview。Store署名と区別し、署名方式変更は別の方針決定とする |
| 最終候補のclean runner CI | 既存のActions利用制限方針により未実施。localの349 passed／0 failed／24 ignoredをCI成功とは扱わない。今回CIは起動しない |
| Store署名済み実体の追加scan | 9月7日にNSISと提出前MSIXのDefender scanは完了。旧V1基準にある最終署名実体の追加scanとは区別する |

## GitHub警告の再確認（2026-09-08 16:04 JST）

| 対象 | API上の状態 | 公開sourceとの照合 |
| --- | --- | --- |
| public `M.I.O` | open 6件: fast-uri High 4、qs Moderate 1、glib Moderate 1 | `main`のlockfileは修正版fast-uri `3.1.7`／qs `6.16.0` |
| private `M.I.O-dev` | open 1件: glib Moderate | npm側の既存警告はfixed。glibは引き続きopen |
| public依存関係一覧（SBOM） | fast-uri `3.1.5`、qs `6.15.3`、glib `0.18.5` | 実際のpublic `main`とnpm 2件のversionが不一致 |

公開`package-lock.json`のGit blobは`3701c2b0c5336a4b1257bdb9b59e1b300efd44cc`でlocalと一致した。
全fast-uri／qsエントリーを確認し、それぞれ修正版1件だけだった。
公開`Cargo.lock`も`39f9b0639999f11c798e3bf831d55be9d736d729`でlocalと一致した。
`cargo tree --locked --target x86_64-pc-windows-msvc -i glib`はexit 0、`nothing to print`だった。

この証拠から、npm 5警告についてGitHubの依存関係情報が公開sourceの変更を反映していないと考えられる。
原因や再解析の完了時期は未確定。glibはこの表示の不一致とは別の既知課題である。
alertの手動dismiss、Dependabot PRのmerge／close、依存更新、公開tag／assetの差替えは行っていない。

確認先: [公開リポジトリの警告](https://github.com/blackcometclub/M.I.O/security/dependabot)、
[開発リポジトリのglib警告](https://github.com/blackcometclub/M.I.O-dev/security/dependabot/1)、
[公開lockfile](https://github.com/blackcometclub/M.I.O/blob/main/package-lock.json)。
local証拠: `.tools/public-release-prep/1.0.4/closeout-20260908-1604/`の
`lock-verification.json`、`public-open-alerts.json`、`private-open-alerts.json`、`public-dependency-graph.json`。
