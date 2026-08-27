mod ai_connection_status;
mod ai_dispatch;
mod ai_dispatch_ledger;
mod appearance_settings;
mod browser_bridge;
mod claude_fable;
mod codex_app_server;
mod credential_commands;
mod gemini_antigravity;
mod grok_cli;
mod mcp_server;
mod participant_profiles;
mod relay_client;
mod relay_product;
pub mod relay_runtime;
mod room_ai_continuity;
mod room_backup_commands;
mod room_commands;
mod room_conductor_settings;
mod room_orchestration;
mod room_orchestration_ledger;
mod room_source;
mod room_workspace;
mod system_fonts;
mod time;

use ai_connection_status::desktop_ai_connection_status;
use ai_dispatch::{
    DesktopAiDispatcher, desktop_room_ai_continuity_reset, desktop_room_dispatch_message,
    desktop_room_dispatch_recipient, desktop_room_dispatch_unknowns,
};
use ai_dispatch_ledger::{persistent_ai_dispatch_ledger, product_ai_dispatch_ledger_file};
use appearance_settings::{
    DesktopAppearanceSettings, desktop_appearance_settings, desktop_appearance_settings_save,
    product_appearance_file,
};
use browser_bridge::{browser_bridge_experiment_enabled, start_browser_bridge};
use claude_fable::ClaudeFableAdapter;
use codex_app_server::CodexAppServerAdapter;
use credential_commands::relay_credential_status;
use gemini_antigravity::GeminiAntigravityAdapter;
use grok_cli::GrokCliAdapter;
use mcp_server::{desktop_mcp_server_status, start_product_mcp_server};
use participant_profiles::{
    DesktopParticipantProfiles, desktop_participant_profile_save, desktop_participant_profiles,
    product_profile_file,
};
use relay_client::{desktop_relay_service, relay_connection_status};
use relay_product::start_bundled_product_relay;
use relay_runtime::desktop_relay_orchestrator;
use room_ai_continuity::{persistent_room_ai_continuity, product_ai_continuity_file};
use room_backup_commands::{desktop_room_backup, desktop_room_restore_latest_backup};
use room_commands::{
    desktop_room_add_participant, desktop_room_create, desktop_room_delete, desktop_room_list,
    desktop_room_read, desktop_room_remove_participant, desktop_room_rename,
    desktop_room_write_message,
};
use room_conductor_settings::{
    desktop_room_conductor_clear, desktop_room_conductor_mode_save, desktop_room_conductor_set,
    desktop_room_conductor_status, persistent_room_conductor_settings,
    product_conductor_capabilities, product_room_conductor_settings_file,
};
use room_orchestration::{desktop_room_orchestrate_message, product_room_orchestrator};
use room_orchestration_ledger::{
    persistent_room_orchestration_ledger, product_room_orchestration_ledger_file,
};
use room_source::{prepare_persistent_desktop_room_source, product_room_file};
use room_workspace::{
    desktop_room_workspace_choose, desktop_room_workspace_clear, desktop_room_workspace_status,
    persistent_room_workspaces, product_workspace_file,
};
use std::sync::Arc;
use system_fonts::desktop_system_font_families;
use tauri::{AppHandle, Manager};

#[tauri::command]
fn bootstrap_status() -> moe_core::BootstrapStatus {
    moe_core::bootstrap_status()
}

