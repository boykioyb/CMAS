mod commands;
mod models;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Migrate keychain entries from old account name ("claude-code")
            // to the correct OS username, matching how the Claude Code extension
            // reads credentials.
            services::keychain::migrate_keychain_account_name();

            // Sync the CLI-refreshed active credentials to the active account's
            // backup so token health checks don't fail after restart.
            if let Some(active) = commands::account::load_accounts()
                .into_iter()
                .find(|a| a.is_active)
            {
                services::keychain::sync_active_credentials_to_backup(&active.id);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Account commands
            commands::account::list_accounts,
            commands::account::refresh_all_usage,
            commands::account::add_current_account,
            commands::account::update_account,
            commands::account::remove_account,
            commands::account::get_active_account,
            commands::account::detect_current_account,
            commands::account::add_project_to_account,
            commands::account::remove_project_from_account,
            commands::account::set_selected_project,
            // Auth commands
            commands::auth::auth_backup_current,
            commands::auth::auth_start_login,
            commands::auth::auth_check_login_status,
            commands::auth::auth_confirm_new_account,
            commands::auth::auth_restore_original,
            commands::auth::auth_get_status,
            // Switch commands
            commands::switch::switch_account,
            commands::switch::switch_and_open_vscode,
            commands::switch::switch_to_best_account,
            // Quota/Usage commands
            commands::quota::get_usage_info,
            commands::quota::get_daily_activity,
            commands::quota::get_quota_summary,
            commands::quota::check_account_token,
            // Config commands
            commands::config::get_app_config,
            commands::config::save_app_config,
            commands::config::find_vscode,
            commands::config::find_claude_cli,
            // Usage API commands
            commands::usage_scraper::scrape_claude_usage,
            commands::usage_scraper::fetch_account_usage,
            commands::usage_scraper::open_claude_login,
            // Cost usage commands
            commands::cost_usage::get_cost_usage_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
