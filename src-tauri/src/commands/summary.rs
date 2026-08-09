use std::collections::HashMap;

use crate::models::*;
use serde_json::json;
use tauri::Emitter;

#[tauri::command]
pub async fn generate_summary(app: tauri::AppHandle, directory: String) -> Result<(), String> {
    let directory = crate::session_repository::normalize_directory_path(&directory);
    let temp_dir = std::env::temp_dir().join("opensession-opencode");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let agents_md_path = std::path::Path::new(&directory).join("AGENTS.md");
    let agents_md = if agents_md_path.exists() {
        std::fs::read_to_string(&agents_md_path)
            .ok()
            .map(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.chars().count() > 500 {
                    trimmed.chars().take(500).collect()
                } else {
                    trimmed
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let titles = crate::session_repository::get_recent_session_titles(&directory, 5)
        .unwrap_or_default()
        .join(", ");

    let prompt = format!(
        "请用一句话（15字以内）概括以下代码项目：\n目录：{}\nAGENTS.md：{}\n最近会话：{}\n返回格式：第一行是标题，第二行是详细描述，不要加其他说明。",
        directory, agents_md, titles
    );

    tauri::async_runtime::spawn(async move {
        let opencode_available = tokio::process::Command::new("opencode")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false);

        let (engine, output) = if opencode_available {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let escaped_dir = directory.replace('\'', "'\\''");
            let escaped_prompt = prompt.replace('\'', "'\\''");
            let shell_cmd = format!("opencode run --dir '{}' '{}'", escaped_dir, escaped_prompt);
            (
                "opencode",
                tokio::process::Command::new(&shell)
                    .args(["-lc", &shell_cmd])
                    .current_dir(&temp_dir)
                    .output()
                    .await,
            )
        } else {
            (
                "codex",
                tokio::process::Command::new("codex")
                    .args([
                        "exec",
                        "--ephemeral",
                        "--skip-git-repo-check",
                        "--sandbox",
                        "read-only",
                        "--cd",
                        &directory,
                        &prompt,
                    ])
                    .current_dir(&temp_dir)
                    .output()
                    .await,
            )
        };

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let lines: Vec<&str> = stdout.trim().lines().collect();
                let title = lines.first().unwrap_or(&"").to_string();
                let description = if lines.len() > 1 {
                    lines[1..].join("\n")
                } else {
                    String::new()
                };
                let generated_at = chrono::Utc::now().to_rfc3339();
                let summary = Summary {
                    title,
                    description,
                    generated_at,
                };

                let data_dir = crate::config::app_data_dir();
                if let Ok(()) = std::fs::create_dir_all(&data_dir) {
                    let cache_path = data_dir.join("summaries.json");
                    let mut summaries: HashMap<String, Summary> = if cache_path.exists() {
                        std::fs::read_to_string(&cache_path)
                            .ok()
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .map(crate::session_repository::normalize_summary_keys)
                            .unwrap_or_default()
                    } else {
                        HashMap::new()
                    };
                    summaries.insert(directory.clone(), summary.clone());
                    if let Ok(json) = serde_json::to_string_pretty(&summaries) {
                        let _ = std::fs::write(&cache_path, json);
                    }
                }

                let _ = app.emit(
                    "summary://generated",
                    json!({ "directory": directory, "summary": summary }),
                );
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let _ = app.emit(
                    "summary://generated",
                    json!({ "directory": directory, "error": format!("{} failed: {}", engine, stderr) }),
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "summary://generated",
                    json!({ "directory": directory, "error": format!("Failed to execute {}: {}", engine, e) }),
                );
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn set_cached_summary(directory: String, summary: Summary) -> Result<(), String> {
    let directory = crate::session_repository::normalize_directory_path(&directory);
    let data_dir = crate::config::app_data_dir();
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    let cache_path = data_dir.join("summaries.json");
    let mut summaries: HashMap<String, Summary> = if cache_path.exists() {
        std::fs::read_to_string(&cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .map(crate::session_repository::normalize_summary_keys)
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    summaries.insert(directory, summary);
    serde_json::to_string_pretty(&summaries)
        .map_err(|e| format!("Failed to serialize: {}", e))
        .and_then(|json| {
            std::fs::write(&cache_path, json).map_err(|e| format!("Failed to write cache: {}", e))
        })
}

#[tauri::command]
pub fn delete_cached_summary(directory: String) -> Result<(), String> {
    let directory = crate::session_repository::normalize_directory_path(&directory);
    let cache_path = crate::config::app_data_dir().join("summaries.json");
    if !cache_path.exists() {
        return Ok(());
    }
    let mut summaries: HashMap<String, Summary> = std::fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read cache: {}", e))
        .and_then(|s| {
            serde_json::from_str(&s).map_err(|e| format!("Failed to parse cache: {}", e))
        })?;
    summaries = crate::session_repository::normalize_summary_keys(summaries);
    summaries.remove(&directory);
    serde_json::to_string_pretty(&summaries)
        .map_err(|e| format!("Failed to serialize: {}", e))
        .and_then(|json| {
            std::fs::write(&cache_path, json).map_err(|e| format!("Failed to write cache: {}", e))
        })
}

#[tauri::command]
pub fn get_cached_summary(directory: String) -> Result<Option<Summary>, String> {
    let directory = crate::session_repository::normalize_directory_path(&directory);
    let cache_path = crate::config::app_data_dir().join("summaries.json");
    if !cache_path.exists() {
        return Ok(None);
    }
    let summaries: HashMap<String, Summary> = std::fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read cache: {}", e))
        .and_then(|s| {
            serde_json::from_str(&s).map_err(|e| format!("Failed to parse cache: {}", e))
        })?;
    let summaries = crate::session_repository::normalize_summary_keys(summaries);
    Ok(summaries.get(&directory).cloned())
}
