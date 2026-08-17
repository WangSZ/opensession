use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::DateTime;
use serde::Deserialize;

use crate::db::{epoch_to_iso, time_ago};
use crate::models::{DirectoryGroup, Session, SessionDetail, SourceCounts};
use crate::session_sources::SessionSource;

#[derive(Debug, Deserialize)]
struct ReasonixMeta {
    id: String,
    #[serde(default)]
    parent_id: Option<String>,
    created_at: String,
    updated_at: String,
    model: String,
    #[serde(default)]
    preview: String,
}

#[derive(Debug, Deserialize)]
struct TitleEntry {
    title: String,
}

struct ReasonixRecord {
    session: Session,
    transcript_path: PathBuf,
    model_provider: Option<String>,
}

pub fn reasonix_home() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("REASONIX_HOME") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Some(path);
        }
    }
    dirs::home_dir().map(|home| home.join(".reasonix"))
}

fn session_directories(home: &Path) -> Result<Vec<PathBuf>, String> {
    if !home.exists() {
        return Err("REASONIX_DATA_NOT_FOUND".to_string());
    }

    let mut directories = Vec::new();
    let direct = home.join("sessions");
    if direct.is_dir() {
        directories.push(direct);
    }

    let projects = home.join("projects");
    if projects.exists() {
        let entries = std::fs::read_dir(&projects)
            .map_err(|error| format!("REASONIX_DATA_READ_ERROR: {}", error))?;
        for entry in entries.flatten() {
            let sessions = entry.path().join("sessions");
            if sessions.is_dir() {
                directories.push(sessions);
            }
        }
    }

    Ok(directories)
}

fn read_titles(session_directory: &Path) -> HashMap<String, TitleEntry> {
    std::fs::read_to_string(session_directory.join(".session-titles.json"))
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn workspace_from_transcript(path: &Path) -> Option<String> {
    for line in BufReader::new(File::open(path).ok()?).lines().take(20) {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(content) = value.get("content").and_then(|content| content.as_str()) else {
            continue;
        };
        let Some((_, remainder)) = content.split_once("Current workspace: \"") else {
            continue;
        };
        return Some(remainder.split_once('"')?.0.to_string());
    }
    None
}

fn timestamp_ms(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn count_transcript_messages(path: &Path) -> i64 {
    BufReader::new(match File::open(path) {
        Ok(file) => file,
        Err(_) => return 0,
    })
    .lines()
    .filter_map(Result::ok)
    .filter(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .is_some_and(|value| value.get("role").and_then(|role| role.as_str()).is_some())
    })
    .count() as i64
}

fn record_from_meta(
    meta_path: &Path,
    titles: &HashMap<String, TitleEntry>,
) -> Option<ReasonixRecord> {
    let meta: ReasonixMeta =
        serde_json::from_str(&std::fs::read_to_string(meta_path).ok()?).ok()?;
    let created_at = timestamp_ms(&meta.created_at)?;
    let updated_at = timestamp_ms(&meta.updated_at)?;
    let transcript_path = meta_path.with_extension("");
    let directory = workspace_from_transcript(&transcript_path)?;
    let transcript_name = transcript_path.file_name()?.to_str()?;
    let title = titles
        .get(transcript_name)
        .map(|entry| entry.title.trim())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| meta.preview.trim())
        .to_string();
    let (model_provider, model_name) = meta
        .model
        .split_once('/')
        .map(|(provider, model)| (Some(provider.to_string()), model.to_string()))
        .unwrap_or((None, meta.model));

    Some(ReasonixRecord {
        session: Session {
            source: SessionSource::Reasonix,
            id: meta.id,
            directory,
            title,
            slug: String::new(),
            agent: "reasonix".to_string(),
            model_name: Some(model_name),
            cost: None,
            tokens_input: None,
            tokens_output: None,
            tokens_total: None,
            time_created: epoch_to_iso(created_at),
            time_updated: epoch_to_iso(updated_at),
            time_ago: time_ago(updated_at),
            parent_id: meta.parent_id,
            file_changes: None,
            first_message: meta.preview,
            hidden: false,
            pinned: false,
            note: String::new(),
        },
        transcript_path,
        model_provider,
    })
}

fn scan_records(home: &Path) -> Result<Vec<ReasonixRecord>, String> {
    let mut records = Vec::new();
    let mut meta_files = 0usize;

    for session_directory in session_directories(home)? {
        let titles = read_titles(&session_directory);
        let entries = std::fs::read_dir(&session_directory)
            .map_err(|error| format!("REASONIX_DATA_READ_ERROR: {}", error))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let is_meta = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".jsonl.meta"));
            if !is_meta {
                continue;
            }
            meta_files += 1;
            if let Some(record) = record_from_meta(&path, &titles) {
                records.push(record);
            }
        }
    }

    if meta_files > 0 && records.is_empty() {
        return Err("REASONIX_DATA_UNSUPPORTED_SCHEMA".to_string());
    }

    records.sort_by(|a, b| b.session.time_updated.cmp(&a.session.time_updated));
    Ok(records)
}

