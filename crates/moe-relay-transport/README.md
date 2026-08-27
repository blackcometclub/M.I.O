# M.O.E. Relay transport

Desktop から常設 Relay の `/desktop-link` へ接続する、同期 HTTPS transport です。

- `https://` と検証済み DNS hostname だけを受け付けます。
- Windows では OS の信頼済み証明書ストアと失効方針を使い、TLS hostname を検証します。
- connect / read / write timeout と、HTTP header / NDJSON frame の上限を持ちます。
- device credential は `String` や header map に複製せず、借用 byte slice から TLS stream へ直接書き込みます。
- blocking I/O は `RelayShutdownHandle` で外部から中断できます。

ローカル試験だけは生成した CA を明示的に信頼する client config を注入します。証明書検証を無効化する経路はありません。
