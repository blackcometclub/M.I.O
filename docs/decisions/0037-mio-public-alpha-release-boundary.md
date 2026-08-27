# ADR 0037: M.I.O. public alpha release boundary

- Status: Accepted
- Date: 2026-08-17
- Supersedes: ADR 0001のユーザー向け製品名
- Depends on: ADR 0025 (single-instance desktop ownership), ADR 0027
  (durable AI dispatch ledger), ADR 0034 (device-local AI access
  permissions), ADR 0035 (Room-scoped conductor orchestration), ADR 0036
  (local Codex owner-proxy MCP)

## Context

M.I.O.は、複数のAI participantを同じTalk Roomへ集め、Ownerが直接宛先を
選ぶDirect modeと、1人のAIへ限定的な作業分担を任せるConductor modeを持つ
Windows desktop applicationとして動作している。

一方、Claude Webの正式接続、公開Remote Relay、任意Provider接続、token
streaming、全AIへのworkspace accessなど、将来機能も同じrepositoryで研究中で
ある。すべての将来機能が完成するまで公開を延期すると、現在すでに動く安全な核を
評価してもらえない。逆に、研究中のsurfaceを完成機能のように見せると、利用者が
外部送信、権限、可用性を誤認する。

最初の公開では、完成済みと説明する範囲、実験中または未対応として残す範囲、
および公開を許可する前の合格条件を明示する必要がある。

## Decision

### Release identity and positioning

最初の公開候補を次のように定義する。

> **M.I.O. v0.1.0-alpha.1**
> 複数のローカルAIをひとつのTalk Roomに集め、直接会話または指揮者モードで
> 協働させるWindows向け研究用α版

ユーザー向け製品名は **M.I.O.**、正式展開名は
**Malevolent Immortal Overdrive** とする。日本語UIでも正式展開名を翻訳しない。

ここでいう「ローカルAI」は、M.I.O.が利用者のWindows上で検出・起動するCLI
adapterを指し、model推論が端末内だけで行われることを意味しない。Providerへ送った
会話は、各Providerのnetwork、契約、課金、保存方針の対象になり得る。この区別を
READMEとUIの接続説明で明示する。

既存Room ID、app data path、Rust crate、package、environment variableなどの
`moe`内部識別子は、保存データと開発境界の互換性を守るため、この公開名変更だけを
理由にrenameしない。利用者に見える製品名、説明、title、icon、公開文書はM.I.O.へ
段階的に統一する。

### Included alpha scope

次の機能を、対応する公開前試験に合格した場合だけalpha.1の完成機能として説明する。

- Windows上で起動するTauri / React Talk Room UI。
- Room作成、名前変更、参加AI管理、永続message履歴、backup、restore。
- 参加AIの表示名、avatar、基本設定、対応済みaccess modeのdevice-local設定。
- Codex、Gemini Antigravity、Claude Fable、Grokの、利用可能なローカルCLIを介した
  boundedな会話配送。
- Direct modeによる明示的な宛先選択。
- Codexを最初の対応conductorとする、1 round・最大3 workerのConductor mode。
- Codexのchat-only境界、およびworkspace read / writeの送信前fail-closed。
- single-instance、recipient単位のdurable dispatch ledger、unknown outcomeの非自動再送。
- token設定時だけloopbackへ起動する、boundedなM.I.O. local MCP tools。

特定CLIの未導入、未認証、Provider側の利用不能は、M.I.O.全体の起動失敗へ変換しない。
利用不能なparticipantは状態を明示し、偽のAI replyや成功表示を生成しない。

### Explicitly excluded or experimental scope

次はalpha.1の完成機能として宣伝しない。

