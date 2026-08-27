# Remote Relay

Claude Webなど、ローカルPCへ直接接続できないWebサービスとの通信を中継する領域です。

Relayは最小限の状態だけを扱い、プロジェクト全体、会話全文、Artifactを恒久保存しない構成を候補とします。Desktop側のproduction transport候補はADR 0008でHTTPS + OS証明書検証へ固定しました。常設Relayのhosting、credential record、rotation、運用方式はまだ未決定です。

2026-08-12の `spikes/relay-roundtrip/` では、Desktopからの外向き永続link、request correlation、切断時の即時エラー、再接続後の復旧をlocalhostで確認しました。Relayが持つ状態はDesktop接続状態と処理中requestだけで、Room snapshot保持数は0です。PoCのHTTP NDJSON transportを製品実装へそのまま採用する決定ではありません。

同日のRemote MCP統合probeでは、公式MCPクライアント → Remote MCP → Relay → Desktop Room source → Relay → MCP resultの全経路もPASSしました。公開ネットワークは使わず、Relay URLはloopback originだけに制限しています。

device pairing probeでは、短時間・単回code、誤入力制限、期限切れ、credential失効、再pairingを確認しました。秘密の生値はmemoryにも保存せず、process-memory keyによるHMACだけを保持します。productionではOS credential vaultとRelay側の安全な永続credential recordが別途必要です。

`crates/moe-relay-transport/` は `https://<DNS hostname>[:port]/desktop-link` だけを受け付け、Windowsの証明書ストアとhostname検証、I/O deadline、cancel可能なsocket、8 KiBのheader / frame上限を持ちます。生成CAを使うローカルTLS fixtureはPASSしています。公開Relay接続はまだ行っていません。

Desktop orchestratorへの製品設定結合はADR 0009で完了しました。設定済みbuildはWindows Credential Managerからcredentialを直接loadし、HTTPS認証、切断後のbounded retry、stopまでをbackground taskで処理します。公開Relayの配布先とRelay側credential recordは未決定です。

ADR 0010で、確立済みHTTPS link上の `moe_read_room` requestをDesktopのprovider-neutral Room sourceへ結ぶ製品routerを追加しました。request ID相関、重複上限、未知method・不正paramsの固定error、cancel後のresponse抑止を持ちます。現在のsourceはDesktop起動時の読み取り専用bootstrap snapshotであり、React UI state、message write、永続化、Artifact、公開Relayはまだ結合していません。
