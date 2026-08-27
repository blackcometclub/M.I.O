# 0008: Production Relay TLS transport

Status: Accepted

Date: 2026-08-12

## Context

localhost限定のHTTP chunked NDJSON driverで、Windows Credential Managerからの認証、Room応答、切断、自動再接続、停止を製品serviceとDesktop orchestratorまで結合できた。しかし、このdriverは平文HTTPであり、公開Relayへ昇格できない。製品transportにはTLS証明書とhostnameの検証、接続とI/Oの期限、blocking I/Oの停止、受信量の上限、credentialを余分なapplication bufferへ複製しないAuthorization生成が必要である。

現在のDesktop runtimeは標準thread上の同期I/Oで構成される。TLSのためだけに別のasync runtimeやWebSocket stackを加えると、所有権とcancel境界が二重になる。

## Decision

- 製品候補を `crates/moe-relay-transport/` とし、`moe-relay-client::RelayTransport` を同期HTTPSで実装する。
- TLSは `rustls` の安全な既定protocol versionとring providerを使う。client application向けの `rustls-platform-verifier` でOSの信頼済み証明書・hostname・失効方針を適用する。
- endpointは `https://<DNS hostname>[:port]/desktop-link` だけを受け付ける。平文HTTP、IP literal、userinfo、query、fragment、別path、0番port、不正なDNS labelはnetwork I/O前に拒否する。
- 証明書検証を無効化する製品APIは作らない。単体試験だけがprivate constructorへ明示的なclient configと生成CAを渡す。
- TCP接続は解決済みaddressごとに `connect_timeout` を適用する。TLS handshakeと各高水準read/write操作は共通deadline内に完了させる。
- TLS socketをnonblockingとし、最大5 ms間隔でcancel flagとdeadlineを確認する。`RelayShutdownHandle` は同じsocket handleへcancel flagと `Shutdown::Both` を適用し、blocking readを停止できるようにする。
- HTTP response headerとtrailerの合計、chunk、1 NDJSON frameを各8 KiB以下へ制限する。HTTP/1.1、200、`application/x-ndjson`、chunked transfer、正しい `hello_ack` を要求する。
- 401 / 403だけをcredential拒否へ分類する。証明書、hostname、socket、timeout、protocolの詳細やserver bodyはmetadataへ運ばない。
- device credentialはBearer tokenとして安全なASCII文字だけを許可し、`String` やheader mapへ複製せず、`SecretBytes` の借用sliceからTLS streamへ直接writeする。
- helloのprotocol versionは `moe-protocol::PROTOCOL_VERSION` を参照し、transport内へ重複定義しない。

## Consequences

- 公開Relayへ使えるTLS client境界と、Room request / responseを継続できるstream connectionが製品crateとして成立する。
- ローカルTLS fixtureで、信頼済み証明書の成功、未信頼証明書とhostname不一致の拒否、401分類、header / frame上限、read timeout、cancelによるread解除を公開networkなしで検証できる。
- OS verifierを使うため、配布先OSのtrust storeと更新・失効設定が結果へ反映される。独自CAを製品側で暗黙に信頼しない。
- 標準libraryのDNS解決そのものにはこのTCP connect timeoutが適用されない。Relay endpointの固定方法と名前解決の全体deadlineは、Desktop統合時に別途決める。
- このtrancheではtransport crateをDesktop orchestratorのtask factoryへまだ結合せず、公開Relay、Relay側credential永続化、rotation、複数accountも検証しない。
