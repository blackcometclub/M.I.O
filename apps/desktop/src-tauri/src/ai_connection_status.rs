use crate::browser_bridge::{DesktopBrowserBridge, GEMINI_SEARCH_PARTICIPANT_ID};
use crate::claude_fable::ClaudeFableAdapter;
use crate::gemini_antigravity::GeminiAntigravityAdapter;
use crate::grok_cli::GrokCliAdapter;
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

const CODEX_PARTICIPANT_ID: &str = "codex";
const CLAUDE_CODE_PARTICIPANT_ID: &str = "claude-code";
const CLAUDE_WEB_PARTICIPANT_ID: &str = "claude-web";
const GROK_PARTICIPANT_ID: &str = "grok";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum AiConnectionState {
    Ready,
    Installed,
    SetupRequired,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiConnectionStatus {
    participant_id: &'static str,
    state: AiConnectionState,
    label: &'static str,
    detail: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiConnectionStatusSuccess {
    ok: bool,
    connections: Vec<AiConnectionStatus>,
}

fn is_explicit_launcher(value: Option<std::ffi::OsString>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

fn product_codex_available() -> bool {
    if is_explicit_launcher(env::var_os("MOE_CODEX_BIN")) {
        return true;
    }
    if env::var_os("MOE_CODEX_CLI_JS").is_some_and(|path| Path::new(&path).is_file()) {
        return true;
    }
    if let Some(app_data) = env::var_os("APPDATA") {
        let cli = PathBuf::from(app_data)
            .join("npm")
            .join("node_modules")
            .join("@openai")
            .join("codex")
            .join("bin")
            .join("codex.js");
        if cli.is_file() {
            return true;
        }
    }
    command_on_path("codex")
}

fn command_on_path(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|directory| {
        [
            name.to_owned(),
            format!("{name}.exe"),
            format!("{name}.cmd"),
        ]
        .into_iter()
        .any(|candidate| directory.join(candidate).is_file())
    })
}

#[allow(clippy::too_many_arguments)]
fn connection_statuses(
    codex_available: bool,
    claude_code_installed: bool,
    claude_live_response_seen: bool,
    grok_installed: bool,
    grok_live_response_seen: bool,
    gemini_installed: bool,
    gemini_live_response_seen: bool,
    browser_bridge_enabled: bool,
    browser_bridge_listening: bool,
    browser_extension_seen: bool,
) -> Vec<AiConnectionStatus> {
    vec![
        AiConnectionStatus {
            participant_id: CODEX_PARTICIPANT_ID,
            state: if codex_available {
                AiConnectionState::Ready
            } else {
                AiConnectionState::SetupRequired
            },
            label: if codex_available {
                "利用可能"
            } else {
                "設定が必要"
            },
            detail: if codex_available {
                "Codex App Serverを利用できます。"
            } else {
                "Codexの実行環境を検出できません。"
            },
        },
        AiConnectionStatus {
            participant_id: CLAUDE_CODE_PARTICIPANT_ID,
            state: if claude_live_response_seen {
                AiConnectionState::Ready
            } else if claude_code_installed {
                AiConnectionState::Installed
            } else {
                AiConnectionState::SetupRequired
            },
            label: if claude_live_response_seen {
                "利用可能"
            } else if claude_code_installed {
                "CLIあり・初回送信待ち"
            } else {
                "CLI設定が必要"
            },
            detail: if claude_live_response_seen {
                "Claude CodeのFable 5による会話専用応答を確認済みです。Fableへ送った内容はAnthropicで30日保持されます。"
            } else if claude_code_installed {
                "Claude Codeは検出済みです。必要時だけ非表示で起動して返答後に終了します。Fableへ送った内容はAnthropicで30日保持されます。"
            } else {
                "Claude Codeの実行環境を検出できません。"
            },
        },
        AiConnectionStatus {
            participant_id: CLAUDE_WEB_PARTICIPANT_ID,
            state: AiConnectionState::SetupRequired,
            label: "Web接続待ち",
            detail: "Claude Web用Remote MCP / Relayの接続設定が必要です。",
        },
        AiConnectionStatus {
            participant_id: GEMINI_SEARCH_PARTICIPANT_ID,
            state: if gemini_live_response_seen {
                AiConnectionState::Ready
            } else if gemini_installed {
                AiConnectionState::Installed
            } else if !browser_bridge_enabled {
                AiConnectionState::Unsupported
            } else if browser_bridge_listening && browser_extension_seen {
                AiConnectionState::Ready
            } else {
                AiConnectionState::SetupRequired
            },
            label: if gemini_live_response_seen {
                "利用可能"
            } else if gemini_installed {
                "CLIあり・初回送信待ち"
            } else if !browser_bridge_enabled {
                "実験版のみ"
            } else if browser_bridge_listening && browser_extension_seen {
                "ブラウザ接続中"
            } else if browser_bridge_listening {
                "拡張を開いてください"
            } else {
                "Bridge起動失敗"
            },
            detail: if gemini_live_response_seen {
                "Gemini Antigravity CLIの会話専用応答を確認済みです。"
            } else if gemini_installed {
                "Antigravity CLIは検出済みです。M.I.O.が必要時だけ非表示で起動し、返答後に終了します。"
            } else if !browser_bridge_enabled {
                "Google Search Gemini Browser Bridgeはお遊び用PoCとして通常版では無効です。"
            } else if browser_bridge_listening && browser_extension_seen {
                "Google Search Geminiを手動確認つきブラウザBridgeで利用できます。"
            } else if browser_bridge_listening {
                "M.I.O.拡張を有効にしてGoogle検索またはAI Modeを開いてください。"
            } else {
                "M.I.O.のローカルBrowser Bridgeを起動できませんでした。"
            },
        },
        AiConnectionStatus {
            participant_id: "chatgpt",
            state: AiConnectionState::Unsupported,
            label: "未接続",
            detail: "ChatGPT Web adapterはまだ実装されていません。",
        },
        AiConnectionStatus {
            participant_id: GROK_PARTICIPANT_ID,
            state: if grok_live_response_seen {
                AiConnectionState::Ready
            } else if grok_installed {
                AiConnectionState::Installed
            } else {
                AiConnectionState::SetupRequired
            },
            label: if grok_live_response_seen {
                "利用可能"
            } else if grok_installed {
                "CLIあり・初回送信待ち"
            } else {
                "CLI設定が必要"
            },
            detail: if grok_live_response_seen {
                "Grok CLIの会話専用応答を確認済みです。"
            } else if grok_installed {
                "Grok CLIは検出済みです。接続状態は最初の実回答で確認します。"
            } else {
                "Grok CLIの実行環境を検出できません。"
            },
        },
        AiConnectionStatus {
            participant_id: "openai-api",
            state: AiConnectionState::Unsupported,
            label: "未接続",
            detail: "OpenAI API adapterはまだ実装されていません。",
        },
        AiConnectionStatus {
            participant_id: "generic-mcp",
            state: AiConnectionState::Unsupported,
            label: "未接続",
            detail: "Generic MCP adapterはまだ実装されていません。",
        },
        AiConnectionStatus {
            participant_id: "other",
            state: AiConnectionState::Unsupported,
            label: "未接続",
            detail: "Custom adapterはまだ設定されていません。",
        },
    ]
}

#[tauri::command]
pub(crate) fn desktop_ai_connection_status(
    browser_bridge: State<'_, Arc<DesktopBrowserBridge>>,
    claude_fable: State<'_, Arc<ClaudeFableAdapter>>,
    grok_cli: State<'_, Arc<GrokCliAdapter>>,
    gemini_cli: State<'_, Arc<GeminiAntigravityAdapter>>,
) -> AiConnectionStatusSuccess {
    AiConnectionStatusSuccess {
        ok: true,
        connections: connection_statuses(
            product_codex_available(),
            claude_fable.installed(),
            claude_fable.live_response_seen(),
            grok_cli.installed(),
            grok_cli.live_response_seen(),
            gemini_cli.installed(),
            gemini_cli.live_response_seen(),
            browser_bridge.enabled(),
            browser_bridge.listening(),
            browser_bridge.extension_recently_seen(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status<'a>(statuses: &'a [AiConnectionStatus], id: &str) -> &'a AiConnectionStatus {
        statuses
            .iter()
            .find(|status| status.participant_id == id)
            .unwrap()
    }

    #[test]
    fn keeps_claude_code_and_claude_web_as_separate_connections() {
        let statuses = connection_statuses(
            true, true, false, true, false, false, false, true, true, true,
        );

        assert_eq!(status(&statuses, "codex").state, AiConnectionState::Ready);
        assert_eq!(
            status(&statuses, "claude-code").state,
            AiConnectionState::Installed
        );
        assert_eq!(
            status(&statuses, "claude-web").state,
            AiConnectionState::SetupRequired
        );
        assert_ne!(
            status(&statuses, "claude-code").detail,
            status(&statuses, "claude-web").detail
        );
    }

    #[test]
    fn never_claims_claude_code_is_connected_from_cli_detection_alone() {
        let statuses = connection_statuses(
            false, true, false, true, false, false, false, true, true, false,
        );

        assert_eq!(
            status(&statuses, "claude-code").label,
            "CLIあり・初回送信待ち"
        );
        assert_eq!(
            status(&statuses, "codex").state,
            AiConnectionState::SetupRequired
        );
    }

    #[test]
    fn claude_code_requires_a_live_fable_reply_before_reporting_ready() {
        let installed = connection_statuses(
            true, true, false, true, false, false, false, false, false, false,
        );
        assert_eq!(
            status(&installed, CLAUDE_CODE_PARTICIPANT_ID).state,
            AiConnectionState::Installed
        );

        let ready = connection_statuses(
            true, true, true, true, false, false, false, false, false, false,
        );
        assert_eq!(
            status(&ready, CLAUDE_CODE_PARTICIPANT_ID).state,
            AiConnectionState::Ready
        );
    }

    #[test]
    fn reports_gemini_ready_only_after_the_browser_extension_is_seen() {
        let waiting = connection_statuses(
            true, true, false, true, false, false, false, true, true, false,
        );
        assert_eq!(
            status(&waiting, GEMINI_SEARCH_PARTICIPANT_ID).state,
            AiConnectionState::SetupRequired
        );

        let connected = connection_statuses(
            true, true, false, true, false, false, false, true, true, true,
        );
        assert_eq!(
            status(&connected, GEMINI_SEARCH_PARTICIPANT_ID).state,
            AiConnectionState::Ready
        );
    }

    #[test]
    fn normal_product_keeps_the_browser_experiment_disabled() {
        let statuses = connection_statuses(
            true, true, false, true, false, false, false, false, false, false,
        );
        let gemini = status(&statuses, GEMINI_SEARCH_PARTICIPANT_ID);

        assert_eq!(gemini.state, AiConnectionState::Unsupported);
        assert_eq!(gemini.label, "実験版のみ");
    }

    #[test]
    fn grok_requires_a_live_reply_before_reporting_ready() {
        let installed = connection_statuses(
            true, true, false, true, false, false, false, false, false, false,
        );
        assert_eq!(
            status(&installed, GROK_PARTICIPANT_ID).state,
            AiConnectionState::Installed
        );

        let ready = connection_statuses(
            true, true, false, true, true, false, false, false, false, false,
        );
        assert_eq!(
            status(&ready, GROK_PARTICIPANT_ID).state,
            AiConnectionState::Ready
        );
    }

    #[test]
    fn antigravity_cli_takes_priority_over_the_browser_experiment() {
        let installed = connection_statuses(
            true, true, false, true, false, true, false, true, true, true,
        );
        assert_eq!(
            status(&installed, GEMINI_SEARCH_PARTICIPANT_ID).state,
            AiConnectionState::Installed
        );

        let ready = connection_statuses(
            true, true, false, true, false, true, true, false, false, false,
        );
        assert_eq!(
            status(&ready, GEMINI_SEARCH_PARTICIPANT_ID).state,
            AiConnectionState::Ready
        );
    }
}
