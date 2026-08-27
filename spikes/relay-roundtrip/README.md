# Desktop outbound Relay roundtrip spike

M.O.E. Desktop役がRelayへ外向きの永続接続を張り、Relay側のRoom要求をDesktopへ転送して結果を対応付けられるかを、公開ネットワークなしで確認するPoCです。

Phase 0のtransport-neutralな境界確認なので、完成版のWSSそのものではありません。Node標準機能だけで、localhost上の双方向HTTP NDJSON streamを使います。RelayはRoom本文やsnapshotを保持せず、接続状態と処理中requestの相関だけを持ちます。

```powershell
npm.cmd test --workspace @moe/relay-roundtrip-spike
```

device pairing contract:

```powershell
npm.cmd run test:pairing --workspace @moe/relay-roundtrip-spike
```

検証項目:

- DesktopからRelayへの外向き接続
- probe専用Bearer tokenが違う接続の拒否
- 同時Room要求2件のrequest correlation
- Relay入力でraw filesystem pathを拒否
- Relay切断状態で即座に `desktop_offline`
- Desktop再接続後のRoom読み取り復旧
- RelayのRoom保持件数が常に0
- localhost以外へ通信しない

Pairing probeでは、Relay側のホスト操作で8文字の短時間・単回コードを発行し、Desktopが `/pair` で一度だけdevice credentialへ交換します。Relayはコードとcredentialの生値を保持せず、process-memory keyを使ったHMAC-SHA-256だけをmemory上に持ちます。誤入力5回、期限切れ、再利用、失効済みcredentialを拒否し、失効時は接続中の同じdeviceも切断します。

このPoCはdevice pairingのローカルcontractまでを実装しますが、永続credential保管、OS credential vault、TLS/WSS、公開配置、Artifact、Job、idempotentな書き込みは実装しません。観測結果を製品Relayへそのままコピーせず、protocol契約と安全境界だけを昇格候補にします。

Remote MCPを入口にした全経路は、隣のworkspaceから次で検証します。

```powershell
npm.cmd run test:relay-integration --workspace @moe/remote-mcp-spike
```
