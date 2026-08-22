use crate::models::*;
use crate::session_sources::SessionSource;

use super::git_info;
use super::meta;

/// Shared pipeline: cached source groups + fresh meta/git overlay.
/// `apply_cached_and_hide` must see missing groups (it auto-hides dead
/// worktrees), so filtering happens after it runs.
fn build_directory_list(
    app: &tauri::AppHandle,
) -> Result<(Vec<DirectoryGroup>, SourceStatuses), String> {
    let (mut groups, statuses) =
        crate::session_repository::list_directory_groups_with_statuses()?;
    meta::merge_into(&mut groups);
    let stale = git_info::apply_cached_and_hide(&mut groups);
    if !stale.is_empty() {
        git_info::spawn_refresh(app.clone(), stale);
    }
    Ok((groups, statuses))
}

#[tauri::command]
pub async fn list_directories(
    app: tauri::AppHandle,
    include_missing: Option<bool>,
) -> Result<DirectoryListResponse, String> {
    let include_missing = include_missing.unwrap_or(false);
    let (mut groups, statuses) = build_directory_list(&app)?;
    let total_count = groups.len();
    if !include_missing {
        // Missing directories are only needed for the "show hidden" view;
        // dropping them here avoids shipping ~1MB of dead entries per reload.
        groups.retain(|g| !g.is_missing);
    }
    Ok(DirectoryListResponse {
        directories: groups,
        statuses,
        total_count,
    })
}

#[tauri::command]
pub fn get_source_statuses() -> SourceStatuses {
    crate::session_repository::get_source_statuses()
}

#[tauri::command]
pub async fn search_directories(
    _app: tauri::AppHandle,
    query: String,
) -> Result<Vec<DirectoryGroup>, String> {
    let (groups, _) = build_directory_list(&_app)?;
    let query_lower = query.to_lowercase();
    Ok(groups
        .into_iter()
        .filter(|g| {
            g.path.to_lowercase().contains(&query_lower)
                || g.name.to_lowercase().contains(&query_lower)
        })
        .collect())
}

#[tauri::command]
pub fn get_sessions(directory: String) -> Result<Vec<Session>, String> {
    let mut sessions = crate::session_repository::get_sessions(&directory)?;
    super::session_meta::merge_into_sessions(&mut sessions);
    Ok(sessions)
}

#[tauri::command]
pub fn get_session_detail(
    session_source: SessionSource,
    session_id: String,
) -> Result<SessionDetail, String> {
    crate::session_repository::get_session_detail(session_source, &session_id)
}
