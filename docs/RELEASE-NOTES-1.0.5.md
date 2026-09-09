# M.I.O. v1.0.5 更新案内

準備日: 2026-09-08、公開日: 2026-09-09。
直接配布用は未署名Preview `1.0.5`、Microsoft Store版は`1.0.5.0`。
Microsoft Store、TINMOON公式サイト、BOOTH、itch.ioで公開済み。

## TINMOON日本語・BOOTH・Microsoft Store日本語

### v1.0.5 更新内容

- アプリを再起動すると、最後に開いていたRoomが自動で開くようになりました。
- バックアップを復元したときも、表示中のRoomが残っていれば、そのRoomの表示を続けます。
- 前回のRoomが削除されている場合は、利用できる先頭のRoomを開きます。

会話データやバックアップの形式は変更していません。

## TINMOON英語・itch.io Devlog・Microsoft Store英語

### M.I.O. v1.0.5 — Resume your last Room

- M.I.O. now reopens your last selected Room when you restart the app.
- After restoring a backup, the current Room stays selected if it is still available.
- If your previous Room no longer exists, M.I.O. opens the first available Room.

Conversation data and the backup format are unchanged.

## 配布先への反映結果

- TINMOON: 日英の更新履歴へ1.0.5を追記済み。1.0.4以前の履歴を保持した。
- BOOTH: 商品名とdownload対象を1.0.5へ更新し、上記日本語の履歴を追加済み。
- itch.io: download対象を1.0.5へ更新し、上記英語を商品説明とDevlogへ追加済み。過去のDevlogを保持した。
- Microsoft Store: `1.0.5.0`のSubmission 5が認定・公開処理を完了し、Storeで入手可能な状態を確認済み。
- GitHub: source-onlyの通常Release [`v1.0.5`](https://github.com/blackcometclub/M.I.O/releases/tag/v1.0.5)を公開済み。
  source ZIP、snapshot manifest、SHA256SUMSの3点を添付し、未署名installerは添付していない。

直接配布の案内には、既存方針に従って「未署名Preview / Unsigned Preview」、
Windowsの警告が出る場合があること、installerのSHA-256とsource commitを併記する。
Store提出用の未署名MSIXを一般download用として扱わない。

## 検証の扱い

Room復帰の実装は14ケースの回帰確認と専用native appでの既存／新規Roomの再起動確認が完了している。
詳しくは[NEXT-VERSION-NOTES.md](NEXT-VERSION-NOTES.md)を参照。
1.0.5のpackage作成結果と公開確認の詳細は`RELEASE-1.0.5-PREPARATION.md`に記録した。
