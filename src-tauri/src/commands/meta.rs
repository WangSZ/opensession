use std::collections::HashMap;

use crate::models::*;

fn meta_path() -> std::path::PathBuf {
    crate::config::app_config_dir().join("directory_meta.json")
}

fn load_meta() -> HashMap<String, DirectoryMeta> {
    let path = meta_path();
    if !path.exists() {
        return HashMap::new();
    }
    let meta = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    normalize_meta_keys(meta)
}

fn normalize_meta_keys(meta: HashMap<String, DirectoryMeta>) -> HashMap<String, DirectoryMeta> {
    let mut normalized = HashMap::<String, DirectoryMeta>::new();
    for (path, entry) in meta {
        let key = crate::session_repository::normalize_directory_path(&path);
        let target = normalized.entry(key).or_default();
        for tag in entry.tags {
            if !target.tags.contains(&tag) {
                target.tags.push(tag);
            }
        }
        target.pinned |= entry.pinned;
        target.hidden |= entry.hidden;
    }
    normalized
}

fn save_meta(meta: &HashMap<String, DirectoryMeta>) -> Result<(), String> {
    let path = meta_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create meta dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize meta: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write meta: {}", e))
}

pub fn mark_hidden(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let mut meta = load_meta();
    for p in paths {
        let path = crate::session_repository::normalize_directory_path(p);
        let entry = meta.entry(path).or_default();
        entry.hidden = true;
    }
    let _ = save_meta(&meta);
}

pub fn merge_into(groups: &mut Vec<DirectoryGroup>) {
    let meta = load_meta();
    for g in groups.iter_mut() {
        let entry = meta.get(&g.path).cloned().unwrap_or_default();
        g.tags = entry.tags;
        g.pinned = entry.pinned;
        g.hidden = entry.hidden;
    }
}

#[tauri::command]
pub fn set_directory_tags(path: String, tags: Vec<String>) -> Result<(), String> {
    let mut meta = load_meta();
    let path = crate::session_repository::normalize_directory_path(&path);
    let entry = meta.entry(path).or_default();
    entry.tags = tags;
    save_meta(&meta)
}

#[tauri::command]
pub fn delete_tag_global(tag: String) -> Result<(), String> {
    let mut meta = load_meta();
    for entry in meta.values_mut() {
        entry.tags.retain(|t| t != &tag);
    }
    save_meta(&meta)
}

#[tauri::command]
pub fn toggle_directory_pin(path: String) -> Result<bool, String> {
    let mut meta = load_meta();
    let path = crate::session_repository::normalize_directory_path(&path);
    let entry = meta.entry(path).or_default();
    entry.pinned = !entry.pinned;
    let new_state = entry.pinned;
    save_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn toggle_directory_hidden(path: String) -> Result<bool, String> {
    let mut meta = load_meta();
    let path = crate::session_repository::normalize_directory_path(&path);
    let entry = meta.entry(path).or_default();
    entry.hidden = !entry.hidden;
    let new_state = entry.hidden;
    save_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn rename_directory_meta(old: String, new: String) -> Result<(), String> {
    let mut meta = load_meta();
    let old = crate::session_repository::normalize_directory_path(&old);
    let new = crate::session_repository::normalize_directory_path(&new);
    let entry = meta.remove(&old).unwrap_or_default();
    meta.insert(
        new,
        DirectoryMeta {
            hidden: false,
            ..entry
        },
    );
    save_meta(&meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_for_equivalent_paths_is_merged() {
        let root = std::env::temp_dir().join(format!("opensession-meta-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let mut meta = HashMap::new();
        meta.insert(
            root.to_string_lossy().to_string(),
            DirectoryMeta {
                tags: vec!["one".into()],
                pinned: true,
                hidden: false,
            },
        );
        meta.insert(
            root.join(".").to_string_lossy().to_string(),
            DirectoryMeta {
                tags: vec!["two".into()],
                pinned: false,
                hidden: true,
            },
        );

        let normalized = normalize_meta_keys(meta);
        let key = std::fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let entry = normalized.get(&key).unwrap();

        assert_eq!(normalized.len(), 1);
        assert_eq!(entry.tags.len(), 2);
        assert!(entry.tags.contains(&"one".to_string()));
        assert!(entry.tags.contains(&"two".to_string()));
        assert!(entry.pinned);
        assert!(entry.hidden);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[tauri::command]
pub fn open_in_file_manager(directory: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Finder", &directory])
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(path.clone());
        std::process::Command::new("xdg-open")
            .arg(&parent)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_jetbrains(directory: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "IntelliJ IDEA", &directory])
            .spawn()
            .map_err(|e| format!("Failed to open IntelliJ IDEA: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("idea")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open IntelliJ IDEA: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("idea")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open IntelliJ IDEA: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_vscode(directory: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Visual Studio Code", &directory])
            .spawn()
            .map_err(|e| format!("Failed to open VS Code: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("code")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open VS Code: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("code")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open VS Code: {}", e))?;
    }

    Ok(())
}
