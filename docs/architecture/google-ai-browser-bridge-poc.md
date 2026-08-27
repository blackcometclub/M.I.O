# Google AI Browser Bridge PoC

Status: Archived experiment / Not a product feature

## Outcome

Google Search / AI ModeをM.I.O.の `Gemini Search` 参加者として扱うため、ブラウザ拡張とM.I.O.デスクトップの間にloopback bridgeを置く。

体験評価の結果、正式な製品機能にはせず、後日再実行できるお遊び用PoCとして保存する。通常起動では `MOE_EXPERIMENT_GOOGLE_AI_BRIDGE=1` がないためBridgeを開始しない。

このPoCはGoogleの非公開APIや認証情報を利用しない。外部サービスへの最終送信と回答の選択は利用者がGoogle画面上で確認して行う。

## Flow

1. OwnerがM.I.O.で `Gemini Search` 宛に発言する
2. Roomへhuman messageを永続化する
3. AI dispatchが質問とbounded Room contextをbrowser outboxへ登録する
4. 拡張が `127.0.0.1:38473` から質問を取得する
5. 利用者が「質問をGoogleへ入力」を押し、内容を確認後にGoogle側で送信する
6. 利用者が表示済み回答を選択し「M.I.O.へ返す」を押す
7. M.I.O.がtoken、Google URL、Room contractを検証する
8. `authorId = gemini`、`recipients = [owner]` のRoom messageとして永続化し、UI eventを発行する

## Safety boundary

- bind先はloopback固定
- 専用ヘッダーがないHTTP要求は拒否。Edge拡張backgroundのGETはOriginなしになるため許可するが、Web Originつき要求は拡張Origin以外を拒否
- dispatchごとの一回用reply tokenを検証
- GoogleのCookie、認証情報、非公開RPC、network responseを取得しない
- Googleへ自動submitしない
- 回答は利用者が選択した表示済み文字列だけ
- Google source URLはhost/pathだけを送信し、query/hashは送信しない
- Room write上限を超える回答はUTF-8境界で切り、末尾へ省略表示を付ける
- M.I.O.への保存成功後だけUI eventを発行する

## PoC limitations

- outboxとdispatch tokenはprocess内だけで、再起動すると失われる
- 同じ拡張を開いた複数Google tabが同じpending dispatchを見られる
- Google入力欄の構造変更時は自動入力が失敗し、clipboard fallbackになる
- 回答本文と引用を構造化せず、選択文字列だけを扱う
- unpacked extensionの配布・署名・更新機構は未設計
- loopback port競合時はGemini Searchを接続済みと表示しない

## Before productization

provider-neutralなbrowser adapter contract、永続dispatch ledger、明示的pairing、複数tab ownership、引用情報schema、拡張の配布・更新・権限説明を別ADRで決定する。PoC成功だけで正式なGoogle integration完成とは扱わない。

現時点ではproductizationを予定しない。再開時は `spikes/google-ai-mode-browser-bridge/run-experiment.cmd` を入口にし、この文書の境界を再確認する。