pub fn list_directory_groups_in(home: &Path) -> Result<Vec<DirectoryGroup>, String> {
    let mut grouped = HashMap::<String, DirectoryGroup>::new();

    for record in scan_records(home)? {
        let session = record.session;
        let path = session.directory.clone();
        let updated_at = timestamp_ms(&session.time_updated).unwrap_or_default();
        if let Some(group) = grouped.get_mut(&path) {
            group.session_count += 1;
            group.source_counts.reasonix += 1;
            if updated_at > group.last_active_epoch {
                group.last_active_epoch = updated_at;
                group.last_active = session.time_updated;
            }
            continue;
        }

        let path_ref = Path::new(&path);
        let name = path_ref
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&path)
            .to_string();
        let has_agents_md = path_ref.join("AGENTS.md").exists();
        let is_missing = !path_ref.exists();
        grouped.insert(
            path.clone(),
            DirectoryGroup {
                path,
                name,
                session_count: 1,
                last_active: session.time_updated,
                last_active_epoch: updated_at,
                summary: None,
                has_agents_md,
                tags: Vec::new(),
                pinned: false,
                hidden: false,
                is_missing,
                is_git_repo: false,
                is_worktree: false,
                repo_name: None,
                branch_name: None,
                total_cost: 0.0,
                source_counts: SourceCounts {
                    opencode: 0,
                    codex: 0,
                    reasonix: 1,
                },
            },
        );
    }

    let mut groups: Vec<_> = grouped.into_values().collect();
    groups.sort_by(|a, b| b.last_active_epoch.cmp(&a.last_active_epoch));
    Ok(groups)
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    let home = reasonix_home().ok_or_else(|| "REASONIX_DATA_NOT_FOUND".to_string())?;
    list_directory_groups_in(&home)
}

pub fn get_sessions_in(home: &Path, directory: &str) -> Result<Vec<Session>, String> {
    let normalized = crate::session_repository::normalize_directory_path(directory);
    Ok(scan_records(home)?
        .into_iter()
        .map(|record| record.session)
        .filter(|session| {
            crate::session_repository::normalize_directory_path(&session.directory) == normalized
        })
        .collect())
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    let home = reasonix_home().ok_or_else(|| "REASONIX_DATA_NOT_FOUND".to_string())?;
    get_sessions_in(&home, directory)
}

pub fn get_session_detail_in(home: &Path, session_id: &str) -> Result<SessionDetail, String> {
    let record = scan_records(home)?
        .into_iter()
        .find(|record| record.session.id == session_id)
        .ok_or_else(|| "REASONIX_SESSION_NOT_FOUND".to_string())?;
    let messages_count = count_transcript_messages(&record.transcript_path);
    Ok(SessionDetail {
        first_message: record.session.first_message.clone(),
        session: record.session,
        messages_count: Some(messages_count),
        model_provider: record.model_provider,
    })
}

pub fn get_session_detail(session_id: &str) -> Result<SessionDetail, String> {
    let home = reasonix_home().ok_or_else(|| "REASONIX_DATA_NOT_FOUND".to_string())?;
    get_session_detail_in(&home, session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opensession-reasonix-{}", uuid::Uuid::new_v4()));
        let sessions = root.join("projects").join("-tmp-project").join("sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        let id = "20260817-070035.889638000-deepseek-v4-pro";
        std::fs::write(
            sessions.join(format!("{}.jsonl.meta", id)),
            serde_json::json!({
                "id": id,
                "parent_id": "parent-session",
                "created_at": "2026-08-17T07:00:35.922254Z",
                "updated_at": "2026-08-17T07:04:02.665827Z",
                "model": "deepseek-pro/deepseek-v4-pro",
                "turns": 2,
                "preview": "Implement Reasonix support"
            })
            .to_string(),
        )
        .unwrap();
        let transcript = [
            serde_json::json!({
                "role": "system",
                "content": "You are Reasonix.\n\nCurrent workspace: \"/tmp/project\"\n"
            }),
            serde_json::json!({ "role": "user", "content": "Implement Reasonix support" }),
            serde_json::json!({ "role": "assistant", "content": "Done" }),
        ]
        .into_iter()
        .map(|message| message.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        std::fs::write(sessions.join(format!("{}.jsonl", id)), transcript).unwrap();
        let title_key = format!("{}.jsonl", id);
        let titles = HashMap::from([(
            title_key,
            serde_json::json!({ "title": "Reasonix integration" }),
        )]);
        std::fs::write(
            sessions.join(".session-titles.json"),
            serde_json::to_string(&titles).unwrap(),
        )
        .unwrap();
        root
    }

    #[test]
    fn discovers_reasonix_sessions_and_directory_groups() {
        let root = fixture();

        let groups = list_directory_groups_in(&root).unwrap();
        let sessions = get_sessions_in(&root, "/tmp/project").unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].path, "/tmp/project");
        assert_eq!(groups[0].source_counts.reasonix, 1);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source, SessionSource::Reasonix);
        assert_eq!(sessions[0].title, "Reasonix integration");
        assert_eq!(sessions[0].model_name.as_deref(), Some("deepseek-v4-pro"));
        assert_eq!(sessions[0].parent_id.as_deref(), Some("parent-session"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exposes_reasonix_session_detail() {
        let root = fixture();
        let detail =
            get_session_detail_in(&root, "20260817-070035.889638000-deepseek-v4-pro").unwrap();

        assert_eq!(detail.messages_count, Some(3));
        assert_eq!(detail.model_provider.as_deref(), Some("deepseek-pro"));
        assert_eq!(detail.first_message, "Implement Reasonix support");

        std::fs::remove_dir_all(root).unwrap();
    }
}
