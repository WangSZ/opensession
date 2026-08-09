use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config;
use crate::models::*;
use crate::session_sources::{SessionRef, SessionSource};

#[derive(Debug, Serialize, Deserialize, Default)]
struct IssueStore {
    issues: HashMap<String, Issue>,
    #[serde(default)]
    issue_links: HashMap<String, IssueLink>,
    #[serde(default)]
    comments: HashMap<String, Vec<IssueComment>>,
    #[serde(default)]
    dir_links: HashMap<String, DirectoryLink>,
}

fn store_path() -> PathBuf {
    config::app_config_dir().join("issues.json")
}

fn normalize_store_keys(mut store: IssueStore) -> IssueStore {
    store.issue_links = store
        .issue_links
        .into_values()
        .map(|mut link| {
            link.directory = crate::session_repository::normalize_directory_path(&link.directory);
            let key = SessionRef::new(link.session_source, &link.session_id).storage_key();
            (key, link)
        })
        .collect();
    store.dir_links = store
        .dir_links
        .into_values()
        .map(|mut link| {
            link.directory = crate::session_repository::normalize_directory_path(&link.directory);
            (link.directory.clone(), link)
        })
        .collect();
    store
}

fn load_store() -> IssueStore {
    let path = store_path();
    if !path.exists() {
        return IssueStore::default();
    }
    normalize_store_keys(
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    )
}

fn save_store(store: &IssueStore) -> Result<(), String> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    let json =
        serde_json::to_string_pretty(store).map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write error: {}", e))
}

#[tauri::command]
pub fn create_issue(
    title: String,
    description: Option<String>,
    url: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    deadline: Option<String>,
) -> Result<Issue, String> {
    let mut store = load_store();
    let now = Utc::now().to_rfc3339();
    let issue = Issue {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        description,
        url,
        priority: priority.unwrap_or_else(|| "medium".to_string()),
        status: status.unwrap_or_else(|| "open".to_string()),
        deadline,
        created_at: now.clone(),
        updated_at: now,
    };
    store.issues.insert(issue.id.clone(), issue.clone());
    save_store(&store)?;
    Ok(issue)
}

#[tauri::command]
pub fn update_issue(
    id: String,
    title: Option<String>,
    description: Option<String>,
    url: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    deadline: Option<String>,
) -> Result<Issue, String> {
    let mut store = load_store();
    let issue = store
        .issues
        .get_mut(&id)
        .ok_or_else(|| "Issue not found".to_string())?;
    if let Some(t) = title {
        issue.title = t;
    }
    if let Some(d) = description {
        issue.description = Some(d);
    }
    if url.is_some() {
        issue.url = url;
    }
    if let Some(p) = priority {
        issue.priority = p;
    }
    if let Some(s) = status {
        issue.status = s;
    }
    issue.deadline = deadline;
    issue.updated_at = Utc::now().to_rfc3339();
    let result = issue.clone();
    save_store(&store)?;
    Ok(result)
}

#[tauri::command]
pub fn delete_issue(id: String) -> Result<(), String> {
    let mut store = load_store();
    store.issues.remove(&id);
    store.issue_links.retain(|_, v| v.issue_id != id);
    store.dir_links.retain(|_, v| v.issue_id != id);
    store.comments.remove(&id);
    save_store(&store)
}

#[tauri::command]
pub fn link_session_to_issue(
    session_source: SessionSource,
    session_id: String,
    issue_id: String,
    directory: String,
) -> Result<(), String> {
    let mut store = load_store();
    if !store.issues.contains_key(&issue_id) {
        return Err("Issue not found".to_string());
    }
    let link = IssueLink {
        session_source,
        session_id: session_id.clone(),
        issue_id,
        directory: crate::session_repository::normalize_directory_path(&directory),
        linked_at: Utc::now().to_rfc3339(),
    };
    let key = SessionRef::new(session_source, session_id).storage_key();
    store.issue_links.insert(key, link);
    save_store(&store)
}

#[tauri::command]
pub fn unlink_session_from_issue(
    session_source: SessionSource,
    session_id: String,
) -> Result<(), String> {
    let mut store = load_store();
    let key = SessionRef::new(session_source, session_id).storage_key();
    store.issue_links.remove(&key);
    save_store(&store)
}

