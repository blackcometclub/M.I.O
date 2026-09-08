# M.I.O. V1 利用ガイド

[English](USER-GUIDE.md) | **日本語**

この文書は一般公開済みのWindows V1系を対象にしています。推奨する導入方法は署名済みのMicrosoft Store版です。
BOOTH／itch.ioでは、直接downloadを明示的に選ぶ利用者向けに、別配布の未署名`v1.0.3` Previewを提供しています。
ローカルでbuildしたinstallerは公式storefrontのdownloadではありません。

## 1. 動作条件

- 64-bit版Windows 11
- Microsoft Edge WebView2 Evergreen Runtime
- WebView2取得、Provider CLIの導入・認証、Providerへのmessage送信、明示確認したnpm package導入時だけinternet接続
- 利用するAIに対応するProvider CLI。M.I.O.本体はProvider CLIが一つもなくても起動可能

Windows 10、Windows on Arm、32-bit Windows、macOS、LinuxはV1の対応対象ではありません。

## 2. インストールと初回起動

1. 推奨： [Microsoft Storeページ](https://apps.microsoft.com/detail/9NS9B7T71XHN)を開き、
   **入手**または**インストール**を選びます。署名済みpackageの検証と導入はMicrosoft Storeが行います。
2. 直接downloadする場合は、公開中の[BOOTH](https://tinmoon.booth.pm/items/8807279) ZIPまたは
   [itch.io](https://tinmoon-label.itch.io/mio-talk-room) installerだけを使います。これらは明示的に
   **未署名Preview**として配布しています。実行前に、同じstorefront pageに記載されたSHA-256と照合します。

   ```powershell
   Get-FileHash .\M.I.O_1.0.3_windows-x64_unsigned-preview_setup.exe -Algorithm SHA256
   ```

3. Microsoft Store版はStore経由で署名されます。BOOTH／itch.io版にはAuthenticode発行者がなく、
   SmartScreenが表示される場合があります。この境界を許容しない場合はMicrosoft Store版を使用します。
4. 直接導入では、起動中のM.I.O.を終了してinstallerを実行し、WindowsのUACを承認します。
   全ユーザー用として`%ProgramFiles%\M.I.O`へ導入するため、管理者の承認が必要です。
5. WebView2がない場合は、直接配布installerがMicrosoft公式bootstrapperをdownloadします。この段階だけinternet接続が必要です。
6. Start menuから **M.I.O.** を起動します。Roomを作成または選択し、利用するAIだけを追加します。
   local folderが必要になるまでは、participant権限を **会話のみ** のままにします。

直接配布installerにはproject licenseと`THIRD-PARTY-NOTICES.txt`を同梱します。

## 3. Provider CLIの導入とlogin

M.I.O.はProvider CLIの導入、credential入力、loginを代行しません。M.I.O.を終了し、[READMEの公式手順](../README.ja.md#provider-cliの導入と認証)から利用するProviderだけを導入します。CLIを単独起動してloginを完了してからM.I.O.を再起動します。

- password、OAuth認証code、cookie、API keyをTalk Roomへ貼り付けないでください。
- CLIを検出しただけでは、account、model、契約、利用上限が使える証明になりません。適切なlive replyが成功してから利用可能になります。
- 明示したmodelを使えない場合、別modelのreplyを要求どおりの成功として扱いません。

## 4. 手動update

V1に自動updateはありません。

1. **Room settings** でbackup先を選び、**Back up**を実行します。
2. M.I.O.を終了します。
3. Microsoft Store版はMicrosoft Storeから更新します。直接配布版は、同じBOOTHまたはitch.io pageから
   新fileを入手し、pageに記載されたSHA-256と照合してから実行します。
4. 既存版が`%LOCALAPPDATA%\M.I.O`の現在利用者用alphaの場合は、保持dataを削除せずにappだけを
   uninstallしてからV1を導入します。既存の全ユーザー用直接配布V1は通常そのまま上書きできます。
   Store版と直接配布版を同じpackageとして切り替えず、配布経路を変える場合は保持dataを削除せず現在のappをuninstallします。
5. M.I.O.を起動し、Room一覧、participant profile、appearance、workspace権限、model選択、最近の会話履歴を確認します。

Release notesのmigration対象が現在のversionと一致しない場合、Store署名が無効な場合、または直接配布fileの
hashがstorefront pageと一致しない場合はupdateを止めます。

## 5. Backupと端末内data

M.I.O.の製品dataは`%APPDATA%\app.moe.desktop`へ保存します。WebView2 cacheは`%LOCALAPPDATA%\app.moe.desktop`にも作られる場合があります。

**Back up all Rooms**はRoom名と会話履歴をJSONへ保存します。既定の保存先は`Documents\M.O.E Backups`で、Room settingsから別のfolderを選択できます。保存先を変更しても、既存のbackup fileは移動しません。

復元前に、最新backupのfile名、時刻、Room数を表示します。復元すると、現在のRoom一覧と会話履歴を確認済みbackupの内容へ置き換えます。participant profile、appearance、Provider credential、workspace権限、model選択、すべてのdispatch記録を復元する機能ではありません。

重要なbackupは別driveなど、利用者が管理できる別の場所にも保管してください。M.I.O.がbackup fileをuploadすることはありません。

## 6. AIへ送るdata

M.I.O.はローカル優先ですが、接続するAI modelの多くはremote serviceです。

| Data | 境界 |
|---|---|
| 現在のmessageと範囲を限定したRoom文脈 | 明示的に選んだ宛先、または選択したConductor／worker経路だけへ送信 |
| participant表示名と端末内AI案内 | そのparticipantのreplyを調整する必要がある場合に送信 |
| 選択folderの内容 | Codex participantへ読取または読取・編集権限を与えた場合だけ利用可能。M.I.O.が仲介し、選択folder内へ限定 |
| 固定開発操作 | 対象となるCodex読取・編集profileだけで利用可能。重要操作はOwner確認を要求 |
| 画像生成promptと希望設定 | 現在のmessageが画像生成を明示的に頼んだ場合だけCodexへ送信 |
| password、OAuth code、API key、Provider cookie | M.I.O.へ入力しない。Provider CLIのloginはM.I.O.外で実施 |

Provider側の推論、契約、利用上限、課金、保存期間、学習利用、account規則はM.I.O.ではなく各ProviderとCLIが管理します。非公開情報や有料業務の内容を送る前に、Providerの最新規約とaccount設定を確認してください。V1に公開Remote Relayはなく、M.I.O.がCLI会話用の中継cloud serviceを運営することもありません。

## 7. アンインストール

1. 後で必要になる可能性があればRoomをbackupします。
2. M.I.O.を終了します。
3. **Windows設定 → アプリ → インストールされているアプリ → M.I.O → アンインストール**を開き、確認して進めます。

uninstallerはinstall済みapp、同梱command helper、第三者notice、shortcut、uninstall登録を削除します。updateや再installでRoomを予告なく消さないよう、端末内の製品dataとWebView2 cacheは残します。

残ったdataも完全に削除する場合は、backupを確認してから`%APPDATA%\app.moe.desktop`と`%LOCALAPPDATA%\app.moe.desktop`を利用者自身で削除します。この操作はM.I.O.から元に戻せません。Provider CLIとcredentialは別appのため削除しません。

## 8. 困ったとき

### Provider CLIが見つからない

M.I.O.を終了し、新しいPowerShell windowでCLIの`--version`が成功することを確認してからM.I.O.を再起動します。CLI導入前から開いているterminalには古い`PATH`が残る場合があります。

### 導入済みだが利用可能にならない

CLIを単独起動して公式loginを完了します。選択model、契約、利用上限も確認します。login結果をM.I.O.へ貼り付けないでください。

### Timeoutまたは結果不明

Provider accountとnetworkを確認します。M.I.O.が「届いたか不明」と表示した場合、重複turnを防ぐため自動再送しません。Provider側を確認してから、新しいmessageを送るか判断してください。

### SmartScreen警告または署名の問題

Microsoft Store版はStore経由で署名されます。別配布のBOOTH／itch.io installerは未署名Previewのため、
SmartScreenが表示される場合があります。同じstorefront pageに記載されたSHA-256と照合してください。
Store版の署名が無効な場合、または直接配布版のhashが異なる場合は実行しません。

### WebView2がない

internetへ接続して公式installerを再実行し、Microsoft WebView2 bootstrapperを完了させます。または[Microsoft公式WebView2 page](https://developer.microsoft.com/microsoft-edge/webview2/)からEvergreen Runtimeを導入します。

### 復元できない

表示中のbackup directoryが存在し、変更されていない`moe-room-backup-*.json`があることを確認します。もう一度 **Review restore**を実行してください。確認後から復元までの間にbackupをrename・編集しません。

## 9. V1の主な非対応範囲

- 公開Remote Relay、複数device同期、複数account管理
- 自動update、background automation、無制限Conductor round、nested delegation
- Claude CodeとGeminiのworkspace読取・編集。GrokはOwnerが選択した追跡済みGit status／差分の
  読取専用reviewだけに限定。Codexの任意shell・desktop自由操作
- token単位のstreaming表示
- 生成画像の正確なpixel数保証。アスペクト比・品質は希望値で、取込画像は16 MiBまで
