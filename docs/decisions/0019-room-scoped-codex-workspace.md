# 0019: Room-scoped Codex workspace mode

- Status: Accepted
- Date: 2026-08-12

## Context

ADR 0013のCodex participantは安全な最初の実接続として、空の一時directory、text-only permission profile、tool禁止instructionで起動した。このため会話はできるが、M.O.E.本来の用途である「同じRoomで相談しながらローカルprojectを読み書きする」作業はできない。

Desktop全体やhome directoryを暗黙に渡すのは範囲が広すぎる。一方、手入力pathは非エンジニア利用者に不向きで、誤入力や意図しない上位directory指定も起こりやすい。

## Decision

- Room設定にCodex作業モードを追加する。Windows native folder pickerで利用者が明示選択した単一directoryだけを、そのRoomのworkspace rootとする。
- folder pickerは公式 `tauri-plugin-dialog` のRust APIを使う。選択pathはRustでcanonicalize・directory検査し、WebViewへは末尾folder名と利用可能状態だけを返す。
- workspace bindingはdevice-localな `room-workspaces-v1.json` にversioned・bounded・atomic保存する。会話snapshotや持出しbackupにはmachine固有pathを混ぜない。
- folder未選択時は従来のchat-only一時directoryとtool禁止を維持する。workspace modeでは選択rootをcwdとし、そのrootだけをread/write可能、network無効、approval policy `never` とする。範囲外permission requestは拒否する。
- Codex以外のparticipantへbindingを流用しない。Claude Code / Claude Webは各adapterで明示的に同じRoom workspace契約へ対応した時点で参加させる。
- UIは `会話のみ`、`作業可`、folder利用不能を区別し、いつでも `会話のみに戻す` 操作を提供する。

## Consequences

- CodexはM.O.E.の会話から、選択project内の調査・編集・検証を実行できる。
- PC全体、別Room、networkへの権限は暗黙に広がらない。
- folder移動・削除後はworkspaceを利用不能としてfail closedし、別directoryへ自動置換しない。
