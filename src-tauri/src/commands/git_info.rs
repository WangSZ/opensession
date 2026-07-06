use std::collections::HashMap;
use std::time::SystemTime;

use crate::models::*;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GitInfoEntry {
    repo_name: Option<String>,
    branch_name: Option<String>,
    git_mtime: Option<i64>,
}

fn cache_path() -> std::path::PathBuf {
    crate::config::app_data_dir().join("git_info.json")
}

fn load() -> HashMap<String, GitInfoEntry> {
    let path = cache_path();
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(cache: &HashMap<String, GitInfoEntry>) {
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        if let Some(parent) = cache_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(cache_path(), json);
    }
}

fn git_mtime(path: &std::path::Path) -> Option<i64> {
    if !path.exists() {
        return None;
    }
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

fn infer_from_path(path: &str) -> (Option<String>, Option<String>) {
    let p = std::path::Path::new(path);
    let branch = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let repo = p
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|folder| {
            let folder = folder.to_string_lossy();
            folder.strip_suffix("-worktrees").map(|r| r.to_string())
        });
    (repo, branch)
}

/// 同步快路径：读缓存 + stat .git → 填充 is_git_repo / is_worktree / repo_name / branch_name
/// 返回需要后台修正的目录路径列表（cache miss 的 worktree）
pub fn apply_cached(groups: &mut Vec<DirectoryGroup>) -> Vec<String> {
    let cache = load();
    let mut stale = Vec::new();

    for g in groups.iter_mut() {
        let git_path = std::path::Path::new(&g.path).join(".git");
        if !git_path.exists() {
            g.is_git_repo = false;
            continue;
        }
        g.is_git_repo = true;

        if !git_path.is_file() {
            continue;
        }
        g.is_worktree = true;

        let cur_mtime = git_mtime(&git_path);
        if let Some(entry) = cache.get(&g.path) {
            if entry.git_mtime == cur_mtime {
                g.repo_name = entry.repo_name.clone();
                g.branch_name = entry.branch_name.clone();
                continue;
            }
        }

        let (repo, branch) = infer_from_path(&g.path);
        g.repo_name = repo;
        g.branch_name = branch;
        stale.push(g.path.clone());
    }

    stale
}

/// 自动隐藏已删除的 worktree 目录：missng 目录在 git_info 缓存中有记录则视为失效 worktree
pub fn auto_hide_missing_worktrees(groups: &mut Vec<DirectoryGroup>) {
    let cache = load();
    if cache.is_empty() {
        return;
    }

    let mut to_hide = Vec::new();

    for g in groups.iter_mut() {
        if !g.is_missing {
            continue;
        }
        if cache.contains_key(&g.path) {
            g.hidden = true;
            to_hide.push(g.path.clone());
        }
    }

    if to_hide.is_empty() {
        return;
    }

    let mut cache = cache;
    for p in &to_hide {
        cache.remove(p);
    }
    save(&cache);
    super::meta::mark_hidden(&to_hide);
}

/// 从缓存中移除指定路径的条目
pub fn clear_cache_entry(path: &str) {
    let mut cache = load();
    if cache.remove(path).is_some() {
        save(&cache);
    }
}

/// 后台异步修正：fork git 拿准 repo_name / branch_name → 更新缓存 → 发事件
pub fn spawn_refresh(app: tauri::AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let mut entries = Vec::new();

        for path in &paths {
            let mtime = git_mtime(&std::path::Path::new(path).join(".git"));
            let (fallback_repo, fallback_branch) = infer_from_path(path);
            let repo_name = get_main_repo_name(path).await.or(fallback_repo);
            let branch_name = get_current_branch(path).await.or(fallback_branch);

            let entry = GitInfoEntry {
                repo_name,
                branch_name,
                git_mtime: mtime,
            };
            entries.push((path.clone(), entry));
        }

        let mut cache = load();
        for (path, entry) in &entries {
            cache.insert(path.clone(), entry.clone());
        }
        save(&cache);

        let _ = app.emit("git_info://updated", serde_json::json!({}));
    });
}

async fn get_main_repo_name(path: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", path, "rev-parse", "--git-common-dir"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let common_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = std::path::Path::new(&common_dir);
    if !path.is_absolute() {
        return None;
    }
    Some(path.parent()?.file_name()?.to_string_lossy().to_string())
}

async fn get_current_branch(path: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", path, "branch", "--show-current"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}
