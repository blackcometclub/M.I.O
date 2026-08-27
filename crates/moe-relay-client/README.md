# moe-relay-client

M.O.E. Desktop backend専用のRelay pairing・接続境界です。

- pairing responseのdevice credentialをWebViewへ返さずcredential storeへ保存する
- 短時間pairing codeをredacted・消去対象の型に閉じ、Rust pairing transportへ一度だけ渡す
- account IDからcredential targetを直接組み立てず`RelayCredentialId`へ委譲する
- pairing対象とresponseのdevice IDが一致しない場合は保存しない
- 接続時だけcredentialを読み出し、借用値としてRust transportへ渡す
- credential未保存、保存失敗、接続失敗、再pairing、削除を区別する
- accountごとのoffline / connecting / connected / errorを管理する
- managed connectionのdropでofflineへ戻し、同一accountの二重接続を拒否する
- transport非依存のstart / stop / unexpected disconnect状態機械を持つ
- retryを1秒、2秒、5秒、10秒、30秒の5回に制限し、手動stopで待機をcancelする
- credential不足、secure storage障害、認証拒否、cancelを自動retryしない
- OS errorをWebViewへ出さず、安全なmetadata error codeへ分類する
- Desktop runtime生成失敗も固定`runtimeUnavailable`へ分類し、再start可能なterminal errorとする
- secretを含む型はSerialize / Cloneを実装せず、Debug表示をredactする

現段階では実HTTP / HTTPS / WSS transport、timer、network threadをこのcrateへ提供しません。pairing・connection transport trait、lifecycle service、決定的なretry状態機械、fakeを使ったcontract testを固定し、Desktop側orchestratorが実taskへ写像します。公開Relay接続や操作UIは後続trancheで追加します。
