# M.I.O. 公開手順

この文書は、M.I.O.の次回以降のalpha公開を、確認境界を保ったまま簡略化するための手順です。

公開作業は次の4段階に分けます。

1. private側で公開候補をcommit・pushする
2. ローカルで公開素材を一括準備する
3. public repositoryを更新し、GitHub PrereleaseのDraftを作る
4. Draftを確認して公開する

第2段階まではGitHubを変更しません。第3段階と第4段階は、ヒツジさんの明示承認後にだけ実行します。

## 最短の頼み方

通常、ヒツジさんがPowerShellを操作する必要はありません。Codexへ次の順で依頼できます。

1. 「次のalphaを準備して」
2. 表示された版番号、変更内容、検査結果、添付ファイルを確認する
3. 「Draftを作って」
4. GitHubのDraft内容を確認する
5. 「公開して」

Codexは各段階で止まり、次の承認を待ちます。

## 事前条件

- private development repository: `blackcometclub/M.I.O-dev`
- public repository: `blackcometclub/M.I.O`
- private側の公開候補が`main`へcommit・push済み
- versionが次のファイルで一致している
  - `package.json`
  - `apps/desktop/package.json`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `Cargo.toml`
- `CHANGELOG.md`に同じtagの節がある
- public repositoryのローカルcloneがclean
- GitHub CLIがログイン済み

未追跡のFable文書とhandoff文書は公開候補に含めません。準備処理はcommit済みtreeからsourceを作るため、未追跡ファイルを読み込んだりstageしたりしません。

## 1. 公開候補をprivate側で確定する

機能修正、version、README、CHANGELOGを確認し、private repositoryへcommit・pushします。

この段階ではpublic repository、tag、GitHub Releaseを変更しません。commitとpushは通常どおり別々に承認を得ます。

## 2. 公開素材を一括準備する

repository rootでPowerShell 7を開き、次を実行します。

```powershell
& .\scripts\prepare-public-release.ps1
```

このcommandは次を自動実行します。

- tracked working treeとstageがcleanか確認
- 公開候補commitと`origin/main`の一致確認
- alpha versionとCHANGELOG節の確認
- TypeScript typecheck
- frontend build
- Rust format check
- Rust workspace test
- Windows x64向けNSIS installer build（RCは未署名、正式版はAuthenticode署名必須）
- commit済みtreeから履歴なしsource ZIPを生成
- 公開対象のfile listとmanifestを生成
- 公開sourceをGitleaksで検査
- source ZIPとinstallerのSHA-256を生成
- Release notesとmachine-readableな`release-plan.json`を生成

出力先は次の形式です。

```text
.tools/public-release-prep/<version>/<commit>-<timestamp>/
```

主な確認ファイル:

- `REVIEW.txt`: 人が読む短い結果
- `release-plan.json`: 次のcommandが読む固定計画
- `release-notes.md`: CHANGELOGから抽出した公開文案
- `artifacts/`: source ZIP、installer、manifest、SHA256SUMS
- `snapshot/source/`: public repositoryへ反映するsource tree
- `gitleaks-report.json`: 0 findingsの検査結果

このcommandだけでは、public repository、tag、GitHub Releaseを変更しません。

### 検査済み成果物だけを作り直す場合

通常公開では使いません。時間のかかるbuild/testを既に同じcommitで確認済みの場合だけ、理由を記録して次のoptionを使用できます。

```powershell
& .\scripts\prepare-public-release.ps1 -SkipChecks
```

source-only Releaseを準備し、installerを作らない場合は次を使います。

```powershell
& .\scripts\prepare-public-release.ps1 -SkipInstaller
```

正式な配布準備では、原則としてoptionなしのcommandを使用します。

正式版`1.0.0`以降では、certificate storeにあるcode signing certificateの正確な
subjectとthumbprint、issuer指定のRFC 3161 HTTPS timestamp URLを渡します。

```powershell
$certificate = Get-Item 'Cert:\CurrentUser\My\<thumbprint>'
& .\scripts\prepare-public-release.ps1 `
    -ExpectedSignerSubject $certificate.Subject `
    -SigningCertificateThumbprint $certificate.Thumbprint `
    -TimestampUrl '<issuerのRFC 3161 HTTPS URL>'
```

秘密鍵、token、passwordはcommand、repository、`release-plan.json`へ書きません。
正式版ではmain executableとinstallerの署名、certificate subject、chain、timestampを
検証できない場合、公開素材の生成を途中で停止します。RCの未署名実機試験は継続できます。

## 3. 公開前プレビュー

準備結果に表示された`release-plan.json`と、cleanなpublic repository cloneを指定します。

```powershell
& .\scripts\publish-public-release.ps1 `
    -Plan "<release-plan.jsonのpath>" `
    -PublicRepositoryPath "<blackcometclub/M.I.Oのclone path>"
