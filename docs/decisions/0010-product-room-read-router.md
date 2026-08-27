# 0010: Product Room read router

Status: Accepted

Date: 2026-08-12

## Context

ADR 0009でDesktopがWindows Credential Managerのdevice credentialを使い、production HTTPS transportを自動所有できるようになった。ただし `hello_ack` 後のframeはすべてprotocol errorとして切断しており、RelayからRoomを読む製品経路は存在しなかった。

先行PoCにはRoom snapshotのschemaとcursor読取がある一方、現在のReact画面は `useDemoRooms` のローカルstateで動いている。Rust backendからReact stateを直接読むことはできないため、通信契約と製品Room sourceを先に固定し、UIとの単一state化は別trancheに分離する。

## Decision

- `moe-protocol` に、strictなRelay request envelope、`moe_read_room` params、success / error responseを置く。未知field、不正request ID、不正Room ID、1から30の範囲外のlimitを拒否する。
- 1接続につき受理するrequest IDを256件まで保持する。同じIDの再利用は `duplicate_request`、未知methodは `unsupported_method`、不正paramsは `invalid_request` として、同じrequest IDを持つresponseを返す。
- `moe-core` にprovider-neutralなRoom、participant、message、read query / result、`RoomSource` traitを置く。messageは宛先、生成日時、Artifact IDを持ち、source構築時に参照整合性と上限を検証する。
- Desktopは起動時に不変のbackend Room snapshotを構築する。当面のsnapshotはReactデモと同じ参加者・初期3 messageを持つが、React stateそのものではなく、送信後のUI変更も反映しない。
- HTTPS taskは `hello_ack` 後にbounded frameを順次読み、検証、routing、response書込を行う。stopが観測された後はresponseを書かない。
- Room readは、cursorなしなら末尾からlimit件、cursorありならその次からlimit件を返す。Roomまたはcursorが無い場合はprotocol切断ではなく、provider-neutralなnot-found結果を返す。
- responseも既存transportの8 KiB frame上限に従う。大きなmessage集合のbyte-budget pagingはこのADRでは導入しない。

## Consequences

- Windows Credential Manager → production HTTPS → Desktop runtime → Room source → 相関済みresponseの製品経路が成立する。
- 生成CAのlocalhost TLS fixtureで、切断後の自動再接続と、2世代目の実Room responseを同時に検証できる。実Windows credentialを使う隔離testでも同じ経路を確認する。
- 未知method、不正params、重複requestは安全な固定errorへ変換され、未検証payloadをRoom sourceやWebViewへ渡さない。
- 現在のRoom sourceは読み取り専用のbootstrap snapshotである。React UIとの単一state化、message write、永続化、Artifact、byte-budget paging / backpressure、公開Relayは未完である。

## Follow-up to ADR 0009

ADR 0009の「確立後frameをprotocol errorにする」「Room request routerは未完」という暫定判断は、本ADRの範囲で置き換える。build-time設定、credential境界、TLS trust、cancel、bounded retryの判断はそのまま維持する。

## Follow-up

ADR 0011で、初期開発室の読取をbounded Tauri commandからReact UIへ結合した。message writeと永続化は引き続き未完である。
