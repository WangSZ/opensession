use crate::db;
use crate::models::*;

use super::git_info;
use super::meta;

#[tauri::command]
pub async fn list_directories(app: tauri::AppHandle) -> Result<Vec<DirectoryGroup>, String> {
    let mut groups = db::list_directory_groups()?;
    meta::merge_into(&mut groups);
    let stale = git_info::apply_cached(&mut groups);
    git_info::auto_hide_missing_worktrees(&mut groups);
    if !stale.is_empty() {
        git_info::spawn_refresh(app, stale);
    }
    Ok(groups)
}

#[tauri::command]
pub async fn search_directories(
    _app: tauri::AppHandle,
    query: String,
) -> Result<Vec<DirectoryGroup>, String> {
    let mut groups = db::list_directory_groups()?;
    meta::merge_into(&mut groups);
    git_info::apply_cached(&mut groups);
    git_info::auto_hide_missing_worktrees(&mut groups);
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
    let mut sessions = db::get_sessions(&directory)?;
    super::session_meta::merge_into_sessions(&mut sessions);
    Ok(sessions)
}

#[tauri::command]
pub fn get_session_detail(session_id: String) -> Result<SessionDetail, String> {
    db::get_session_detail(&session_id)
}
