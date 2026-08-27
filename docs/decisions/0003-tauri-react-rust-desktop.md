# 0003: Tauri 2, React, TypeScript and Rust desktop stack

Status: Accepted

Date: 2026-08-11

## Context

M.O.E.は、カスタマイズ可能なDesktop UIに加え、複数のlocal agent、CLI、MCP、API、長時間Job、approval、Artifactを安全に監督する必要があります。

## Decision

Desktop applicationにはTauri 2を採用します。

- UI: React、TypeScript、Vite
- Desktop shell / privileged backend: Tauri 2、Rust
- Core / Protocol / Adapter SDK: Rust workspace crates
- UIとの境界: 明示的なTauri commandとevent
- Phase 0の捨てられる接続PoC: 必要に応じてTypeScript / Node.jsも使用可能

## Consequences

- WebViewへ任意shell、filesystem、secretアクセスを公開しません。
- local processの起動、監視、停止、再接続はRust backendが担当します。
- Tauri capabilityは最小権限から追加します。
- Provider固有型をReact UIへ直接渡しません。
- ElectronはTauriで解決できない重大な接続・配布上の問題が実証された場合のfallbackとします。
- Rust toolchainとWindows C++ Build Toolsが開発要件になります。
