# Third-party notices

- Status: Draft for M.I.O. v0.1.0-alpha.1
- Publication mode: Source-first
- Last reviewed: 2026-08-17

この文書は、M.I.O.のsource-first公開候補に含まれる第三者素材と依存関係の確認状況を記録します。将来のinstallerやbinary配布に必要なnotice一式の完成を宣言するものではありません。

## Pixelify Sans

M.I.O.のロゴ `apps/desktop/src/assets/mio-logo.svg` は、Google Fonts配布のPixelify Sans Boldから文字 `M.I.O.` をoutlineへ変換したSVG pathを含みます。font binary自体はM.I.O.へ同梱していません。

- Font: Pixelify Sans
- Designer: Stefie Justprince / Typecalism Foundryline
- Copyright: Copyright 2021 The Pixelify Sans Project Authors
- License: SIL Open Font License 1.1
- Google Fonts source: https://github.com/google/fonts/tree/main/ofl/pixelifysans
- Upstream project: https://github.com/eifetx/Pixelify-Sans
- Included license text: [Pixelify-Sans-OFL.txt](apps/desktop/src/assets/licenses/Pixelify-Sans-OFL.txt)

ロゴSVGには由来とOFL 1.1を示すhuman-readable commentを保持しています。Pixelify Sansの作者名は由来表示にのみ使用し、M.I.O.の推奨・承認を示すものではありません。

## Project assets

| Asset | Source and distribution boundary | Current status |
|---|---|---|
| `apps/desktop/src/assets/mio-logo.svg` | Pixelify Sans Boldの `M.I.O.` outline。上記OFL 1.1の対象 | Verified; Owner approved 2026-08-21 |
| `apps/desktop/src-tauri/app-icon.svg` | Repository内で作成した幾何学図形のproject icon | Owner approved 2026-08-21 |
| `apps/desktop/src-tauri/icons/**` | `app-icon.svg`から生成したplatform別raster / icon | Owner approved 2026-08-21 |
| `spikes/codex-app-server/fixtures/local-image-vision.png` | 同梱の`generate-image-fixture.ps1`がSegoe UI / Consolasの文字と単純図形だけで生成。runtime artifactには含めない | Owner approved 2026-08-21 |
| `spikes/fixtures/seeded-bug-app/baseline/ui-state.svg` | Repository内で文字、矩形、配色だけから作成したtest fixture。runtime artifactには含めない | Owner approved 2026-08-21 |
| 利用者が設定する背景・飾り絵・avatar | Repositoryや配布artifactへ同梱しないdevice-local data | Not a bundled asset |
| 公開用screenshot | まだ作成・承認していない | Pending |

上記App icon、logo、test fixtureは、技術的な由来確認に加えてOwnerがsource-first公開snapshotへの同梱を承認しました。公開用screenshotは撮影後に別途確認します。

## JavaScript dependencies

`package-lock.json`に記録された外部 `node_modules` entry 180件を2026-08-21に再集計し、次のlicense identifierを確認しました。

| License identifier | Entries |
|---|---:|
| MIT | 116 |
| Apache-2.0 | 22 |
| Apache-2.0 OR MIT | 13 |
| MPL-2.0 | 12 |
| ISC | 8 |
| BSD-3-Clause | 3 |
| BSD-2-Clause | 1 |

license fieldがない5 entryは、すべてこのrepository自身のprivate npm workspaceです。M.I.O.のsource snapshotは `node_modules`をvendorしません。将来binaryやinstallerを配布する前に、実際にbundleされるdependencyだけを対象にlicense textとnoticeを再生成します。

## Rust dependencies

Rust dependencyは `Cargo.lock`でversionを固定し、source snapshotへCargo registry packageをvendorしません。2026-08-21に全targetのcrate sourceを`cargo fetch --locked`で取得し、`cargo metadata --format-version 1 --locked --offline`から外部crate 541件を集計しました。

- license identifierまたはlicense fileのmetadata欠落: 0件
- 主なlicense family: MIT、Apache-2.0、BSD、ISC、Zlib、Unicode-3.0、MPL-2.0、CDLA-Permissive-2.0
- 不明またはlicense無記載の外部crate: 0件

この確認はsource-first snapshotの依存metadataを対象とします。将来binaryやinstallerを配布する場合は、実際にlink・bundleされるtarget別dependency、license text、notice、例外条項を配布artifact単位で再生成します。

## Fonts selected from the device

M.I.O.の通常UIはCSSのsystem font fallbackや、利用者が端末から選択したfont familyを使用できます。それらのfont fileをrepositoryまたは配布artifactへコピーしません。利用者の端末にあるfontの利用条件は、そのfontとOSのライセンスに従います。
