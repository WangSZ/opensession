use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime};

use crate::models::*;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

/// Minimum interval between actual git refreshes of the same directory.
/// Stale detection is .git-mtime based, so this only kicks in while a repo
/// flaps (branch switching etc.) and prevents refresh/event storms.
const REFRESH_COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct GitInfoEntry {
    repo_name: Option<String>,
    branch_name: Option<String>,
    git_mtime: Option<i64>,
}

#[derive(Default)]
struct GitInfoState {
    /// Lazily hydrated from disk on first access; memory is authoritative
    /// afterwards, disk writes are only persistence.
    loaded: bool,
    entries: HashMap<String, GitInfoEntry>,
    /// When the last actual refresh started per directory (memory only).
    /// Doubles as in-flight marker since it is set before spawning.
    last_refresh: HashMap<String, Instant>,
}

fn state() -> &'static RwLock<GitInfoState> {
    static STATE: OnceLock<RwLock<GitInfoState>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(GitInfoState::default()))
}

fn cache_path() -> std::path::PathBuf {
    crate::config::app_data_dir().join("git_info.json")
}

fn ensure_loaded(st: &mut GitInfoState) {
    if !st.loaded {
        st.entries = read_entries_from_disk();
        st.loaded = true;
    }
}

fn load() -> HashMap<String, GitInfoEntry> {
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut st);
    st.entries.clone()
}

fn read_entries_from_disk() -> HashMap<String, GitInfoEntry> {
    let path = cache_path();
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_entries_to_disk(entries: &HashMap<String, GitInfoEntry>) {
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        if let Some(parent) = cache_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(cache_path(), json);
    }
}

/// Insert refreshed entries; persist only when something actually changed.
/// Returns whether any entry differs from what was cached before.
fn store_entries(updates: Vec<(String, GitInfoEntry)>) -> bool {
    if updates.is_empty() {
        return false;
    }
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut st);
    let mut changed = false;
    for (path, entry) in updates {
        if st.entries.get(&path) != Some(&entry) {
            st.entries.insert(path, entry);
            changed = true;
        }
    }
    if changed {
        write_entries_to_disk(&st.entries);
    }
    changed
}

fn remove_entries(paths: &[String]) {
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut st);
    let mut removed = false;
    for path in paths {
        removed |= st.entries.remove(path).is_some();
        st.last_refresh.remove(path);
    }
    if removed {
        write_entries_to_disk(&st.entries);
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

/// 同步快路径：读内存缓存（仅一次磁盘 IO）+ stat .git → 填充 is_git_repo / is_worktree / repo_name / branch_name
/// 同时自动隐藏已删除的 worktree（missing 目录在缓存中有记录则视为失效 worktree）
/// 返回需要后台修正的目录路径列表（cache miss 的 worktree）
pub fn apply_cached_and_hide(groups: &mut Vec<DirectoryGroup>) -> Vec<String> {
    let cache = load();

    // 空缓存快速分支：设置 git 信息 + 收集所有 worktree 为 stale
    if cache.is_empty() {
        let mut stale = Vec::new();
        for g in groups.iter_mut() {
            let git_path = std::path::Path::new(&g.path).join(".git");
            g.is_git_repo = git_path.exists();
            if g.is_git_repo && git_path.is_file() {
                g.is_worktree = true;
                let (repo, branch) = infer_from_path(&g.path);
                g.repo_name = repo;
                g.branch_name = branch;
                stale.push(g.path.clone());
            }
        }
        return stale;
    }

    let mut stale = Vec::new();
    let mut to_hide = Vec::new();

    for g in groups.iter_mut() {
        let git_path = std::path::Path::new(&g.path).join(".git");
        if !git_path.exists() {
            g.is_git_repo = false;
            // missing 目录在缓存中有记录 → 失效 worktree，自动隐藏
            if g.is_missing && cache.contains_key(&g.path) {
                g.hidden = true;
                to_hide.push(g.path.clone());
            }
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

    if !to_hide.is_empty() {
        remove_entries(&to_hide);
        super::meta::mark_hidden(&to_hide);
    }

    stale
}

/// 从缓存中移除指定路径的条目
pub fn clear_cache_entry(path: &str) {
    remove_entries(std::slice::from_ref(&path.to_string()));
}

/// 后台异步修正：并发 fork git 拿准 repo_name / branch_name → 更新缓存 → 有实际变化才发事件
/// 同一目录在冷却期内的重复刷新会被跳过，避免 mtime 抖动引发的刷新风暴。
pub fn spawn_refresh(app: tauri::AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }

    // 冷却期过滤 + 在途去重：标记发生在锁内、spawn 之前，
    // 并发调用方不会对同一仓库发起重复刷新。
    let due = {
        let mut st = state().write().unwrap_or_else(|e| e.into_inner());
        ensure_loaded(&mut st);
        let now = Instant::now();
        let mut due = Vec::with_capacity(paths.len());
        for path in paths {
            let allowed = match st.last_refresh.get(&path) {
                Some(at) => now.duration_since(*at) >= REFRESH_COOLDOWN,
                None => true,
            };
            if allowed {
                st.last_refresh.insert(path.clone(), now);
                due.push(path);
            }
        }
        due
    };

    if due.is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let handles: Vec<_> = due
            .into_iter()
            .map(|path| tokio::spawn(refresh_entry(path)))
            .collect();

        let mut updates = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(update) = handle.await {
                updates.push(update);
            }
        }

        if store_entries(updates) {
            let _ = app.emit("git_info://updated", serde_json::json!({}));
        }
    });
}

async fn refresh_entry(path: String) -> (String, GitInfoEntry) {
    let mtime = git_mtime(&std::path::Path::new(&path).join(".git"));
    let (fallback_repo, fallback_branch) = infer_from_path(&path);

    // 两个探测相互独立，单仓库内也并发执行。
    let (main_repo_name, current_branch) =
        tokio::join!(get_main_repo_name(&path), get_current_branch(&path));

    (
        path,
        GitInfoEntry {
            repo_name: main_repo_name.or(fallback_repo),
            branch_name: current_branch.or(fallback_branch),
            git_mtime: mtime,
        },
    )
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
