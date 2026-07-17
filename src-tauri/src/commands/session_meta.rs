use std::collections::HashMap;

use crate::models::*;

fn session_meta_path() -> std::path::PathBuf {
    crate::config::app_config_dir().join("session_meta.json")
}

fn load_session_meta() -> HashMap<String, SessionMeta> {
    let path = session_meta_path();
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_session_meta(meta: &HashMap<String, SessionMeta>) -> Result<(), String> {
    let path = session_meta_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create session meta dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize session meta: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write session meta: {}", e))
}

pub fn merge_into_sessions(sessions: &mut Vec<Session>) {
    let meta = load_session_meta();
    for s in sessions.iter_mut() {
        if let Some(entry) = meta.get(&s.id) {
            s.hidden = entry.hidden;
            s.pinned = entry.pinned;
            s.note = entry.note.clone();
        }
    }
}

#[tauri::command]
pub fn toggle_session_hidden(session_id: String) -> Result<bool, String> {
    let mut meta = load_session_meta();
    let entry = meta.entry(session_id).or_default();
    entry.hidden = !entry.hidden;
    let new_state = entry.hidden;
    save_session_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn toggle_session_pin(session_id: String) -> Result<bool, String> {
    let mut meta = load_session_meta();
    let entry = meta.entry(session_id).or_default();
    entry.pinned = !entry.pinned;
    let new_state = entry.pinned;
    save_session_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn set_session_note(session_id: String, note: String) -> Result<(), String> {
    let mut meta = load_session_meta();
    let entry = meta.entry(session_id).or_default();
    entry.note = note;
    save_session_meta(&meta)?;
    Ok(())
}
