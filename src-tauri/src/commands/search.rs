use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use unicode_normalization::UnicodeNormalization;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 10000;
const BATCH_SIZE: usize = 200;
const EVENT_NAME: &str = "search://results";

#[cfg(target_os = "macos")]
const MDFIND_BIN: &str = "/usr/bin/mdfind";
#[cfg(not(target_os = "macos"))]
const MDFIND_BIN: &str = "mdfind";

fn log_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library").join("Logs").join("opensession").join("file-search.log"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir().map(|d| d.join("opensession").join("logs").join("file-search.log"))
    }
}

fn flog(msg: &str) {
    // Off by default; set OPENSESSION_SEARCH_LOG=1 to trace searches to a file
    if std::env::var_os("OPENSESSION_SEARCH_LOG").is_none() {
        return;
    }
    let Some(path) = log_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", ts, msg);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Name,
    Content,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SearchScopeArg {
    Home,
    Global,
    Custom { id: String },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchScopeDef {
    pub id: String,
    pub name: String,
    pub directories: Vec<String>,
}

struct ActiveSearch {
    cancelled: Arc<AtomicBool>,
    children: Arc<Mutex<Vec<tokio::process::Child>>>,
}

static ACTIVE_SEARCHES: LazyLock<Mutex<HashMap<String, ActiveSearch>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn scopes_path() -> std::path::PathBuf {
    crate::config::app_config_dir().join("search_scopes.json")
}

fn load_scopes() -> Vec<SearchScopeDef> {
    let path = scopes_path();
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_scopes(scopes: &[SearchScopeDef]) -> Result<(), String> {
    let path = scopes_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(scopes)
        .map_err(|e| format!("Failed to serialize scopes: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write scopes: {}", e))
}

#[tauri::command]
pub fn list_search_scopes() -> Result<Vec<SearchScopeDef>, String> {
    Ok(load_scopes())
}

#[tauri::command]
pub fn add_search_scope(name: String, directories: Vec<String>) -> Result<SearchScopeDef, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Scope name is required".to_string());
    }
    if directories.is_empty() {
        return Err("At least one directory is required".to_string());
    }
    let mut scopes = load_scopes();
    let scope = SearchScopeDef {
        id: format!(
            "scope-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ),
        name,
        directories,
    };
    scopes.push(scope.clone());
    save_scopes(&scopes)?;
    Ok(scope)
}

#[tauri::command]
pub fn delete_search_scope(id: String) -> Result<(), String> {
    let mut scopes = load_scopes();
    scopes.retain(|s| s.id != id);
    save_scopes(&scopes)
}

// Ad-hoc signed rebuilds get a new cdhash, orphaning previous TCC grants.
// Spotlight then silently filters results inside protected directories.
// Probing them here triggers the system permission prompt so the grant is
// recorded against the current binary and unfiltered results come back.
#[cfg(target_os = "macos")]
fn probe_protected_dirs() {
    let Some(home) = dirs::home_dir() else { return };
    for name in ["Downloads", "Documents", "Desktop"] {
        let dir = home.join(name);
        if !dir.exists() {
            continue;
        }
        match std::fs::read_dir(&dir) {
            Ok(_) => flog(&format!("probe: {} accessible", dir.display())),
            Err(e) => flog(&format!("probe: {} DENIED ({})", dir.display(), e)),
        }
    }
}

fn scope_covers_home(directories: &[String]) -> bool {
    let Some(home) = dirs::home_dir() else { return false };
    let home = home.to_string_lossy().to_string();
    directories.is_empty() || directories.iter().any(|d| d == &home)
}

fn resolve_scope(scope: &SearchScopeArg) -> Result<Vec<String>, String> {
    match scope {
        SearchScopeArg::Home => {
            let home = dirs::home_dir().ok_or_else(|| "Failed to resolve home directory".to_string())?;
            Ok(vec![home.to_string_lossy().to_string()])
        }
        SearchScopeArg::Global => Ok(Vec::new()),
        SearchScopeArg::Custom { id } => {
            let scopes = load_scopes();
            let scope = scopes
                .iter()
                .find(|s| &s.id == id)
                .ok_or_else(|| format!("Search scope not found: {}", id))?;
            if scope.directories.is_empty() {
                return Err(format!("Search scope is empty: {}", scope.name));
            }
            Ok(scope.directories.clone())
        }
    }
}

// Kangxi radicals (U+2F00–U+2FDF) are visually identical to common CJK chars
// but are distinct codepoints that Spotlight's name matching does not fold.
// Files named via PDF text extraction often contain them, so we also search
// with radical variants of the query.
fn radical_to_common() -> &'static HashMap<char, char> {
    static MAP: LazyLock<HashMap<char, char>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        for c in '\u{2F00}'..='\u{2FDF}' {
            let decomp: String = c.nfkd().collect();
            if let Some(only) = decomp.chars().next() {
                if decomp.chars().count() == 1 {
                    m.insert(c, only);
                }
            }
        }
        m
    });
    &MAP
}

