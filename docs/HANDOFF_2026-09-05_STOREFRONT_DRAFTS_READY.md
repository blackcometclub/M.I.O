# M.I.O. 引継ぎ — Storefront非公開Draft準備後

作成日: 2026-09-05 JST

作業場所: `D:\desktop\M.O.E`

branch: `main`

## 再開時に最初に確認すること

1. ルートの`AGENTS.md`、本書、`docs/V1-READINESS.md`を読む。
2. `git status --short --branch`、`git log -5 --oneline`、
   `git rev-list --left-right --count HEAD...origin/main`を実行し、live stateを優先する。
3. Partner CenterのM.I.O.概要を読み取り専用で確認する。
4. 認定の取消、再提出、package差替え、公開設定変更、`今すぐ公開`、Storefront公開、
   Git commit／pushは、それぞれOwnerの明示依頼または承認を待つ。
5. 未追跡のFable文書、過去の引継ぎ、試験画像を変更・移動・削除・stageしない。
   `git add .`、`git add -A`、reset、clean、force pushを使わない。

## Gitの基準

本書作成前の確認では次の状態だった。再開時は必ず再確認する。

- `HEAD`: `1b92dbc BOOTH非公開下書きの作成結果を記録する`
- `main...origin/main`: ahead `0` / behind `0`
- `origin`: `https://github.com/blackcometclub/M.I.O-dev.git`
- 本書と`docs/V1-READINESS.md`の更新は未commitである。

## Microsoft Storeの現在の境界

- 製品名: `M.I.O`
- Store ID: `9NS9B7T71XHN`
- Submission ID: `1152921505701802472`
- 提出package version: `1.0.1.0`
- 提出実体:
  `D:\desktop\M.O.E\.tools\public-release-prep\1.0.0\8e3eb4e22c67-store-20260904-104728\artifacts\M.I.O_1.0.1.0_x64_store-unsigned_20260904-120025.msix`
- 正式V1候補source commit:
  `8e3eb4e22c67a52007ca1890cb912b477314368a`
- package size: `6,479,052 bytes`
- package SHA-256:
  `279D7DFFBF36C06E7FF698C599C91530F567DEB8FFF605AA5E1AEEEA6BEAFDD5`
- 旧`1.0.0.0`提出はOwner承認のもとで取り消し・削除済み。
- 2026-09-05 12:51 JSTのPartner Centerでは、上部に`認定プロセスに合格しました`、
  `公開準備が完了しました`、`今すぐ公開`が表示された。
- 同じ画面の下部には`製品の申請: 認定中 (Submission 1: 最終変更日 2026/09/04)`も
  残っており、表示更新の混在がある。
- 認定後すぐに自動公開せず、Ownerが`今すぐ公開`を明示承認するまで公開保留となる設定を適用済み。
- 認定合格と一般公開は別の状態として扱う。
- 読み取り確認だけを行い、`認定の取り消し`、`今すぐ公開`、公開時刻変更は操作していない。

### 公開前local再照合

2026-09-05 13:04 JSTに提出実体を読み取り確認した。

- file sizeとSHA-256は本書の記録および隣接`.sha256`と一致
- manifest: Identity `TINMOON.M.I.O`
- Publisher: `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`
- package version: `1.0.1.0`
- architecture: `x64`
- target: `Windows.Desktop`、minimum `10.0.22000.0`
- resources: `en-us`、`ja-jp`
- main executableのFileVersion／ProductVersion: `1.0.0`
- `LICENSE`、`THIRD-PARTY-NOTICES.txt`、main executable、helper、ロゴを含む
- 正式V1候補commitから現在のHEADまでruntime code変更は0件
- local MSIXはStore提出用のため`NotSigned`。Store署名済み配布物の実機確認はまだ行っていない
- 未login browserで`https://apps.microsoft.com/detail/9NS9B7T71XHN`を開くと
  `製品が見つかりません`となり、一般公開前であることを確認した
- Partner Centerの`Current packages`でSubmission 1を展開し、提出file名、version `1.0.1.0`、x64、
  Windows.Desktop minimum `10.0.22000.0`、`en-us`／`ja-jp`、`runFullTrust`、表示size `6.2 MB`が
  local提出実体と一致することを確認した

Partner Center概要:

`https://partner.microsoft.com/ja-jp/dashboard/products/9NS9B7T71XHN/overview`

## itch.ioの非公開Draft

- URL: `https://tinmoon-label.itch.io/mio-talk-room`
- game edit ID: `4974122`
- 英語版のみの詳細本文
- Kind: Downloadable
- Platform: Windows
- 価格: `$0 or Donate`
- suggested donation: `$2.00`
- coverと5 screenshotを登録済み
- 実行file／正式Store linkは未登録
- Draft／Restrictedのままで、公開していない

## BOOTHの非公開Draft

- item ID: `8807279`
- 編集画面: `https://manage.booth.pm/items/8807279/edit`
- 商品名: `M.I.O. v1.0.0｜複数AIトークルーム（Windows 11）`
- ダウンロード商品／ソフトウェア／全年齢
- 価格: 0円
- BOOSTは任意支援であり、支援しなくても無料で利用できる旨を記載
- 代理購入: 許可しない
- 日本語の詳細本文
- coverと5 screenshotを登録済み
- 実行file／正式Store linkは未登録
- 下書きのままで、公開していない

## 関連する現在の文書

- `docs/storefronts/BOOTH-LISTING-DRAFT.ja.md`
- `docs/storefronts/ITCHIO-LISTING-DRAFT.md`
- `docs/storefronts/BOOTH-ITCHIO-PUBLISHING-CHECKLIST.md`
- `docs/MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md`
- `docs/V1-READINESS.md`

## 次に行う順序

1. Partner Centerを再確認し、`公開準備が完了しました`と下部の`認定中`の混在表示が
   解消したか、公開処理が未開始のままかを読み取る。
2. 認定失敗なら、表示された理由をそのまま記録して停止する。推測で修正、再upload、再提出をしない。
3. 認定合格・公開保留なら、local package照合結果とStore登録情報、V1表記を見比べる。
   local packageの再照合は完了した。Ownerの明示承認なしに`今すぐ公開`を選ばない。
4. BOOTH／itch.ioをStore誘導ページにするか、直接downloadも用意するかをOwnerが決める。
5. 選んだ方式に合わせ、正式linkまたは配布file、size、SHA-256、署名説明を両Draftへ入れる。
6. Store、BOOTH、itch.io、GitHub Release、レーベルサイトは別々の公開境界とし、
   それぞれ直前に対象を示してOwnerの承認を得る。

## 現在触れないもの

- 認定済み・公開保留中のStore提出packageと公開設定
- BOOTH／itch.ioの公開状態
- 未追跡のFable文書、過去の引継ぎ、試験画像
- 一般配布用installer、GitHub tag／Release、レーベルサイトの公開内容
