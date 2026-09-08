# M.I.O. alpha.2 Windows installer smoke — 2026-08-29

## Result

**PASS with one observation boundary:** interactive install、installed app startup、quiet uninstall、existing-data restorationは合格した。Computer Useの製品ポリシーが`uninstall.exe`のUI起動を遮断したため、interactive uninstall画面だけは未観察である。

この結果だけで署名済み一般配布を完成扱いにしない。検証対象installerは未署名である。

## Artifact

- Version: `0.1.0-alpha.2`
- File: `M.I.O._0.1.0-alpha.2_x64-setup.exe`
- Size: `3,590,163 bytes`
- SHA-256: `958F78F6F747F35A172564410CB0FE7A70053CF57C2C50AF6303DBB1DD9A2112`
- Authenticode: `NotSigned`
- Build command: `& .\scripts\prepare-public-release.ps1`
- Build source commit: `2b4a768fe8a3bf74c648f793bb362eb5b165e108`

## Environment and scope

- Host Windowsの通常ユーザー環境で実施した。Windows Sandboxは使用していない。
- インストール開始前、`moe-desktop` processは0件だった。
- `%LOCALAPPDATA%\M.I.O.`は存在しなかった。
- M.I.O.のユーザー用uninstall登録は存在しなかった。
- Provider login、外部送信、Room編集、Preferences変更は行っていない。

## Existing-data protection

実データはTauri identifier `app.moe.desktop`のRoaming data directoryに存在した。

- Roaming source: `%APPDATA%\app.moe.desktop`
- Roaming backup: `%APPDATA%\app.moe.desktop.installer-smoke-backup-20260829-1025`
- Local source: `%LOCALAPPDATA%\app.moe.desktop`
- Local backup: `%LOCALAPPDATA%\app.moe.desktop.installer-smoke-backup-20260829-1025`

M.I.O.停止を確認し、sourceとbackupが同じ親directory内にあること、backup名が未使用であることを確認してから、両方を一時的にrenameした。削除は行っていない。

Roaming側の既存fileは15件だった。rename前に全fileのrelative path、size、SHA-256を取得した。

## Interactive install

Installer UIで次を確認した。

1. `Welcome to M.I.O. Setup`
2. current-user install先が`%LOCALAPPDATA%\M.I.O.`
3. 必要容量表示が`15.1 MB`
4. 管理者権限要求なしでinstallation completeへ到達
5. 完了画面に`Run M.I.O.`と`Create desktop shortcut`

install後の実測:

- `%LOCALAPPDATA%\M.I.O\moe-desktop.exe`: `15,808,000 bytes`
- `%LOCALAPPDATA%\M.I.O\uninstall.exe`: `79,154 bytes`
- HKCU uninstall display name: `M.I.O.`
- HKCU uninstall version: `0.1.0-alpha.2`
- Desktop shortcut target: `%LOCALAPPDATA%\M.I.O\moe-desktop.exe`

## First startup

完了画面の`Run M.I.O.`からinstalled executableを起動した。

- Process path: `%LOCALAPPDATA%\M.I.O\moe-desktop.exe`
- Window title: `M.I.O.`
- `Core + Room ready`へ到達
- 退避した実データではなくbundled demo Roomを表示
- Providerへのmessage送信なし
- 設定変更なし
- 正常終了後、`moe-desktop` processは0件
- 新しいRoaming product data fileは作成されなかった
- Local側にはWebView2 cacheが作成された

## Uninstall

Computer Useから付属`uninstall.exe`を起動しようとしたところ、`product policy blocks this app`でUI automationが停止した。重複起動は行っていない。

対象path、M.I.O.停止、install root内の`uninstall.exe`であることを再確認し、同じ付属uninstallerをNSIS quiet optionで実行した。

```powershell
uninstall.exe /S
```

- Exit code: `0`
- `%LOCALAPPDATA%\M.I.O.`: removed
- Desktop shortcut: removed
- HKCU uninstall registration: removed
- `moe-desktop` process: 0

interactive uninstall画面の表示とbutton操作は未観察である。uninstall処理そのものとcleanup結果は合格した。

## Restore

初回起動で作られたLocal WebView2 cacheは削除せず、次へrenameして保存した。

`%LOCALAPPDATA%\app.moe.desktop.installer-smoke-new-20260829-1059`

その後、元のRoaming backupと元の空Local directoryをoriginal nameへ戻した。

restore後の検証:

- Expected existing data files: `15`
- Actual existing data files: `15`
- Missing files: `0`
- SHA-256 mismatches: `0`
- Roaming backup name: no longer present（original nameへ復元済み）
- Local backup name: no longer present（original nameへ復元済み）
- Saved test WebView2 cache: present

## Decision boundary

合格したもの:

- 未署名NSIS installerの生成
- current-user interactive install
- installer完了画面からのinstalled app起動
- fresh-state `Core + Room ready`
- desktop shortcut作成とtarget
- quiet uninstall
- install directory、shortcut、uninstall登録のcleanup
- existing M.I.O. dataのbyte-identical restoration

未完了または別判断のもの:

- interactive uninstall UIの実画面確認
- code signing
- SmartScreen reputation
- automatic update
- installerをGitHub Releaseへ添付する公開判断
- clean machineでWebView2が存在しない場合のdownload bootstrapper再確認