- Claude Webの正式製品接続。
- 公開Remote Relay、複数device、複数account、credential rotation UI。
- ChatGPT Web、OpenAI API、Generic MCP client、任意Custom adapter。
- Google Search / AI Mode Browser Bridgeの正式製品化。
- Fable、Gemini、Grokのworkspace read / write。
- Codexのworkspace read / write。Windows native sandboxのnested junction経由readが
  選択root境界を満たすまで、UIでdisabledとしProvider起動前に拒否する。
- token streaming UI、provider turnの途中cancel、model選択UI。
- background automation、無制限のconductor round、nested delegation。
- Windows以外の対応保証。
- 署名済みinstaller、自動更新、安定版SLA。

これらをUIへ残す場合は、`実験中`、`未接続`、`現在未対応`、または同等に明確な
状態を表示する。押せるcontrolがある場合は、安全に説明または拒否し、無反応にしない。

### Four functional release gates

公開候補commitは、次の4条件すべてに実測証拠を持たなければならない。

1. 未完成のbuttonやcontrolを操作しても、appが壊れず、無反応のままにならない。
2. 実験中、未接続、現在未対応の機能が、利用者から見て区別できる。
3. AI CLIが一つも存在しない隔離Windowsでも、M.I.O.自体が正常起動する。
4. 完成済みと公開文書で説明する機能だけを、Provider状態とM.I.O. runtimeを隔離した
   controlled Windows環境で一巡試験する。

第3条件は、networkとclipboardを無効化し、検査対象build folderだけをread-onlyで
共有したWindows Sandboxを第一候補とする。PATHだけを空にする試験は、各adapterが
`APPDATA`、`LOCALAPPDATA`、`USERPROFILE`以下の標準install pathも検出するため、
正式な合格証拠にはしない。

第4条件では、利用可能と説明するProvider adapterについて、文書化した導入元、version、
署名またはhashを確認する。Providerの認証済み実CLIを使う機能試験は通常Windows上でもよいが、
Room、dispatch ledger、continuity、Conductor、orchestration、workspace、backupをすべて
test専用pathへ向け、旧app dataを読み込まない。非機密のtest-only Roomを使い、実credentialを
変更・記録しない。CLIなし起動を確認する第3条件は引き続き隔離Windowsで行う。

### Source-first publication boundary

alpha.1の最初の公開形態はsource-firstとする。配布用installer、code signing、
automatic updaterが完成しているとは表現しない。検証用binaryはclean-environment
試験へ使用できるが、それだけで一般配布artifactの完成とは扱わない。

既存のprivate development historyをそのままpublicへ切り替えない。公開候補は、
秘密情報、個人情報、local path、session / turn / tunnel metadata、非公開のreview・
handoff文書を除外した現在snapshotから準備する。公開repositoryの作成、visibility
変更、tag、release uploadは、それぞれ実行前にOwnerの明示承認を得る。

### Public screenshots

公開用スクリーンショットは、M.I.O.のユーザー向け表記とalpha UIが確定してから
作成する。公開専用Roomと架空の表示名を使い、全AIへそのturnだけの英語回答を明示的に
依頼する。既存会話、個人名、local path、credential、内部error detailを写さない。

## Release evidence

実行用チェックリストは `docs/PUBLIC-ALPHA1-READINESS.md` を正本とする。checkboxは
証拠を記録して初めて完了にできる。自動testのPASSだけで実画面、controlled Windows、
Provider live path、public repository sanitizationを完了扱いにしない。

## Consequences

- 将来機能を削除せず、現在動く核を正直なresearch alphaとして公開準備できる。
- 未完成surfaceには明示状態と安全な拒否が必要になり、無反応controlを残せない。
- Public brandingとinternal compatibility identifierが当面異なるため、文書と実装で
  その境界を維持する必要がある。
- CLIなし起動とProviderありの完成経路を別々の隔離条件でtestする費用が増える。
- Source公開とinstaller配布を分離することで、署名・更新・installer UXを未完成のまま
  完成品として出すことを避けられる。
- Public snapshotの準備ではprivate development historyを保存したまま、公開対象だけを
  最小化できる。
