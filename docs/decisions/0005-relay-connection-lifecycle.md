# 0005: Relay connection lifecycle and Desktop handle ownership

Status: Accepted

Date: 2026-08-12

## Context

製品Rust境界では、credentialをWebViewへ出さずにRelayへ接続し、接続handleのdropでofflineへ戻すところまで成立した。一方、常駐接続には明示start / stop、予期しない切断、再接続待機、二重start拒否、アプリ終了時の破棄を共通規則として持つ必要がある。

production transport、接続先選択、pairing UXはまだ未決定である。これらを先に固定せず、HTTP、WebSocket、MCPなどから独立したlifecycle契約だけを実装する。

## Decision

- `moe-relay-client`に、thread、clock、network I/Oを持たない決定的な`RelayLifecycle`状態機械を置く。
- phaseは`offline / connecting / connected / retry waiting / stopping / error`とする。
- startはofflineまたはerrorからだけ許可し、それ以外では二重startとして拒否する。
- stopは接続中ならclose、retry待機中ならtimer cancelを要求する。close完了後にofflineへ遷移する。
- retry delayは1秒、2秒、5秒、10秒、30秒の5回までとし、それ以降はerrorで停止する。成功時、手動stop時、新しい明示start時にretry metadataをresetする。
- 自動retryはRelay unavailableとprotocol failureだけを一時的な接続障害として扱う。credential不足、secure storage障害、認証拒否、cancelは自動retryしない。
- runtimeが外部へ返せる値はphase、retry回数、次の待機時間、固定error codeだけとし、credential、OS error、target、server messageを含めない。
- Desktop backendはaccountごとの接続handleを型消去して所有し、二重所有を拒否する。明示stop / shutdownおよびownerのdropでhandleを破棄する。
- この段階ではstart / stop commandをWebViewへ公開せず、実timer、network thread、production transportも実装しない。

## Consequences

- transport実装は状態機械が返すstart、close、schedule、cancel actionを非同期runtimeへ写像できる。
- clockを使わないcontract testで、retry上限、手動stop、認証拒否、予期しない切断、順序違反を安定して検証できる。
- Desktopの通常終了ではmanaged ownerのdropが最後のhandle破棄境界になる。process強制終了やOS crashで非同期cleanup完了は保証しない。
- production transport選定後、timer / cancellationと接続taskをownerへ結び、pairing UXと接続先設定が決まってからstart / stop commandを追加する必要がある。
