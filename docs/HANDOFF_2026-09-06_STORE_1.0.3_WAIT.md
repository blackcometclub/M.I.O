# M.I.O. 引継ぎ — 1.0.3直接配布公開済み／Microsoft Store 1.0.3認定中

- Date: 2026-09-06
- Repository: `D:\desktop\M.O.E`
- Branch: `main`
- Latest pushed commit: `2642b65` (`BOOTHとitch.ioの1.0.3公開記録を更新`)
- Remote: `origin/main`
- Ahead / behind: `0 / 0`

## 現在の結論

BOOTHとitch.ioの直接配布版は`v1.0.3`へ更新し、公開ページで反映を確認した。

2026-09-07のPartner Center実画面で、Microsoft Storeの公開中更新はSubmission 2、package
version `1.0.2.0`であることを確認した。`1.0.3.0`は2026-09-07にlocal生成・検証したが、
Submission 3を作成し、`1.0.3.0`をuploadして保存した。Partner Centerのpackage検査は`Validated`、
section状態は`更新済み`である。Ownerの明示承認後に認定へ提出し、現在は
「Store 1.0.2.0公開済み／Submission 3の1.0.3.0認定中」の段階である。
新しい更新申請を準備しても、現在公開中の`1.0.2.0`は取り下げない。

## 2026-09-06に完了したこと

### BOOTH

- 公開ページ: https://tinmoon.booth.pm/items/8807279
- 商品名を`M.I.O. v1.0.3｜複数AIトークルーム（Windows 11）`へ更新
- 日本語本文のfile名、size、SHA-256、source commit、未署名説明を`1.0.3`へ更新
- 日本語の`v1.0.3（2026-09-06）`更新履歴を追加
- 新しいZIPを無料download対象に設定
- 旧`1.0.0` fileは削除せず、download対象checkを外して保持
- 公開ページで`v1.0.3`、0円、更新履歴、新しいZIPだけがdownload対象であることを確認

公開artifact:

- File: `M.I.O_1.0.3_windows-x64_unsigned-preview.zip`
- Size: 4,504,270 bytes
- SHA-256: `C0F8AF6B135BA1CC9EA5E83586FE7617A1CAAD17F89A97863E5321412DB6FC9F`
- Downloadable ID: `9606110`

### itch.io

- 公開ページ: https://tinmoon-label.itch.io/mio-talk-room
- 英語本文とdownload／install説明を`1.0.3`へ更新
- 英語の`Release notes`／`v1.0.3 — September 6, 2026`を追加
- 新しいEXEを`Executable`／Windowsとして公開
- 旧`1.0.0` fileは削除せず、`Hide this file and prevent it from being downloaded`を有効化
- `$0 or Donate`とsuggested donation `$2.00`は維持
- 公開ページで更新履歴と新しいEXEだけがdownload対象であることを確認

公開artifact:

- File: `M.I.O_1.0.3_windows-x64_unsigned-preview_setup.exe`
- Size: 4,520,172 bytes
- SHA-256: `F9EF0CB56255216B82C92F7D9754FCF638A0B758BADEB9230C5BFD148ED034DE`
- Authenticode: `NotSigned`（publisher／timestampなし）

### 共通のsource境界

- Product version: `1.0.3`
- Artifact source commit: `7ff8b654cfa2a60c97c76e509b940d9993fdb8e6`
- Preparation directory:
  `.tools/public-release-prep/1.0.3/7ff8b65-direct-unsigned-preview-20260906-212359/`
- Microsoft Storeのpackage、Submission、公開設定には今回触れていない

### Microsoft Store 1.0.3.0 local package

- File: `D:\desktop\M.O.E\.tools\store-msix\M.I.O_1.0.3.0_x64_store-unsigned_20260907-080838.msix`
- Size: `6,515,756 bytes`
- SHA-256: `B6B607C5802523E1C4DDCC97285E033DC0FC3EF1D98E2703F429D250C6DF58DD`
- Authenticode: `NotSigned`（Store提出用の想定どおり。sideloadしない）
- Identity: `TINMOON.M.I.O`
- Publisher: `CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7`
- Package version: `1.0.3.0`
- Architecture: `x64`
- Device family: `Windows.Desktop`、minimum `10.0.22000.0`
- Languages: `en-us`、`ja-jp`
- Capability: `runFullTrust`
- Desktop executable ProductVersion／FileVersion: `1.0.3`／`1.0.3`
- Desktop executable SHA-256: `1ADBD965907048F01FD7613DEAC3456186CA6B05000E86173A7F3D34444B480A`
- Command helper SHA-256: `F5B2A63DD8B315FB6C6DE76F34813D96374AE4A195DC9C8AF6D8846319CCBA46`
- `npm.cmd run typecheck`: PASS
- `cargo test --workspace --locked`: PASS（失敗0。実account、network download、実helperなどの明示実行専用試験はignored）
- `pwsh.exe -File scripts\build-alpha-windows.ps1 -Installer`: PASS
- `pwsh.exe -File scripts\package-store-msix.ps1 -PackageVersion 1.0.3.0`: PASS
- Partner Center Submission: `3`（ID `1152921505701823456`）
- Partner Center package check: `Validated`
- Submission section status: `更新済み`
- Certification submission: 実行済み（Ownerの明示承認あり）
- Current certification status: `更新プログラムの認定中`／`前処理中`

### Repository記録

次の5 fileへBOOTH／itch.ioの公開結果と更新履歴を反映し、commit `2642b65`として
`origin/main`へpushした。

- `CHANGELOG.md`
- `docs/V1-READINESS.md`
- `docs/storefronts/BOOTH-ITCHIO-PUBLISHING-CHECKLIST.md`
- `docs/storefronts/BOOTH-LISTING-DRAFT.ja.md`
- `docs/storefronts/ITCHIO-LISTING-DRAFT.md`

`git diff --check`はPASS。push後のahead／behindは`0／0`。

## Microsoft Storeの次回手順

1. Submission 3の認定結果を待つ。
2. 認定失敗ならerror codeと理由をそのまま記録し、推測で再提出しない。
3. 公開完了後、Microsoft Store pageで配信versionを確認する。
4. Storeから更新またはinstallし、起動、終了、再起動、既存Room履歴の保持を実機確認する。
5. Store公開日時と確認結果を`docs/V1-READINESS.md`へ追記する。

現在公開中のStore版と更新申請の扱いは分離する。更新審査中も、既存の公開版を取り下げない。

## Store後に行う候補

- Store版`1.0.3.0`の配信・実機確認結果を記録
- TINMOON公式ページ、BOOTH、itch.io、Microsoft Storeの相互linkとversionを最終確認
- SNS告知とProduct Hunt掲載を行う場合は、投稿直前に本文と公開範囲を確認してOwner承認を得る
- GitHubが通知したModerateのDependabot alert 1件を、公開作業とは別trancheで内容確認する

## 保護する作業tree

repositoryには以前からの未追跡文書、画像、handoff、screenshotが残っている。今回のcommitには
含めていない。次回も`git add .`／`git add -A`を使わず、対象fileだけを指定して扱う。

この引継ぎ文書自体は作成時点では未commitである。commit／pushはOwnerの別の明示依頼を待つ。
