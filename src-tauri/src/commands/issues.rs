use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config;
use crate::db;
use crate::models::*;

#[derive(Debug, Serialize, Deserialize, Default)]
struct IssueStore {
    issues: HashMap<String, Issue>,
    #[serde(default)]
    issue_links: HashMap<String, IssueLink>,
    #[serde(default)]
    comments: HashMap<String, Vec<IssueComment>>,
}

fn store_path() -> PathBuf {
    config::app_config_dir().join("issues.json")
}

fn load_store() -> IssueStore {
    let path = store_path();
    if !path.exists() {
        return IssueStore::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_store(store: &IssueStore) -> Result<(), String> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(store).map_err(|e| format!("Serialize error: {}", e))?;
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
    let issue = store.issues.get_mut(&id).ok_or_else(|| "Issue not found".to_string())?;
    if let Some(t) = title {
        issue.title = t;
    }
    if let Some(d) = description {
        issue.description = Some(d);
    }
    if url.is_some() { issue.url = url; }
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
    store.comments.remove(&id);
    save_store(&store)
}

#[tauri::command]
pub fn link_session_to_issue(session_id: String, issue_id: String, directory: String) -> Result<(), String> {
    let mut store = load_store();
    if !store.issues.contains_key(&issue_id) {
        return Err("Issue not found".to_string());
    }
    let link = IssueLink {
        session_id: session_id.clone(),
        issue_id,
        directory,
        linked_at: Utc::now().to_rfc3339(),
    };
    store.issue_links.insert(session_id, link);
    save_store(&store)
}

#[tauri::command]
pub fn unlink_session_from_issue(session_id: String) -> Result<(), String> {
    let mut store = load_store();
    store.issue_links.remove(&session_id);
    save_store(&store)
}

#[tauri::command]
pub fn get_session_issue(session_id: String) -> Result<Option<Issue>, String> {
    let store = load_store();
    if let Some(link) = store.issue_links.get(&session_id) {
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

    let all_session_ids: Vec<String> = store.issue_links.values().map(|l| l.session_id.clone()).collect();
    let session_map = db::get_session_summaries_by_ids(&all_session_ids)?;

    let mut result = Vec::new();
    for issue in store.issues.values() {
        let mut sessions = Vec::new();
        for link in store.issue_links.values() {
            if link.issue_id != issue.id {
                continue;
            }
            if let Some((title, directory, time_created, time_updated, cost)) = session_map.get(&link.session_id) {
                let p = std::path::Path::new(directory);
                let dir_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(directory)
                    .to_string();
                sessions.push(LinkedSession {
                    session_id: link.session_id.clone(),
                    session_title: title.clone(),
                    directory: directory.clone(),
                    directory_name: dir_name,
                    time_created: db::epoch_to_iso(*time_created),
                    time_ago: db::time_ago(*time_updated),
                    cost: *cost,
                });
            }
        }
        sessions.sort_by(|a, b| b.time_created.cmp(&a.time_created));
        result.push(IssueWithSessions {
            issue: issue.clone(),
            sessions,
        });
    }

    result.sort_by(|a, b| {
        let a_max = a.sessions.first().map(|s| s.time_created.clone()).unwrap_or_default();
        let b_max = b.sessions.first().map(|s| s.time_created.clone()).unwrap_or_default();
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
    store.comments.entry(issue_id).or_default().push(comment.clone());
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
