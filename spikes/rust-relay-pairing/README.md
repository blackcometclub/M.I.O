# Rust Relay pairing / connection integration spike

Node Relayのlocalhost `/pair` と製品候補のRust Relay client／Windows Credential Managerを結ぶWindows専用probeです。

- Relay endpointは`http://127.0.0.1:<port>`だけを許可する
- pairing codeはprivate environment variableから読み、直後にprocess環境から除去する
- pairing codeをcommand line、stdout、stderr、証跡へ出さない
- HTTP response全体とdevice credentialを消去対象bufferへ閉じる
- device credentialをWebViewやNode orchestratorへ返さず、Rust内部からWindows Credential Managerへ保存する
- 製品`RelayConnectionManager`がWindows Credential Managerからcredentialをloadする
- Authorization値は借用secret bufferからsocketへ直接書き、header文字列へ複製しない
- HTTP chunked NDJSONで認証hello、Room request / response、切断を確認する
- 製品`DesktopRelayOrchestrator`が切断を検知し、1秒timer後に新しいgenerationで再認証接続する
- cancel hookがsocket readを中断し、手動stopから1秒未満でworkerをjoinする
- credential削除後はnetworkへ触れる前に再接続を拒否する
- probe専用accountだけを使用し、成功・失敗どちらでもcleanup processが削除を再確認する

実行は先にbinaryをbuildし、pairingは`spikes/relay-roundtrip/probe-rust-product-pairing.mjs`、接続・再接続は`spikes/relay-roundtrip/probe-rust-product-connection.mjs`から行います。公開network、TLS/WSS、製品UIを検証するものではありません。