fn common_to_radical() -> &'static HashMap<char, char> {
    static MAP: LazyLock<HashMap<char, char>> = LazyLock::new(|| {
        radical_to_common()
            .iter()
            .map(|(radical, common)| (*common, *radical))
            .collect()
    });
    &MAP
}

fn expand_query_variants(query: &str) -> Vec<String> {
    let mut variants = Vec::new();
    // Radical query -> common char form: 李⾬(U+2FAC) -> 李雨(U+96E8)
    let nfkd: String = query.nfkd().collect();
    if nfkd != query {
        variants.push(nfkd);
    }
    // Common char query -> radical form: 李雨 -> 李⾬
    let radical_map = common_to_radical();
    let expanded: String = query
        .chars()
        .map(|c| radical_map.get(&c).copied().unwrap_or(c))
        .collect();
    if expanded != query && !variants.contains(&expanded) {
        variants.push(expanded);
    }
    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_common_and_radical_forms() {
        // 李雨 with common 雨 U+96E8 -> expands to radical 雨 U+2FAC form
        let v = expand_query_variants("\u{674E}\u{96E8}");
        assert!(v.contains(&"\u{674E}\u{2FAC}".to_string()));
        // 李⾬ with radical 雨 U+2FAC -> expands to common 雨 U+96E8 form
        let v = expand_query_variants("\u{674E}\u{2FAC}");
        assert!(v.contains(&"\u{674E}\u{96E8}".to_string()));
        // plain query with no radical counterparts -> no variants
        assert!(expand_query_variants("resume").is_empty());
    }
}

fn build_mdfind_args(directories: &[String], query: &str, mode: SearchMode) -> Vec<Vec<String>> {
    let mut jobs = Vec::new();
    if directories.is_empty() {
        // Global search: no -onlyin (all indexed volumes)
        let mut args = vec![MDFIND_BIN.to_string()];
        if matches!(mode, SearchMode::Name) {
            args.push("-name".to_string());
        }
        args.push(query.to_string());
        jobs.push(args);
    } else {
        for dir in directories {
            let mut args = vec![MDFIND_BIN.to_string(), "-onlyin".to_string(), dir.clone()];
            if matches!(mode, SearchMode::Name) {
                args.push("-name".to_string());
            }
            args.push(query.to_string());
            jobs.push(args);
        }
    }
    jobs
}

#[tauri::command]
pub fn cancel_search(search_id: String) -> Result<(), String> {
    if let Some(active) = ACTIVE_SEARCHES.lock().unwrap().get(&search_id) {
        flog(&format!("cancel: id={}", search_id));
        active.cancelled.store(true, Ordering::SeqCst);
        let mut children = active.children.lock().unwrap();
        for child in children.iter_mut() {
            let _ = child.start_kill();
        }
        children.clear();
    }
    Ok(())
}

