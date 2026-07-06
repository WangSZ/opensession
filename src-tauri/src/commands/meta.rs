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
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_meta(meta: &HashMap<String, DirectoryMeta>) -> Result<(), String> {
    let path = meta_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create meta dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize meta: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write meta: {}", e))
}

pub fn mark_hidden(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let mut meta = load_meta();
    for p in paths {
        let entry = meta.entry(p.clone()).or_default();
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
    let entry = meta.entry(path).or_default();
    entry.pinned = !entry.pinned;
    let new_state = entry.pinned;
    save_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn toggle_directory_hidden(path: String) -> Result<bool, String> {
    let mut meta = load_meta();
    let entry = meta.entry(path).or_default();
    entry.hidden = !entry.hidden;
    let new_state = entry.hidden;
    save_meta(&meta)?;
    Ok(new_state)
}

#[tauri::command]
pub fn rename_directory_meta(old: String, new: String) -> Result<(), String> {
    let mut meta = load_meta();
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