#[tauri::command]
pub fn get_session_issue(
    session_source: SessionSource,
    session_id: String,
) -> Result<Option<Issue>, String> {
    let store = load_store();
    let key = SessionRef::new(session_source, &session_id).storage_key();
    // Check session-level link first
    if let Some(link) = store.issue_links.get(&key) {
        return Ok(store.issues.get(&link.issue_id).cloned());
    }
    // Check directory-level link
    if let Ok(detail) = crate::session_repository::get_session_detail(session_source, &session_id) {
        let dir = crate::session_repository::normalize_directory_path(&detail.session.directory);
        if let Some(link) = store.dir_links.get(&dir) {
            return Ok(store.issues.get(&link.issue_id).cloned());
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn link_directory_to_issue(directory: String, issue_id: String) -> Result<(), String> {
    let mut store = load_store();
    if !store.issues.contains_key(&issue_id) {
        return Err("Issue not found".to_string());
    }
    let directory = crate::session_repository::normalize_directory_path(&directory);
    let link = DirectoryLink {
        directory: directory.clone(),
        issue_id,
        linked_at: Utc::now().to_rfc3339(),
    };
    store.dir_links.insert(directory, link);
    save_store(&store)
}

#[tauri::command]
pub fn unlink_directory_from_issue(directory: String) -> Result<(), String> {
    let mut store = load_store();
    let directory = crate::session_repository::normalize_directory_path(&directory);
    store.dir_links.remove(&directory);
    // Also remove any session-level links for sessions in this directory
    // so they don't orphan the issue
    save_store(&store)
}

#[tauri::command]
pub fn get_directory_issue(directory: String) -> Result<Option<Issue>, String> {
    let store = load_store();
    let directory = crate::session_repository::normalize_directory_path(&directory);
    if let Some(link) = store.dir_links.get(&directory) {
        Ok(store.issues.get(&link.issue_id).cloned())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_all_issues() -> Result<Vec<IssueWithSessions>, String> {
    let store = load_store();
    if store.issues.is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for issue in store.issues.values() {
        let mut sessions = Vec::new();
        for link in store.issue_links.values() {
            if link.issue_id != issue.id {
                continue;
            }
            if let Ok(detail) =
                crate::session_repository::get_session_detail(link.session_source, &link.session_id)
            {
                let session = detail.session;
                let directory = &session.directory;
                let p = std::path::Path::new(directory);
                let dir_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(directory)
                    .to_string();
                sessions.push(LinkedSession {
                    session_source: link.session_source,
                    session_id: link.session_id.clone(),
                    session_title: session.title,
                    directory: directory.clone(),
                    directory_name: dir_name,
                    time_created: session.time_created,
                    time_ago: session.time_ago,
                    cost: session.cost,
                });
            }
        }
        sessions.sort_by(|a, b| b.time_created.cmp(&a.time_created));

        // Build directory-level links
        let mut directories = Vec::new();
        for link in store.dir_links.values() {
            if link.issue_id != issue.id {
                continue;
            }
            let count = crate::session_repository::list_directory_groups()
                .ok()
                .and_then(|groups| {
                    groups
                        .into_iter()
                        .find(|group| group.path == link.directory)
                })
                .map(|group| group.session_count)
                .unwrap_or(0);
            let p = std::path::Path::new(&link.directory);
            let dir_name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&link.directory)
                .to_string();
            directories.push(LinkedDirectory {
                directory: link.directory.clone(),
                directory_name: dir_name,
                session_count: count,
            });
        }

        result.push(IssueWithSessions {
            issue: issue.clone(),
            sessions,
            directories,
        });
    }

    result.sort_by(|a, b| {
        let a_max = a
            .sessions
            .first()
            .map(|s| s.time_created.clone())
            .unwrap_or_default();
        let b_max = b
            .sessions
            .first()
            .map(|s| s.time_created.clone())
            .unwrap_or_default();
        b_max.cmp(&a_max)
    });

    Ok(result)
}

#[tauri::command]
pub fn add_comment(issue_id: String, content: String) -> Result<IssueComment, String> {
    let mut store = load_store();
    if !store.issues.contains_key(&issue_id) {
        return Err("Issue not found".to_string());
    }
    let comment = IssueComment {
        id: uuid::Uuid::new_v4().to_string(),
        issue_id: issue_id.clone(),
        content,
        created_at: Utc::now().to_rfc3339(),
    };
    store
        .comments
        .entry(issue_id)
        .or_default()
        .push(comment.clone());
    save_store(&store)?;
    Ok(comment)
}

#[tauri::command]
pub fn delete_comment(issue_id: String, comment_id: String) -> Result<(), String> {
    let mut store = load_store();
    if let Some(comments) = store.comments.get_mut(&issue_id) {
        comments.retain(|c| c.id != comment_id);
    }
    save_store(&store)
}

#[tauri::command]
pub fn get_issue_comments(issue_id: String) -> Result<Vec<IssueComment>, String> {
    let store = load_store();
    Ok(store.comments.get(&issue_id).cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_sources::SessionSource;

    #[test]
    fn legacy_issue_links_migrate_to_opencode_namespace() {
        let mut store = IssueStore::default();
        store.issue_links.insert(
            "legacy-id".to_string(),
            IssueLink {
                session_source: SessionSource::OpenCode,
                session_id: "legacy-id".to_string(),
                issue_id: "issue-1".to_string(),
                directory: "/repo".to_string(),
                linked_at: "2026-01-01T00:00:00Z".to_string(),
            },
        );

        let migrated = normalize_store_keys(store);

        assert!(migrated.issue_links.contains_key("opencode:legacy-id"));
        assert!(!migrated.issue_links.contains_key("legacy-id"));
    }

    #[test]
    fn directory_links_use_normalized_keys() {
        let root = std::env::temp_dir().join(format!("opensession-issue-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let alias = root.join(".").to_string_lossy().to_string();
        let mut store = IssueStore::default();
        store.dir_links.insert(
            alias.clone(),
            DirectoryLink {
                directory: alias,
                issue_id: "issue-1".into(),
                linked_at: "2026-01-01T00:00:00Z".into(),
            },
        );

        let normalized = normalize_store_keys(store);
        let key = std::fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(normalized.dir_links.contains_key(&key));
        std::fs::remove_dir_all(root).unwrap();
    }
}
