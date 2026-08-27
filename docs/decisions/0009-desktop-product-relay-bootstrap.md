# 0009: Desktop product Relay bootstrap

Status: Accepted

Date: 2026-08-12

## Context

ADR 0008でHTTPS、OS証明書・hostname検証、I/O deadline、cancel、frame上限を持つ製品transportが成立したが、Desktopアプリはまだlocalhost probeの外でtransportを生成せず、Windows Credential Managerからの認証と自動再接続runtimeへ結ばれていなかった。

この段階でWebViewやcommand lineから任意endpoint、account、device、credentialを受け取ると、TLS transportの安全なendpoint制約を迂回する設定面とsecret入力面を同時に増やす。常設Relayの配布先が未決定な間は、接続先metadataを配布物の信頼境界へ閉じる必要がある。

## Decision

- Desktop backendへ `relay_product` moduleを追加し、`RelayClientService<PlatformCredentialStore>`、`RelayHttpsTransport`、`DesktopRelayOrchestrator`を実task factoryで結合する。
- 非secretのendpoint、account ID、device IDはbuild-timeの `MOE_RELAY_ENDPOINT`、`MOE_RELAY_ACCOUNT_ID`、`MOE_RELAY_DEVICE_ID`からだけ取得する。WebView、Tauri command、runtime environment、command line、任意設定fileからは受け取らない。
- 3項目がすべて未指定ならRelay未構成として通常起動する。3項目がそろった場合だけ自動startする。不完全、HTTP、IP literal、不正path、不正identityはTauri起動前にfail closedとする。
- `RelayClientService`を `Arc` でTauri stateとbackground taskが共有する。taskごとの接続時にserviceがWindows Credential Managerからcredentialをloadし、transportへ寿命付き借用値として渡す。
- TLS handshakeと `hello_ack` の後にruntimeへconnectedを報告する。connectionのshutdown handleをDesktop cancellationへ登録し、stop / app shutdownがTLS readを中断してworkerをjoinできるようにする。
- HTTPS接続の固定error分類を `RelayTransportError::safe_error_code()` としてclient crateへ集約し、Desktopとserviceが同じ分類を使う。
- Relayから確立後frameを受け取った場合、Room router未実装の間はprotocol errorで切断し、既存のbounded retryへ戻す。未知frameを黙って破棄したり、未検証payloadをWebViewへ渡したりしない。
- test用CA注入はdefault featureから外した `test-root-certificate` featureだけへ隔離する。通常の製品buildにはplatform verifier以外のtrust root注入APIを含めない。

## Consequences

- Desktop起動時に、設定済み配布物はOS credentialを直接使ってproduction HTTPS transportを開始し、UI pollなしで切断後の自動再接続まで進められる。
- 公開Relayがまだ無くても、生成CAのlocalhost TLS Relayと固定test credentialで、認証、切断、1秒retry、別generation再認証、stopを統合検証できる。
- Windows Credential Managerの隔離targetを使った試験で、store、製品service load、2回のTLS認証、cancel、offline、delete、not foundを確認できる。
- 未構成buildは既存UIを変えず、networkへ接続しない。秘密credentialをbuild metadataへ埋め込まない。
- Room request routerとresponse生成、pairing UI、endpoint配布、公開Relay、credential rotationは未完である。現在の接続は受信frameが来るまで待機するだけで、会話本文を配送しない。

## Follow-up

Room request routerとresponse生成はADR 0010で実装した。その他の判断と未完項目は維持する。
