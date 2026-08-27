# Codex App Server schema manifest

## codex-cli 0.145.0

- Generated: 2026-08-11
- Source: installed standalone `@openai/codex@0.145.0`
- TypeScript command: `codex app-server generate-ts --out <typescript-output>`
- JSON Schema command: `codex app-server generate-json-schema --out <json-output>`
- Experimental schema: excluded
- TypeScript output: 617 files / 347,220 bytes
- JSON Schema output: 273 files / 2,824,133 bytes
- `thread/*` と `turn/*` の現行型はTypeScript出力の `v2/` 配下に生成される。
- `codex_app_server_protocol.schemas.json` SHA-256: `7B0AB1679BE705644A1B6C3F486F15E2F90E5F900486D90073B1DC5F0CF4C62C`
- `codex_app_server_protocol.v2.schemas.json` SHA-256: `0F647A4BD25712E824A393A3D48A41857599059C77D689332DCE3F4C16E861F9`

生成物は `generated/0.145.0/` にローカル保存し、Git管理から除外しています。CLI更新時は別のversion directoryへ再生成し、このmanifestへ差分を追記します。