#[cfg(desktop)]
fn restore_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let relay_service = desktop_relay_service();
    let relay_runtime = desktop_relay_orchestrator();
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(
        |app, _args, _working_directory| restore_main_window(app),
    ));
    builder
        .plugin(tauri_plugin_dialog::init())
        .manage(relay_service)
        .manage(relay_runtime)
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            let prepared_room_source =
                prepare_persistent_desktop_room_source(product_room_file(&app_data_dir))?;
            let appearance_settings =
                DesktopAppearanceSettings::load(product_appearance_file(&app_data_dir))?;
            let participant_profiles = DesktopParticipantProfiles::load_with_participant_migration(
                product_profile_file(&app_data_dir),
                prepared_room_source.owner_migration(),
            )?;
            let room_source = prepared_room_source.commit()?;
            let mcp_server = start_product_mcp_server(room_source.clone(), app.handle().clone());
            let room_workspaces =
                persistent_room_workspaces(product_workspace_file(&app_data_dir))?;
            let room_ai_continuity =
                persistent_room_ai_continuity(product_ai_continuity_file(&app_data_dir))?;
            let ai_dispatch_ledger =
                persistent_ai_dispatch_ledger(product_ai_dispatch_ledger_file(&app_data_dir))?;
            let room_conductor_settings = persistent_room_conductor_settings(
                product_room_conductor_settings_file(&app_data_dir),
            )?;
            let conductor_capabilities = product_conductor_capabilities();
            let room_orchestration_ledger = persistent_room_orchestration_ledger(
                product_room_orchestration_ledger_file(&app_data_dir),
            )?;
            let browser_bridge = start_browser_bridge(
                room_source.clone(),
                app.handle().clone(),
                browser_bridge_experiment_enabled(),
            );
            let grok_cli = Arc::new(GrokCliAdapter::product(&app_data_dir));
            let gemini_cli = Arc::new(GeminiAntigravityAdapter::product(&app_data_dir));
            let claude_fable = Arc::new(ClaudeFableAdapter::product(&app_data_dir));
            let codex = Arc::new(CodexAppServerAdapter::product());
            let gemini_cli_installed = gemini_cli.installed();
            let ai_dispatcher = Arc::new(DesktopAiDispatcher::new(
                codex.clone(),
                grok_cli.clone(),
                gemini_cli.clone(),
                claude_fable.clone(),
                gemini_cli_installed,
                room_workspaces.clone(),
                room_ai_continuity.clone(),
                browser_bridge.clone(),
                participant_profiles.clone(),
                ai_dispatch_ledger.clone(),
            ));
            let room_orchestrator = product_room_orchestrator(
                codex,
                ai_dispatcher.clone(),
                room_conductor_settings.clone(),
                conductor_capabilities.clone(),
                participant_profiles.clone(),
                room_orchestration_ledger.clone(),
            );
            let relay_service = app.state::<Arc<relay_client::DesktopRelayService>>();
            let relay_runtime = app.state::<relay_runtime::DesktopRelayOrchestrator>();
            start_bundled_product_relay(
                relay_service.inner().clone(),
                room_source.clone(),
                relay_runtime.inner(),
            )
            .expect("bundled Relay product configuration must be valid");
            app.manage(room_source);
            app.manage(mcp_server);
            app.manage(appearance_settings);
            app.manage(participant_profiles);
            app.manage(room_workspaces);
            app.manage(room_ai_continuity);
            app.manage(ai_dispatch_ledger);
            app.manage(room_conductor_settings);
            app.manage(conductor_capabilities);
            app.manage(room_orchestration_ledger);
            app.manage(room_orchestrator);
            app.manage(browser_bridge);
            app.manage(grok_cli);
            app.manage(gemini_cli);
            app.manage(claude_fable);
            app.manage(ai_dispatcher);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_status,
            desktop_appearance_settings,
            desktop_appearance_settings_save,
            desktop_ai_connection_status,
            desktop_mcp_server_status,
            desktop_room_list,
            desktop_room_read,
            desktop_room_create,
            desktop_room_add_participant,
            desktop_room_rename,
            desktop_room_remove_participant,
            desktop_room_delete,
            desktop_room_backup,
            desktop_room_restore_latest_backup,
            desktop_room_workspace_status,
            desktop_room_workspace_choose,
            desktop_room_workspace_clear,
            desktop_room_dispatch_message,
            desktop_room_dispatch_recipient,
            desktop_room_dispatch_unknowns,
            desktop_room_ai_continuity_reset,
            desktop_room_conductor_status,
            desktop_room_conductor_set,
            desktop_room_conductor_mode_save,
            desktop_room_conductor_clear,
            desktop_room_orchestrate_message,
            desktop_room_write_message,
            desktop_participant_profiles,
            desktop_participant_profile_save,
            desktop_system_font_families,
            relay_credential_status,
            relay_connection_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running M.I.O.");
}
