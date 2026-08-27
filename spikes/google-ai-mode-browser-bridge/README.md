# M.O.E. Google AI Bridge (往復PoC)

> お遊び用として保存された実験版です。通常のM.O.E.では無効で、正式なGemini adapterではありません。

Google Search / AI ModeとM.O.E.を、利用者の確認操作を残したまま往復接続する最小実験です。

Googleの非公開API、Cookie、認証情報、通信内容にはアクセスしません。質問はGoogleの入力欄へ入れるだけで自動送信せず、回答も利用者が明示的に選択した文字列だけをM.O.E.へ返します。

## 試し方

### 1. M.O.E.お遊び版を起動する

通常のM.O.E.を完全に終了してから、同じフォルダーにある `run-experiment.cmd` をダブルクリックします。初回は正規Tauri buildを行うため数分かかる場合があります。buildが成功すると、実験フラグ `MOE_EXPERIMENT_GOOGLE_AI_BRIDGE=1` つきでM.O.E.が起動します。

通常版とお遊び版を同時起動しないでください。どちらも同じRoom保存データを使うためです。

### 2. 拡張を読み込む

### Chrome

1. `chrome://extensions` を開く
2. 右上の「デベロッパーモード」を有効にする
3. 「パッケージ化されていない拡張機能を読み込む」を選ぶ
4. この `google-ai-mode-browser-bridge` フォルダーを選ぶ

### Edge

1. `edge://extensions` を開く
2. 左側の「開発者モード」を有効にする
3. 「展開して読み込み」を選ぶ
4. この `google-ai-mode-browser-bridge` フォルダーを選ぶ

### 3. 往復する

1. 最新版のM.O.E.を起動したまま、Google検索／AI Modeを開く
2. M.O.E.で参加者 `Gemini Search` を宛先にして質問を送る
3. Google右下のBridgeに質問到着が表示されたら「質問をGoogleへ入力」を押す
4. 入力内容を確認して、Google側の送信ボタンを押す
5. Geminiの回答本文だけをドラッグ選択する
6. 「選択回答をM.O.E.へ返す」を押す

成功すると、回答はユーザーの貼り付け発言ではなく、M.O.E.の `Gemini Search` 本人の発言としてRoomへ保存されます。

## このPoCの境界

- 環境変数 `MOE_EXPERIMENT_GOOGLE_AI_BRIDGE=1` がない通常起動ではBridgeを開始しない
- M.O.E.と拡張の通信先は `127.0.0.1:38473` のみ
- Google上で利用者が明示的に選択した文字列だけを返す
- GoogleのDOM構造から回答を自動推測しない
- 質問をGoogleへ自動送信しない
- M.O.E.から届いた質問ごとの一回用トークンが一致する返答だけを受け付ける
- 回答がRoomの保存上限を超える場合は末尾を明示的に省略する

## 次の判断候補

PoCで操作感を確認できた後に限り、Google回答と引用リンクの限定的な構造化取得、複数のブラウザAIへ共通化できるadapter contract、永続dispatch ledgerを個別に検討します。

## 遊び終わったら

M.O.E.お遊び版を終了し、Edgeの拡張管理画面でこの拡張をOFFにします。次回は `run-experiment.cmd` と拡張のONだけで再開できます。
