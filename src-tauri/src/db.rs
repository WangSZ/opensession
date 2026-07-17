use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags};
use std::collections::HashMap;

use crate::models::*;

pub type SessionSummary = (String, String, i64, i64, f64);

pub fn open_connection() -> Result<Connection, String> {
    let path = crate::config::opencode_db_path()
        .ok_or_else(|| "OPENCODE_DB_NOT_FOUND".to_string())?;
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open database: {}", e))
}

fn parse_model_json(model_str: &str) -> (String, String) {
    serde_json::from_str::<serde_json::Value>(model_str)
        .map(|v| {
            let name = v
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let provider = v
                .get("providerID")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (name, provider)
        })
        .unwrap_or(("unknown".to_string(), "unknown".to_string()))
}

pub fn epoch_to_iso(epoch_ms: i64) -> String {
    DateTime::from_timestamp_millis(epoch_ms)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

pub fn time_ago(epoch_ms: i64) -> String {
    if let Some(then) = DateTime::from_timestamp_millis(epoch_ms) {
        let now = Utc::now();
        let seconds = (now - then).num_seconds();
        if seconds < 60 {
            return "just now".to_string();
        } else if seconds < 3600 {
            return format!("{} min ago", seconds / 60);
        } else if seconds < 86400 {
            return format!("{} hours ago", seconds / 3600);
        } else if seconds < 604800 {
            return format!("{} days ago", seconds / 86400);
        } else {
            return then.format("%Y-%m-%d").to_string();
        }
    }
    String::new()
}

fn read_cached_summaries() -> HashMap<String, Summary> {
    let cache_path = crate::config::app_data_dir().join("summaries.json");
    if !cache_path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(&cache_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    let id: String = row.get(0)?;
    let slug: String = row.get(1)?;
    let title: String = row.get(2)?;
    let agent: String = row.get(3)?;
    let model_str: String = row.get(4)?;
    let cost: f64 = row.get(5)?;
    let tokens_input: i64 = row.get(6)?;
    let tokens_output: i64 = row.get(7)?;
    let summary_additions: Option<i64> = row.get(8)?;
    let summary_deletions: Option<i64> = row.get(9)?;
    let parent_id: Option<String> = row.get(10)?;
    let time_created: i64 = row.get(11)?;
    let time_updated: i64 = row.get(12)?;
    let first_message_raw: Option<String> = row.get(13)?;

    let (model_name, _) = parse_model_json(&model_str);
    let file_changes = summary_additions.unwrap_or(0) + summary_deletions.unwrap_or(0);
    let first_message: String = first_message_raw
        .as_deref()
        .and_then(extract_text_from_part)
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect();

    Ok(Session {
        id,
        title,
        slug,
        agent,
        model_name,
        cost,
        tokens_input,
        tokens_output,
        time_created: epoch_to_iso(time_created),
        time_updated: epoch_to_iso(time_updated),
        time_ago: time_ago(time_updated),
        parent_id,
        file_changes,
        first_message,
        hidden: false,
        pinned: false,
        note: String::new(),
    })
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    let conn = open_connection()?;
    let mut stmt = conn
        .prepare(
            "SELECT directory, COUNT(*) as cnt, MAX(time_updated) as max_t, SUM(cost) as total_cost
             FROM session
             WHERE parent_id IS NULL
             GROUP BY directory
             ORDER BY max_t DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let summaries = read_cached_summaries();

    let groups = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let cnt: i64 = row.get(1)?;
            let max_t: i64 = row.get(2)?;
            let total_cost: f64 = row.get(3)?;
            Ok((path, cnt, max_t, total_cost))
        })
        .map_err(|e| format!("Failed to query directory groups: {}", e))?
        .filter_map(|r| r.ok())
        .map(|(path, cnt, max_t, total_cost)| {
            let p = std::path::Path::new(&path);
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string();
            let has_agents_md = p.join("AGENTS.md").exists();
            let is_missing = !p.exists();
            let summary = summaries.get(&path).cloned();
            let last_active = epoch_to_iso(max_t);

            DirectoryGroup {
                path,
                name,
                session_count: cnt,
                last_active,
                last_active_epoch: max_t,
                summary,
                has_agents_md,
                pinned: false,
                hidden: false,
                tags: Vec::new(),
                is_missing,
                is_git_repo: false,
                is_worktree: false,
                repo_name: None,
                branch_name: None,
                total_cost,
            }
        })
        .collect();

    Ok(groups)
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    let conn = open_connection()?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.slug, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output,
                    s.summary_additions, s.summary_deletions, s.parent_id, s.time_created, s.time_updated,
                    (SELECT p.data FROM part p
                     JOIN message m ON p.message_id = m.id
                     WHERE m.session_id = s.id
                     AND json_extract(m.data, '$.role') = 'user'
                     AND json_extract(p.data, '$.type') = 'text'
                     ORDER BY m.time_created ASC
                     LIMIT 1)
             FROM session s
             WHERE s.directory = ?1
             ORDER BY s.time_created DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let sessions = stmt
        .query_map([directory], |row| map_session_row(row))
        .map_err(|e| format!("Failed to query sessions: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn get_session_by_id(session_id: &str) -> Result<(Session, String), String> {
    let conn = open_connection()?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.slug, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output,
                    s.summary_additions, s.summary_deletions, s.parent_id, s.time_created, s.time_updated,
                    (SELECT p.data FROM part p
                     JOIN message m ON p.message_id = m.id
                     WHERE m.session_id = s.id
                     AND json_extract(m.data, '$.role') = 'user'
                     AND json_extract(p.data, '$.type') = 'text'
                     ORDER BY m.time_created ASC
                     LIMIT 1)
             FROM session s
             WHERE s.id = ?1",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let (session, model_provider) = stmt
        .query_row([session_id], |row| {
            let id: String = row.get(0)?;
            let slug: String = row.get(1)?;
            let title: String = row.get(2)?;
            let agent: String = row.get(3)?;
            let model_str: String = row.get(4)?;
            let cost: f64 = row.get(5)?;
            let tokens_input: i64 = row.get(6)?;
            let tokens_output: i64 = row.get(7)?;
            let summary_additions: Option<i64> = row.get(8)?;
            let summary_deletions: Option<i64> = row.get(9)?;
            let parent_id: Option<String> = row.get(10)?;
            let time_created: i64 = row.get(11)?;
            let time_updated: i64 = row.get(12)?;
            let first_message_raw: Option<String> = row.get(13)?;

            let (model_name, model_provider) = parse_model_json(&model_str);
            let file_changes = summary_additions.unwrap_or(0) + summary_deletions.unwrap_or(0);
            let first_message: String = first_message_raw
                .as_deref()
                .and_then(extract_text_from_part)
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect();

            Ok((
                Session {
                    id,
                    title,
                    slug,
                    agent,
                    model_name,
                    cost,
                    tokens_input,
                    tokens_output,
                    time_created: epoch_to_iso(time_created),
                    time_updated: epoch_to_iso(time_updated),
                    time_ago: time_ago(time_updated),
                    parent_id,
                    file_changes,
                    first_message,
                    hidden: false,
                    pinned: false,
                    note: String::new(),
                },
                model_provider,
            ))
        })
        .map_err(|e| format!("Session not found: {}", e))?;

    Ok((session, model_provider))
}

fn extract_text_from_part(data_str: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(data_str)
        .ok()
        .and_then(|v| v.get("text").and_then(|t| t.as_str().map(|s| s.to_string())))
}

pub fn get_first_user_text(session_id: &str, conn: &Connection) -> Option<String> {
    let mut stmt = conn
        .prepare(
            "SELECT p.data FROM part p
             JOIN message m ON p.message_id = m.id
             WHERE m.session_id = ?1
             AND json_extract(m.data, '$.role') = 'user'
             AND json_extract(p.data, '$.type') = 'text'
             ORDER BY m.time_created ASC
             LIMIT 1",
        )
        .ok()?;

    stmt.query_row([session_id], |row| {
        let data_str: String = row.get(0)?;
        Ok(extract_text_from_part(&data_str).unwrap_or_default())
    })
    .ok()
}

pub fn get_session_detail(session_id: &str) -> Result<SessionDetail, String> {
    let (session, model_provider) = get_session_by_id(session_id)?;

    let conn = open_connection()?;

    let messages_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM message WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count messages: {}", e))?;

    Ok(SessionDetail {
        first_message: session.first_message.clone(),
        session,
        messages_count,
        model_provider,
    })
}

pub fn get_session_directory(session_id: &str) -> Result<Option<String>, String> {
    let conn = open_connection()?;
    conn.query_row(
        "SELECT directory FROM session WHERE id = ?1",
        [session_id],
        |row| row.get::<_, String>(0),
    )
    .map(Some)
    .or_else(|e| {
        if e == rusqlite::Error::QueryReturnedNoRows {
            Ok(None)
        } else {
            Err(format!("Failed to get session directory: {}", e))
        }
    })
}

pub fn get_session_count_for_directory(directory: &str) -> Result<i64, String> {
    let conn = open_connection()?;
    conn.query_row(
        "SELECT COUNT(*) FROM session WHERE directory = ?1 AND parent_id IS NULL",
        [directory],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to count sessions: {}", e))
}

pub fn get_session_summaries_by_ids(session_ids: &[String]) -> Result<HashMap<String, SessionSummary>, String> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let conn = open_connection()?;
    let placeholders: Vec<String> = (0..session_ids.len()).map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, title, directory, time_created, time_updated, cost FROM session WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to prepare: {}", e))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = session_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt
        .query_map(params.as_slice(), |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let directory: String = row.get(2)?;
            let time_created: i64 = row.get(3)?;
            let time_updated: i64 = row.get(4)?;
            let cost: f64 = row.get(5)?;
            Ok((id, title, directory, time_created, time_updated, cost))
        })
        .map_err(|e| format!("Failed to query: {}", e))?;

    let mut map = HashMap::new();
    for row in rows {
        if let Ok((id, title, directory, time_created, time_updated, cost)) = row {
            map.insert(id, (title, directory, time_created, time_updated, cost));
        }
    }
    Ok(map)
}

pub fn get_recent_session_titles(directory: &str, limit: i64) -> Result<Vec<String>, String> {
    let conn = open_connection()?;
    let mut stmt = conn
        .prepare(
            "SELECT title FROM session
             WHERE directory = ?1
             ORDER BY time_updated DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let titles = stmt
        .query_map(params![directory, limit], |row| {
            let title: String = row.get(0)?;
            Ok(title)
        })
        .map_err(|e| format!("Failed to query titles: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(titles)
}
