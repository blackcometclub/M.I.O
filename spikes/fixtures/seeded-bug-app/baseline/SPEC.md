# Delivery plan fixture

`buildDeliveryPlan(participants)` は、トークルームで選択されたAIだけを配送先へ変換します。

## Rules

1. `selected: true` の参加者だけを扱う。未選択の参加者は結果へ一切含めない。
2. 同じ `adapterInstanceId` の選択済み参加者が複数ある場合は、入力順で最初の1件だけを使う。
3. 未選択の古いregistry行が同じ `adapterInstanceId` で先に存在しても、後ろの選択済み行を抑止してはならない。
4. `connection: "connected"` は `recipients`、それ以外は `blockedRecipients` へ入れる。
5. 選択済みが0件なら `status: "blocked"` / `reason: "no_recipients"`。
6. 選択済みはあるが接続済みが0件なら `status: "blocked"` / `reason: "all_selected_offline"`。
7. 接続済みと未接続が混在する場合は `status: "ready"` / `warning: "some_selected_offline"`。
8. 全員接続済みなら `status: "ready"` で `reason` と `warning` は `null`。

修正してよいのは `src/delivery-plan.mjs` だけです。test、仕様、画像、package.json、sentinelは変更禁止です。
