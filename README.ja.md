# M.I.O.

[English](README.md) | **日本語**

**M.I.O. (Malevolent Immortal Overdrive)** は、複数のAIをひとつのTalk Roomへ集め、直接会話または指揮者モードで協働させるWindows向けローカル優先デスクトップアプリです。

> [!IMPORTANT]
> **M.I.O. v1.0.5は一般公開済みです。** Microsoft Storeでは署名済み`1.0.5.0`を配布中です
> （2026年9月9日に公開・実機更新済みと確認）。BOOTH／itch.ioでは同日に公開した、別配布の未署名
> `1.0.5` Previewを入手できます。現在のsource versionも`1.0.5`です。
> 公開アプリのbuild元は`3076294e1d87821afd1e18f36420d9162820679e`です。
> その後の公開案内の文書更新によって、アプリ本体のコードは変わっていません。

- [Microsoft Store](https://apps.microsoft.com/detail/9NS9B7T71XHN) — 推奨する署名済みpackage
- [BOOTH](https://tinmoon.booth.pm/items/8807279) — 無料・支援任意の未署名Preview ZIP
- [itch.io](https://tinmoon-label.itch.io/mio-talk-room) — 無料／支援任意の未署名Preview installer
- [v1.0.5の更新履歴](https://tinmoon-label-site.pages.dev/works/mio/#release-notes) — 日本語の変更内容

公開済みの[alpha.2 Release](https://github.com/blackcometclub/M.I.O/releases/tag/v0.1.0-alpha.2)と
それ以前のreleaseは、履歴として変更せず固定しています。旧tagやassetをV1用に差し替えず、
現在のV1は上記の配布経路から取得します。

## Screenshots

![M.I.O.の英語Talk Room。Codex、Claude Web、Geminiが参加する撮影専用UIデモ](docs/assets/screenshots/mio-talk-room.png)

この画面は公開撮影専用のRoomと架空の会話で構成したUIデモです。表示されたAI responseは
機能説明用の架空の文面であり、実際のProvider応答を示すものではありません。

<details>
<summary>Preferences、appearance、Room settingsを見る</summary>

### Preferences

![M.I.O.の英語Preferences画面](docs/assets/screenshots/mio-preferences.png)

### Appearance

![M.I.O.の英語appearance設定画面](docs/assets/screenshots/mio-appearance.png)

### Room settings

![M.I.O.の英語Room settings画面](docs/assets/screenshots/mio-room-settings.png)

</details>

## できること

- 複数のAI participantと人間が同じTalk Roomで会話
- 宛先を明示して送る **Direct mode**
- Codexが回答または最大3人のworkerへ1 roundだけ分担する **Conductor mode**
- Roomの作成、名前変更、参加AI管理、message履歴の永続化
- 参加者の表示名、avatar、AI向けローカル案内、対応済みaccess modeの端末内保存
- 利用者が選んだ保存先への全Room安全JSON backupと、内容確認・承認後の復元
- Codexへ会話のみ、選択folderの読取、選択folderの読取・編集を個別に設定
- Codexへ固定された少数の開発操作を許可し、package導入などの重要操作はOwner確認を要求
- CodexとClaude Codeの確認済みmodelをparticipantごとに選択
- 送信buttonを使って実行中のProvider turnを停止
- Codex生成画像の表示、download、file名確認後の編集folder保存
- token設定時だけloopbackへ起動する、範囲を限定したlocal MCP tools

Conductor modeは自動で無制限に仕事を連鎖させる機能ではありません。最初の対応conductorはCodexだけで、Ownerの1件の依頼に対して1 round・最大3 workerまでに制限しています。Direct modeとConductor modeはRoom画面で選び直せます。

## 対応状況

| 接続先 | V1での状態 | 範囲 |
|---|---|---|
| Codex | 対応 | ローカルCodex CLI経由の会話、Conductor、確認済みmodel選択、画像生成、選択folderの読取・編集、固定command |
| Gemini Antigravity | 対応 | ローカルCLI経由の会話専用応答 |
| Claude Code（表示名は変更可能） | 対応 | ローカルClaude CLI経由の会話専用応答と確認済みmodel選択 |
| Grok | 対応 | ローカルCLI経由の会話と、Ownerが選択した場合だけ追跡済みGit差分の読取専用レビュー |
| Claude Web | 未接続 | Remote MCP / Relayは研究中で、正式な製品接続ではない |
| Google Search / AI Mode Browser Bridge | 実験中 | お遊び用PoC。通常版では無効 |
| ChatGPT Web / OpenAI API | 現在未対応 | UIでは選択不可 |
| Generic MCP client / Custom adapter | 現在未対応 | UIでは選択不可 |

「ローカルCLI」は、M.I.O.が利用者のWindows上で検出・起動するadapterを指します。モデル推論が端末内だけで行われるという意味ではありません。AIへ送信した会話は、各Providerのnetwork、利用規約、契約、課金、保存方針の対象になり得ます。各CLIの導入、認証、利用条件は利用者自身で確認してください。

CLIが見つからない、認証されていない、またはProviderが利用不能でも、M.I.O.自体は起動を継続する設計です。利用不能なparticipantは状態を表示し、接続できていないAIの偽replyを生成しません。

## 安全のための境界

- 1つのsource message / recipientにつき外部turnを一度だけ開始し、結果不明時に自動再送しない
- Roomとdispatchの状態を永続化し、partial resultとunknown outcomeを成功扱いしない
- desktopはsingle-instanceで動作し、同じRoom dataへのwriter重複を避ける
- Codexのworkspace accessを選択した1 folderへ限定し、M.I.O.のpath・file operation brokerを経由させる
- commandは固定toolだけを許可し、指定操作は実行時確認を要求する。任意command文字列、削除、名前変更、無制限network accessは許可しない
- FableとGeminiにはworkspace read / writeを付与しない。GrokにはOwnerが参加者プロフィールで選択した場合だけ、M.I.O.が作成する追跡済みGit差分とstatusの読取専用レビューを許可する
- local MCPはtoken未設定時に起動せず、loopback以外へbindしない
- WebViewへ任意shell権限やcredential値を公開しない

現在の公開境界と検証証拠は [V1準備チェックリスト](docs/V1-READINESS.md) を参照してください。
歴史的なalpha境界は [ADR 0037](docs/decisions/0037-mio-public-alpha-release-boundary.md) に残しています。

## 現在の主な制約

- Windows以外の動作を保証しない
- FableとGeminiは会話専用。Grokのworkspace accessは、追跡済みGit差分とstatusの読取専用レビューに限定する
- Codex workspace toolはUTF-8 text fileと範囲を限定した開発操作用で、desktopやshellを自由に操作する機能ではない
- 画像生成は正確なpixel数を指定できない。アスペクト比と品質は希望値で、M.I.O.が受け取れる画像は16 MiBまで
- 実行中のProvider turnは停止できるが、token単位のstreaming表示には未対応
- 公開Remote Relay、複数device、複数accountは研究中
- background automation、無制限のconductor round、nested delegationは未対応
- 自動updateは未対応。Microsoft Store版はStore経由で署名され、BOOTH／itch.ioの直接配布版は未署名Previewとして明示する

## 動作環境

M.I.O. V1が対応するのは64-bit版Windows 11です。Windows 10はalpha開発中に使用しましたが、最終release
candidateを検証していないためV1では保証しません。実行には
[Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
のEvergreen版が必要です。Microsoft Edge browserそのものではなく、Windows desktop
appが画面を表示するための共有Runtimeです。

install、初回起動、手動update、backup、data送信、uninstall、troubleshootingは
[V1利用ガイド](docs/USER-GUIDE.ja.md)を参照してください。

source checkoutからTauriのWebView2 download bootstrapperを使う未署名の検証用installerを
buildできます。このlocal artifactは、Microsoft Store署名済みpackageや公式storefrontのdownloadではありません。
standalone EXEだけを実行する場合にRuntimeがなければ、Microsoft公式download
pageからEvergreen Runtimeを導入してください。

Codex、Gemini、Claude、GrokのCLIはM.I.O.本体の起動要件ではありません。利用したい
AIのCLIだけを別途導入・認証します。CLIが一つもない隔離Windows環境での公開前Gate 3
起動確認は完了しています。検証内容は
[公開準備チェックリスト](docs/PUBLIC-ALPHA1-READINESS.md)へ記録しています。

### Provider CLIの導入と認証

M.I.O.はCLIの導入、更新、login、credential入力を代行しません。利用するProviderだけを
M.I.O.を終了した状態で導入し、各CLIを単独起動して公式のlogin手順を完了してください。
install scriptを直接実行できない組織では、リンク先の公式手順と組織のsoftware導入policyを
優先してください。

#### Codex

[OpenAI公式のCodex CLI手順](https://learn.chatgpt.com/docs/codex/cli)に従い、
WindowsではPowerShell 7からstandalone installerを実行します。Windows PowerShell 5.1では、
2026-08-23の実機確認時点のinstallerが`OSArchitecture`を取得できず停止したため使用しません。

```powershell
winget install --id Microsoft.PowerShell --source winget
pwsh -NoProfile -Command "irm https://chatgpt.com/codex/install.ps1 | iex"
codex --version
codex
```

初回起動では`Sign in with ChatGPT`または公式手順にあるAPI key方式を選びます。M.I.O.は
`codex`をPATHまたは標準install先から検出し、`codex app-server`を起動します。

Codex participantごとに、会話のみ、選択folderの読取、選択folderの読取・編集を選びます。read / writeは
M.I.O.が仲介し、AIへhost上のworkspace絶対pathを公開しません。write modeではtext fileの新規作成と
安全な置換に加え、固定されたNode／npm／Git status操作を利用できます。必要な操作はOwner確認を要求します。
M.I.O.はCodexの`config.toml`を変更しません。

#### Gemini Antigravity

[Google公式のAntigravity CLI手順](https://codelabs.developers.google.com/antigravity-cli-hands-on)
に従い、Windows PowerShellで導入します。

```powershell
irm https://antigravity.google/cli/install.ps1 | iex
agy --version
agy
```

初回起動でGoogleのloginを完了します。M.I.O.は`agy`をPATHまたは
`%LOCALAPPDATA%\agy\bin\agy.exe`から検出し、会話専用の非interactive modeで使用します。

#### Claude Fable

[Anthropic公式のClaude Code手順](https://code.claude.com/docs/en/installation)に従い、
Windows PowerShellで導入します。

```powershell
winget install Anthropic.ClaudeCode
claude --version
claude
```

初回起動でbrowser loginを完了します。M.I.O.は`claude`をPATHまたは
`%USERPROFILE%\.local\bin\claude.exe`から検出し、toolを無効化した会話専用modeで使用します。
profileではProvider既定またはM.I.O.が表示する確認済みClaude Code modelを選べます。契約上利用できない
modelのreplyは成功扱いにしません。

#### Grok

[xAI公式のGrok CLI手順](https://docs.x.ai/build/overview)に従い、Windows PowerShellで
導入します。

```powershell
irm https://x.ai/cli/install.ps1 | iex
grok --version
grok
```

初回起動でbrowser loginを完了します。M.I.O.は`grok`をPATHまたは
`%USERPROFILE%\.grok\bin\grok.exe`から検出し、web search、memory、subagent、toolを
無効化した1 turnの会話modeで`grok-4.6`を要求します。参加者プロフィールで読取専用を選んだ場合は、
M.I.O.のbrokerが作成したGit statusと追跡済み差分だけをレビュー用に渡します。未追跡file本文、
workspaceの絶対path、編集、command、web accessは渡しません。契約上そのmodelを利用できない場合は
成功扱いにしません。

導入とlogin後は、新しいPATHとcredential状態を読み込ませるためM.I.O.を再起動します。
M.I.O.へpassword、OAuth code、API keyを貼り付けないでください。最初のlive replyを確認する
までは、CLIを検出しても接続済みとは表示しません。送信内容は各Providerのnetwork、契約、
課金、保存方針の対象になるため、live試験時にはCLI version、認証方式、利用plan、retentionを
credential値なしで[公開準備チェックリスト](docs/PUBLIC-ALPHA1-READINESS.md)へ記録します。

## 開発環境

現在の対象はWindowsです。sourceからの開発・検証には次が必要です。

- Node.js 24以上
- npm 11.11.x
- Rust 1.96.x（`rust-toolchain.toml`で固定）
- Microsoft C++ Build Toolsの「Desktop development with C++」workload
- Microsoft Edge WebView2

依存関係をlockfileどおりに導入し、Tauri開発アプリを起動します。

```powershell
npm.cmd ci
npm.cmd run dev
```

開発用binaryをbuildする場合は次を使います。

```powershell
npm.cmd run tauri:build
```

`tauri:build` は開発確認用の `--no-bundle` buildです。インストーラーは生成しません。

Windows x64向けのrelease検証用EXEは、次の専用scriptでbuildします。script名は互換性のため
従来のままです。このscriptはfrontendをbuildした後、Visual C++ Runtimeを静的linkしたrelease executableを生成し、SHA-256を
表示します。installerやWebView2 Runtimeは同梱しません。

```powershell
& .\scripts\build-alpha-windows.ps1
```

生成先は `target/x86_64-pc-windows-msvc/release/moe-desktop.exe` です。

Windows x64向けの未署名NSIS検証用installerは、次のcommandでbuildします。

```powershell
& .\scripts\build-alpha-windows.ps1 -Installer
```

このinstallerは全ユーザー用として`%ProgramFiles%\M.I.O`へ導入し、WindowsのUACで管理者の承認を求めます。
これにより、通常の保守とuninstallに使うWindows設定と「プログラムと機能」へM.I.O.が表示されます。
WebView2がない場合はTauriのdownload bootstrapperを使用するため、install中にinternet接続が必要です。
生成先は `target/x86_64-pc-windows-msvc/release/bundle/nsis/` です。これはローカル検証用artifactで、
未署名のままGitHub Releaseへ自動添付されることはありません。固定requestだけを受ける
`moe-command-helper.exe`を同梱し、eligibleなCodex workspace-write profileだけがconfirmation規則に従って
command toolを利用できます。installerはproject licenseを表示し、`THIRD-PARTY-NOTICES.txt`をinstall先へ配置します。

Commit済みの状態からV1 source snapshot、installer、checksum、secret scan結果、release planを作る場合は、
次を使います。root package、desktop package、Tauri、Rust workspaceのversionが一致しない場合は生成を拒否します。

```powershell
& .\scripts\prepare-public-release.ps1 -Commit HEAD
```

未commitまたはstage済みのtracked差分がある場合も、安全のため生成を拒否します。

## 検証コマンド

```powershell
npm.cmd run typecheck
npm.cmd run build
cargo fmt --all -- --check
cargo test --workspace
```

Providerのログイン、外部公開、OS credentialへの書き込みが必要な試験は自動CIに含めません。各PoCの実行条件は `spikes/` 以下のREADMEに記載します。

## リポジトリ構成

```text
apps/
  desktop/            M.I.O.のWindows desktop UIとTauri backend
  relay/              将来のRemote Relay用プレースホルダー
crates/
  moe-core/           Provider非依存のRust core
  moe-protocol/       中立なRustデータ契約
  moe-adapter-sdk/    Rust Adapter境界
  moe-credential-store/ OS credential保管境界
packages/             TypeScript境界
adapters/             Provider別実装の予定地
spikes/               製品実装から隔離した短期PoCと匿名化済み証跡
docs/
  architecture/       未確定の設計案と分析
  decisions/          明示的に採用された判断（ADR）
```

内部のcrate、package、environment variable、app dataには、既存データと開発境界の互換性を守るため `moe` 識別子が残っています。これは旧製品名へ戻す意図ではありません。

ADR 0037より前の履歴文書には、判断時点の旧製品名 `M.O.E.` が残る場合があります。現在のユーザー向け製品名は `M.I.O.` です。採用した判断は理由と影響を [ADR](docs/decisions/) に記録します。

## 公開プロジェクトとしての案内

- 不具合報告: [不具合報告フォームを開く](https://github.com/blackcometclub/M.I.O/issues/new?template=bug_report.yml)
- 機能提案: [機能提案フォームを開く](https://github.com/blackcometclub/M.I.O/issues/new?template=feature_request.yml)
- Contribution方針: [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)
- Support方針: [SUPPORT.ja.md](SUPPORT.ja.md)
- 脆弱性の報告: [SECURITY.ja.md](SECURITY.ja.md)
- 現行V1 Release notes: [docs/RELEASE-NOTES-1.0.0.ja.md](docs/RELEASE-NOTES-1.0.0.ja.md)
- 文書の読み方: [docs/README.md](docs/README.md)

## License

Copyright (c) 2026 blackcometclub.

公開コードは、特記がない限り [GNU Affero General Public License v3.0 only](LICENSE)（`AGPL-3.0-only`）で利用できます。AGPLの条件を適用できない非公開製品・サービス向けには、著作権者が別途商用ライセンスを提供できます。詳細は [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md) を参照してください。

Third-party dependencies and assets remain subject to their respective licenses. The software license does not grant trademark rights in the M.I.O. name or branding.

Pixelify Sansを含む第三者素材とdependency licenseの確認状況は [Third-party notices](THIRD-PARTY-NOTICES.md) を参照してください。
