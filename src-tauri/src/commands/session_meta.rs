use std::collections::HashMap;

use crate::models::*;
use crate::session_sources::{SessionRef, SessionSource};

fn session_meta_path() -> std::path::PathBuf {
    crate::config::app_config_dir().join("session_meta.json")
}

fn normalize_session_meta_keys(meta: HashMap<String, SessionMeta>) -> HashMap<String, SessionMeta> {
    meta.into_iter()
        .map(|(key, value)| {
            if key.starts_with("opencode:")
                || key.starts_with("codex:")
                || key.starts_with("reasonix:")
            {
                (key, value)
            } else {
                (format!("opencode:{}", key), value)
            }
        })
        .collect()
}

fn load_session_meta() -> HashMap<String, SessionMeta> {
    let path = session_meta_path();
    if !path.exists() {
        return HashMap::new();
    }
    normalize_session_meta_keys(
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    )
}

fn save_session_meta(meta: &HashMap<String, SessionMeta>) -> Result<(), String> {
    let path = session_meta_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create session meta dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize session meta: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write session meta: {}", e))
}

pub fn merge_into_sessions(sessions: &mut Vec<Session>) {
    let meta = load_session_meta();
    for s in sessions.iter_mut() {
        let key = SessionRef::new(s.source, &s.id).storage_key();
        if let Some(entry) = meta.get(&key) {
            s.hidden = entry.hidden;
            s.pinned = entry.pinned;
            s.note = entry.note.clone();
        }
    }
}

#[tauri::command]
pub fn toggle_session_hidden(
    session_source: SessionSource,
    session_id: String,
) -> Result<bool, String> {
    let mut meta = load_session_meta();
    let key = SessionRef::new(session_source, session_id).storage_key();
    let entry = meta.entry(key).or_default();
    entry.hidden = !entry.hidden;
    let new_state = entry.hidden;
    save_session_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn toggle_session_pin(
    session_source: SessionSource,
    session_id: String,
) -> Result<bool, String> {
    let mut meta = load_session_meta();
    let key = SessionRef::new(session_source, session_id).storage_key();
    let entry = meta.entry(key).or_default();
    entry.pinned = !entry.pinned;
    let new_state = entry.pinned;
    save_session_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn set_session_note(
    session_source: SessionSource,
    session_id: String,
    note: String,
) -> Result<(), String> {
    let mut meta = load_session_meta();
    let key = SessionRef::new(session_source, session_id).storage_key();
    let entry = meta.entry(key).or_default();
    entry.note = note;
    save_session_meta(&meta)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_session_meta_keys_migrate_to_opencode_namespace() {
        let mut meta = HashMap::new();
        meta.insert(
            "legacy-id".to_string(),
            SessionMeta {
                note: "keep me".to_string(),
                ..SessionMeta::default()
            },
        );
        meta.insert("codex:019f".to_string(), SessionMeta::default());
        meta.insert("reasonix:20260817".to_string(), SessionMeta::default());

        let migrated = normalize_session_meta_keys(meta);

        assert_eq!(migrated["opencode:legacy-id"].note, "keep me");
        assert!(migrated.contains_key("codex:019f"));
        assert!(migrated.contains_key("reasonix:20260817"));
        assert!(!migrated.contains_key("legacy-id"));
    }
}
