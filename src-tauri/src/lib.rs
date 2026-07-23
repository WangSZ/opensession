pub mod commands;
pub mod config;
pub mod db;
pub mod models;
pub mod plugins;

use plugins::mac_rounded_corners;
use tauri::Manager;

pub fn run() {
    config::migrate_summaries_if_needed();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = mac_rounded_corners::enable_modern_window_style(
                    app.handle().clone(),
                    window,
                    Some(10.0),
                    None,
                    None,
                );
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::issues::create_issue,
            commands::issues::update_issue,
            commands::issues::delete_issue,
            commands::issues::link_session_to_issue,
            commands::issues::unlink_session_from_issue,
            commands::issues::get_session_issue,
            commands::issues::get_all_issues,
            commands::issues::add_comment,
            commands::issues::delete_comment,
            commands::issues::get_issue_comments,
            commands::issues::link_directory_to_issue,
            commands::issues::unlink_directory_from_issue,
            commands::issues::get_directory_issue,
            commands::sessions::list_directories,
            commands::sessions::get_sessions,
            commands::sessions::get_session_detail,
            commands::sessions::search_directories,
            commands::summary::generate_summary,
            commands::summary::get_cached_summary,
            commands::summary::set_cached_summary,
            commands::summary::delete_cached_summary,
            commands::terminal::open_in_terminal,
            commands::terminal::open_plain_terminal,
            commands::terminal::fork_session,
            commands::terminal::bootstrap_session,
            commands::meta::set_directory_tags,
            commands::meta::delete_tag_global,
            commands::meta::toggle_directory_pin,
            commands::meta::toggle_directory_hidden,
            commands::meta::rename_directory_meta,
            commands::meta::open_in_file_manager,
            commands::meta::open_in_vscode,
            commands::meta::open_in_jetbrains,
            commands::session_meta::toggle_session_hidden,
            commands::session_meta::toggle_session_pin,
            commands::session_meta::set_session_note,
            commands::terminal::open_worktree,
            commands::terminal::list_git_bases,
            commands::terminal::remove_worktree,
            mac_rounded_corners::enable_rounded_corners,
            mac_rounded_corners::enable_modern_window_style,
            mac_rounded_corners::reposition_traffic_lights,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
