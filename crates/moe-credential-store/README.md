# moe-credential-store

M.O.E. Desktop backend専用のdevice credential保管境界です。

- target schema: `M.O.E./relay-device/v1/<account-id>`
- account ID: 小文字英数字で開始し、以後は小文字英数字・`.`・`_`・`-`のみ、最大64 bytes
- Windows backend: Credential Manager Generic Credential
- WebViewへsecret型をserializeしない
- `SecretBytes`のDebug表示は常にredacted
- secret bufferはdrop時にvolatile writeとcompiler fenceで消去する
- Windows APIのunsafe codeをこのcrateへ隔離する

`PlatformCredentialStore`はstore / load / delete / containsをRust backendへ提供します。Tauri側でWebViewに公開するのはmetadata-onlyのstatus commandだけです。Relay接続処理は将来、Rust内部で`SecretBytes`を読み、WebViewを経由せずtransportへ渡します。

現段階ではWindows backendだけが実装済みです。rotation rollback、複数account lifecycle、macOS Keychain、Linux Secret Serviceは未実装です。