```

このcommandは次を確認・表示するだけです。

- 対象repositoryとbranch
- source ZIPとinstallerのSHA-256
- public側で追加、変更、削除されるfile数
- version、tag、private source commit

ファイル変更、commit、push、tag作成、Release作成は行いません。

## 4. GitHub PrereleaseのDraftを作る

プレビュー結果をヒツジさんが承認した後、画面に表示されたexact source commitを指定して実行します。

```powershell
& .\scripts\publish-public-release.ps1 `
    -Plan "<release-plan.jsonのpath>" `
    -PublicRepositoryPath "<blackcometclub/M.I.Oのclone path>" `
    -CreateDraft `
    -ConfirmSourceCommit <40文字のprivate source commit>
```

このcommandは実行直前に次を再確認します。

- public cloneが`main`かつclean
- originが`blackcometclub/M.I.O`
- fetch後の`HEAD`と`origin/main`が一致
- 同名tagが存在しない
- 公開file listが準備済みsnapshotと一致

確認後、次を実行します。

- public cloneへsnapshotのfileだけを明示的に反映
- snapshotのfileだけを明示的にstage
- public release commitとannotated tagを作成
- `main`とtagをatomic push
- source ZIP、manifest、SHA256SUMS、installerを添付したDraft Prereleaseを作成

`git add .`、`git add -A`、reset、cleanは使用しません。途中で失敗した場合も自動的に状態を消さず、調査できるようその場で停止します。

## 5. Draftを公開する

GitHub上のDraftで、版番号、説明、添付ファイル、hashを確認します。ヒツジさんが公開を明示承認した後、次を実行します。

```powershell
& .\scripts\publish-public-release.ps1 `
    -Plan "<release-plan.jsonのpath>" `
    -PublicRepositoryPath "<blackcometclub/M.I.Oのclone path>" `
    -PublishDraft `
    -ConfirmTag <vで始まるtag>
```

公開後は、Release URL、匿名download、asset名とSHA-256、public CIを確認します。この公開後確認は外部状態を読むため、command本体とは分けて記録します。

## 承認境界

- 準備: 外部変更なし。通常の作業承認で実行可能
- プレビュー: 外部変更なし
- Draft作成: public repositoryのcommit、push、tag、Draft Releaseを作るため、直前の明示承認が必要
- Draft公開: Releaseを外部公開するため、別の明示承認が必要
- SNS投稿: Release公開とは別扱い。投稿直前の明示承認が必要

## 省略しないもの

作業回数は減らしますが、次は省略しません。

- private履歴とpublic snapshotの分離
- 未追跡Fable／handoff文書の保護
- versionとcommitの固定
- secret scan
- SHA-256確認
- public差分の確認
- 外部公開前のOwner承認
- 公開後の匿名アクセス確認

## Installerについて

RC用に`prepare-public-release.ps1`が生成するinstallerは未署名です。生成できたことだけで
正式V1とは扱いません。Ownerが未署名RC／Previewの公開を承認した場合だけ、表示名へRCまたは
Previewを付け、未署名であること、SHA-256、source commit、SmartScreen警告時の確認方法を
公開ページへ記載します。NSIS正式版ではADR 0045に従って署名引数を必須とし、
`scripts/test-windows-release-signature.ps1`がmain executableとinstallerを検証します。
Microsoft Store用MSIXは別の成果物・検証経路とします。現在のSubmissionは認定に合格し、
`今すぐ公開`を選ぶまで公開しない設定で保留されています。認定合格だけでは公開を開始せず、認定結果と
最終版の整合を確認してから、ヒツジさんがStore公開を明示承認した場合だけ`今すぐ公開`へ進みます。
GitHub ReleaseやレーベルサイトもM.I.O.側から自動公開することはありません。

Microsoft Storeへ提出したMSIXが公開された後は、GitHub Releaseの手順と混ぜず、
`docs/MICROSOFT-STORE-POST-PUBLICATION-CHECKLIST.md`に従って、匿名Store page、
Storeからの取得、Windows 11実機、既存Room data、レーベルサイトの順に確認します。
Storeからのinstall／uninstall、レーベルサイトのpush／deploy、GitHub Releaseは、
それぞれ別の承認境界として扱います。

初めてinstallerをReleaseへ添付する前、またはinstaller設定を変更した場合は、既存のM.I.O.データと導入先を保護したうえで、install、初回起動、終了、uninstall、既存データ非破壊を実機確認します。通常のsource-only更新で同じ検証を無条件に繰り返す必要はありません。