#[tauri::command]
pub async fn start_search(
    app: tauri::AppHandle,
    search_id: String,
    query: String,
    mode: SearchMode,
    scope: SearchScopeArg,
    limit: Option<usize>,
) -> Result<(), String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(());
    }
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    flog(&format!(
        "start: id={} query={:?} mode={:?} limit={} exe={:?} cwd={:?}",
        search_id,
        query,
        mode,
        limit,
        std::env::current_exe().ok(),
        std::env::current_dir().ok()
    ));
    let directories = match resolve_scope(&scope) {
        Ok(d) => d,
        Err(e) => {
            flog(&format!("scope resolve FAILED: {} ({:?})", e, scope));
            return Err(e);
        }
    };
    let mut jobs: Vec<Vec<String>> = Vec::new();
    let mut queries = vec![query.clone()];
    queries.extend(expand_query_variants(&query));
    flog(&format!("scope resolved: dirs={:?} variants={:?}", directories, &queries[1..]));
    #[cfg(target_os = "macos")]
    if scope_covers_home(&directories) {
        probe_protected_dirs();
    }
    for q in &queries {
        jobs.extend(build_mdfind_args(&directories, q, mode));
    }
    for j in &jobs {
        flog(&format!("job: {:?}", j));
    }

    cancel_search(search_id.clone())?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let children: Arc<Mutex<Vec<tokio::process::Child>>> = Arc::new(Mutex::new(Vec::new()));
    ACTIVE_SEARCHES.lock().unwrap().insert(
        search_id.clone(),
        ActiveSearch {
            cancelled: cancelled.clone(),
            children: children.clone(),
        },
    );

    let (tx, mut rx) = mpsc::channel::<String>(2048);

    for args in jobs {
        let tx = tx.clone();
        let cancelled = cancelled.clone();
        let children = children.clone();
        tauri::async_runtime::spawn(async move {
            let job_desc = args.join(" ");
            let mut cmd = tokio::process::Command::new(&args[0]);
            cmd.args(&args[1..])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    flog(&format!("spawn FAILED: {} ({})", job_desc, e));
                    return;
                }
            };
            let pid = child.id();
            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => return,
            };
            if let Some(mut stderr) = child.stderr.take() {
                tauri::async_runtime::spawn(async move {
                    let mut buf = String::new();
                    let _ = tokio::io::AsyncReadExt::read_to_string(&mut stderr, &mut buf).await;
                    for line in buf.lines() {
                        if !line.contains("[UserQueryParser]") {
                            flog(&format!("mdfind stderr: {}", line));
                        }
                    }
                });
            }
            children.lock().unwrap().push(child);
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            let mut count = 0usize;
            while !cancelled.load(Ordering::SeqCst) {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        count += 1;
                        if tx.send(line).await.is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            let exit = if !cancelled.load(Ordering::SeqCst) {
                let taken = {
                    let mut reg = children.lock().unwrap();
                    reg.iter().position(|c| c.id() == pid).map(|i| reg.swap_remove(i))
                };
                match taken {
                    Some(mut c) => match c.wait().await {
                        Ok(status) => format!("{}", status),
                        Err(e) => format!("wait error: {}", e),
                    },
                    None => "no child handle".to_string(),
                }
            } else {
                "cancelled".to_string()
            };
            flog(&format!("job done: pid={:?} lines={} exit={} ({})", pid, count, exit, job_desc));
        });
    }
    drop(tx);

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut seen: HashSet<String> = HashSet::new();
        let mut batch: Vec<String> = Vec::new();
        let mut truncated = false;

        let flush = |app: &tauri::AppHandle, paths: &[String], done: bool, truncated: bool| {
            if paths.is_empty() && !done {
                return;
            }
            flog(&format!(
                "emit: id={} batch={} done={} truncated={}",
                search_id,
                paths.len(),
                done,
                truncated
            ));
            let _ = app.emit(
                EVENT_NAME,
                json!({
                    "searchId": search_id,
                    "paths": paths,
                    "done": done,
                    "truncated": truncated,
                }),
            );
        };

        loop {
            let timer = tokio::time::sleep(Duration::from_millis(200));
            let item: Option<Option<String>> = tokio::select! {
                item = rx.recv() => Some(item),
                _ = timer => None,
            };
            let has_item = item.is_some();
            let channel_done = matches!(item, Some(None));
            if let Some(Some(raw)) = item {
                let path = raw.trim().to_string();
                if !path.is_empty() && seen.insert(path.clone()) {
                    batch.push(path);
                    if seen.len() >= limit {
                        truncated = true;
                        break;
                    }
                }
            }
            if !batch.is_empty() && (batch.len() >= BATCH_SIZE || !has_item) {
                flush(&app_handle, &batch, false, truncated);
                batch.clear();
            }
            if truncated || channel_done {
                break;
            }
        }

        if truncated || cancelled.load(Ordering::SeqCst) {
            let mut children = children.lock().unwrap();
            for child in children.iter_mut() {
                let _ = child.start_kill();
            }
            children.clear();
        }

        flog(&format!(
            "done: id={} total={} truncated={} cancelled={}",
            search_id,
            seen.len(),
            truncated,
            cancelled.load(Ordering::SeqCst)
        ));
        flush(&app_handle, &batch, true, truncated);
        ACTIVE_SEARCHES.lock().unwrap().remove(&search_id);
    });

    Ok(())
}
