use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags};
use std::collections::HashMap;

use crate::models::*;
use crate::session_sources::SessionSource;

pub type SessionSummary = (String, String, i64, i64, f64);

pub fn open_connection() -> Result<Connection, String> {
    let path =
        crate::config::opencode_db_path().ok_or_else(|| "OPENCODE_DB_NOT_FOUND".to_string())?;
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
        .map(crate::session_repository::normalize_summary_keys)
        .unwrap_or_default()
}

fn map_session_row(row: &rusqlite::Row, directory: &str) -> rusqlite::Result<Session> {
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
        source: SessionSource::OpenCode,
        id,
        directory: directory.to_string(),
        title,
        slug,
        agent,
        model_name: Some(model_name),
        cost: Some(cost),
        tokens_input: Some(tokens_input),
        tokens_output: Some(tokens_output),
        tokens_total: Some(tokens_input + tokens_output),
        time_created: epoch_to_iso(time_created),
        time_updated: epoch_to_iso(time_updated),
        time_ago: time_ago(time_updated),
        parent_id,
        file_changes: Some(file_changes),
        first_message,
        hidden: false,
        pinned: false,
        note: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Schema compatibility: OpenCode v1 vs v2
//
// v1 stores sessions in `session` and messages as `message` + `part` rows.
// v2 migrated sessions to `session_v2` and renders messages into a single
// typed `session_message` JSON stream. Both tables coexist after migration
// (the v1 rows become a stale snapshot), so we detect the schema at runtime
// and always prefer v2 rows, falling back to v1 rows that were never
// migrated. Threaded queries go through a `session` CTE with the same id
// filter so the v1 correlated subqueries keep working unchanged.
// ---------------------------------------------------------------------------

fn detect_schema(conn: &Connection) -> &'static str {
    let has = |name: &str| -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1",
            [name],
            |_| Ok(()),
        )
        .is_ok()
    };
    if has("session_v2") {
        "v2"
    } else {
        "v1"
    }
}

/// Whether the OpenCode database has been migrated to the v2 schema.
pub fn is_opencode_v2(conn: &Connection) -> bool {
    detect_schema(conn) == "v2"
}

/// `FROM` fragment selecting sessions with v2 priority and v1 fallback.
/// `session_v2` has columns the v1 table lacks (fork, suspension tracking),
/// so both sides are aligned explicitly and the extras are NULL-padded on the
/// v1 side; `UNION ALL` + `NOT EXISTS` gives rows the v2 shape and stale v1
/// copies only surface when they were never migrated.
fn session_source(schema: &str) -> &'static str {
    match schema {
        "v2" => {
            "(SELECT id, project_id, workspace_id, parent_id, fork_session_id, fork_boundary,
                     slug, directory, path, title, version, share_url,
                     summary_additions, summary_deletions, summary_files, summary_diffs, metadata,
                     cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                     revert, permission, agent, model,
                     time_created, time_updated, time_compacting, time_archived,
                     time_suspended, resume_attempts, time_idle, time_viewed, idle_outcome
              FROM session_v2
              UNION ALL
              SELECT id, project_id, workspace_id, parent_id,
                     NULL, NULL,
                     slug, directory, path, title, version, share_url,
                     summary_additions, summary_deletions, summary_files, summary_diffs, metadata,
                     cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                     revert, permission, agent, model,
                     time_created, time_updated, time_compacting, time_archived,
                     NULL, 0, NULL, NULL, NULL
              FROM session
              WHERE NOT EXISTS (SELECT 1 FROM session_v2 v2 WHERE v2.id = session.id))"
        }
        _ => "(SELECT * FROM session)",
    }
}

/// CTE definition carrying the same selection, so queries can address
/// sessions as `__oc_sessions` while v1 correlated subqueries keep
/// referencing `s.id` unchanged.
fn session_definition(schema: &str) -> String {
    format!("WITH __oc_sessions AS {} ", session_source(schema))
}

/// Correlated subquery yielding the raw JSON of the first user text message,
/// shaped so `extract_text_from_part` can read it in both schemas.
///
/// In mixed databases every v2 row reads `session_message` first, falling
/// back to the v1 `message`/`part` tables for rows that were never migrated.
/// The v1 fallback must live here as literals because the `message` name is
/// shadowed by the caller's CTE.
fn first_user_text_expr(schema: &str) -> &'static str {
    match schema {
        "v2" => {
            "COALESCE(
                (SELECT sm.data FROM session_message sm
                 WHERE sm.session_id = s.id AND sm.type = 'user'
                 ORDER BY sm.seq ASC LIMIT 1),
                (SELECT p.data FROM part p
                 JOIN message m ON p.message_id = m.id
                 WHERE m.session_id = s.id
                 AND json_extract(m.data, '$.role') = 'user'
                 AND json_extract(p.data, '$.type') = 'text'
                 ORDER BY m.time_created ASC
                 LIMIT 1))"
        }
        _ => {
            "(SELECT p.data FROM part p
              JOIN message m ON p.message_id = m.id
              WHERE m.session_id = s.id
              AND json_extract(m.data, '$.role') = 'user'
              AND json_extract(p.data, '$.type') = 'text'
              ORDER BY m.time_created ASC
              LIMIT 1)"
        }
    }
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    let conn = open_connection()?;
    list_directory_groups_with_conn(&conn)
}

fn list_directory_groups_with_conn(conn: &Connection) -> Result<Vec<DirectoryGroup>, String> {
    let source = session_source(detect_schema(conn));
    let mut stmt = conn
        .prepare(&format!(
            "SELECT directory, COUNT(*) as cnt, MAX(time_updated) as max_t, SUM(cost) as total_cost
             FROM {source}
             WHERE parent_id IS NULL
             GROUP BY directory
             ORDER BY max_t DESC"
        ))
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
            let summary = summaries
                .get(&crate::session_repository::normalize_directory_path(&path))
                .cloned();
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
                source_counts: SourceCounts {
                    opencode: cnt,
                    codex: 0,
                    reasonix: 0,
                },
            }
        })
        .collect();

    Ok(groups)
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    let conn = open_connection()?;
    get_sessions_with_conn(&conn, directory)
}

fn get_sessions_with_conn(conn: &Connection, directory: &str) -> Result<Vec<Session>, String> {
    let schema = detect_schema(conn);
    let cte = session_definition(schema);
    let first_expr = first_user_text_expr(schema);
    let mut stmt = conn
        .prepare(&format!(
            "{cte}
             SELECT s.id, s.slug, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output,
                    s.summary_additions, s.summary_deletions, s.parent_id, s.time_created, s.time_updated,
                    {first_expr}
             FROM __oc_sessions s
             WHERE s.directory = ?1
             ORDER BY s.time_created DESC"
        ))
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let sessions = stmt
        .query_map([directory], |row| map_session_row(row, directory))
        .map_err(|e| format!("Failed to query sessions: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn get_session_by_id(session_id: &str) -> Result<(Session, String), String> {
    let conn = open_connection()?;
    get_session_by_id_with_conn(&conn, session_id)
}

fn get_session_by_id_with_conn(
    conn: &Connection,
    session_id: &str,
) -> Result<(Session, String), String> {
    let schema = detect_schema(conn);
    let cte = session_definition(schema);
    let first_expr = first_user_text_expr(schema);
    let mut stmt = conn
        .prepare(&format!(
            "{cte}
             SELECT s.id, s.slug, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output,
                    s.summary_additions, s.summary_deletions, s.parent_id, s.time_created, s.time_updated,
                    s.directory,
                    {first_expr}
             FROM __oc_sessions s
             WHERE s.id = ?1"
        ))
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
            let directory: String = row.get(13)?;
            let first_message_raw: Option<String> = row.get(14)?;

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
                    source: SessionSource::OpenCode,
                    id,
                    directory,
                    title,
                    slug,
                    agent,
                    model_name: Some(model_name),
                    cost: Some(cost),
                    tokens_input: Some(tokens_input),
                    tokens_output: Some(tokens_output),
                    tokens_total: Some(tokens_input + tokens_output),
                    time_created: epoch_to_iso(time_created),
                    time_updated: epoch_to_iso(time_updated),
                    time_ago: time_ago(time_updated),
                    parent_id,
                    file_changes: Some(file_changes),
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
        .and_then(|v| {
            v.get("text")
                .and_then(|t| t.as_str().map(|s| s.to_string()))
        })
}

pub fn get_first_user_text(session_id: &str, conn: &Connection) -> Option<String> {
    let legacy_query = "SELECT p.data FROM part p
         JOIN message m ON p.message_id = m.id
         WHERE m.session_id = ?1
         AND json_extract(m.data, '$.role') = 'user'
         AND json_extract(p.data, '$.type') = 'text'
         ORDER BY m.time_created ASC
         LIMIT 1";
    let data_str: Option<String> = match detect_schema(conn) {
        "v2" => conn
            .query_row(
                "SELECT sm.data FROM session_message sm
                 WHERE sm.session_id = ?1 AND sm.type = 'user'
                 ORDER BY sm.seq ASC LIMIT 1",
                [session_id],
                |row| row.get(0),
            )
            .ok()
            .or_else(|| {
                conn.query_row(legacy_query, [session_id], |row| row.get(0))
                    .ok()
            }),
        _ => conn
            .query_row(legacy_query, [session_id], |row| row.get(0))
            .ok(),
    };
    data_str.and_then(|raw| extract_text_from_part(&raw))
}

pub fn get_session_detail(session_id: &str) -> Result<SessionDetail, String> {
    let conn = open_connection()?;
    get_session_detail_with_conn(&conn, session_id)
}

fn get_session_detail_with_conn(
    conn: &Connection,
    session_id: &str,
) -> Result<SessionDetail, String> {
    let (session, model_provider) = get_session_by_id_with_conn(conn, session_id)?;

    let messages_count: i64 = match detect_schema(conn) {
        "v2" => conn
            .query_row(
                "SELECT COUNT(*) FROM session_message
                 WHERE session_id = ?1 AND type IN ('user', 'assistant')",
                [session_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count messages: {}", e))?,
        _ => conn
            .query_row(
                "SELECT COUNT(*) FROM message WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count messages: {}", e))?,
    };

    Ok(SessionDetail {
        first_message: session.first_message.clone(),
        session,
        messages_count: Some(messages_count),
        model_provider: Some(model_provider),
    })
}

pub fn get_session_count_for_directory(directory: &str) -> Result<i64, String> {
    let conn = open_connection()?;
    let source = session_source(detect_schema(&conn));
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {source} WHERE directory = ?1 AND parent_id IS NULL"),
        [directory],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to count sessions: {}", e))
}

pub fn get_session_summaries_by_ids(
    session_ids: &[String],
) -> Result<HashMap<String, SessionSummary>, String> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let conn = open_connection()?;
    let source = session_source(detect_schema(&conn));
    let placeholders: Vec<String> = (0..session_ids.len()).map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, title, directory, time_created, time_updated, cost FROM {source} WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare: {}", e))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = session_ids
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
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
    let source = session_source(detect_schema(&conn));
    let mut stmt = conn
        .prepare(&format!(
            "SELECT title FROM {source}
             WHERE directory = ?1
             ORDER BY time_updated DESC
             LIMIT ?2"
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    const MIGRATION_NOTE: &str = "migrated note";

    fn create_v2_schema(conn: &Connection) {
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             CREATE TABLE session (
                 id text PRIMARY KEY,
                 project_id text NOT NULL,
                 parent_id text,
                 slug text NOT NULL,
                 directory text NOT NULL,
                 title text NOT NULL,
                 version text NOT NULL,
                 share_url text,
                 summary_additions integer,
                 summary_deletions integer,
                 summary_files integer,
                 summary_diffs text,
                 revert text,
                 permission text,
                 time_created integer NOT NULL,
                 time_updated integer NOT NULL,
                 time_compacting integer,
                 time_archived integer,
                 workspace_id text,
                 path text,
                 agent text,
                 model text,
                 cost real DEFAULT 0 NOT NULL,
                 tokens_input integer DEFAULT 0 NOT NULL,
                 tokens_output integer DEFAULT 0 NOT NULL,
                 tokens_reasoning integer DEFAULT 0 NOT NULL,
                 tokens_cache_read integer DEFAULT 0 NOT NULL,
                 tokens_cache_write integer DEFAULT 0 NOT NULL,
                 metadata text
             );
             CREATE TABLE session_v2 (
                 id text PRIMARY KEY,
                 project_id text NOT NULL,
                 workspace_id text,
                 parent_id text,
                 fork_session_id text,
                 fork_boundary text,
                 slug text NOT NULL,
                 directory text NOT NULL,
                 path text,
                 title text,
                 version text NOT NULL,
                 share_url text,
                 summary_additions integer,
                 summary_deletions integer,
                 summary_files integer,
                 summary_diffs text,
                 metadata text,
                 cost real DEFAULT 0 NOT NULL,
                 tokens_input integer DEFAULT 0 NOT NULL,
                 tokens_output integer DEFAULT 0 NOT NULL,
                 tokens_reasoning integer DEFAULT 0 NOT NULL,
                 tokens_cache_read integer DEFAULT 0 NOT NULL,
                 tokens_cache_write integer DEFAULT 0 NOT NULL,
                 revert text,
                 permission text,
                 agent text,
                 model text,
                 time_created integer NOT NULL,
                 time_updated integer NOT NULL,
                 time_compacting integer,
                 time_archived integer,
                 time_suspended integer,
                 resume_attempts integer DEFAULT 0 NOT NULL,
                 time_idle integer,
                 time_viewed integer,
                 idle_outcome text
             );
             CREATE TABLE session_message (
                 id text PRIMARY KEY,
                 session_id text NOT NULL,
                 type text NOT NULL,
                 seq integer NOT NULL,
                 time_created integer NOT NULL,
                 time_updated integer NOT NULL,
                 data text NOT NULL
             );
             CREATE TABLE message (
                 id text PRIMARY KEY,
                 session_id text NOT NULL,
                 time_created integer NOT NULL DEFAULT 0,
                 time_updated integer NOT NULL DEFAULT 0,
                 data text NOT NULL
             );
             CREATE TABLE part (
                 id text PRIMARY KEY,
                 message_id text NOT NULL,
                 session_id text NOT NULL,
                 time_created integer NOT NULL DEFAULT 0,
                 time_updated integer NOT NULL DEFAULT 0,
                 data text NOT NULL
             );",
        )
        .unwrap();
    }

    fn create_v1_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE session (
                 id text PRIMARY KEY,
                 project_id text NOT NULL,
                 parent_id text,
                 slug text NOT NULL,
                 directory text NOT NULL,
                 title text NOT NULL,
                 version text NOT NULL,
                 share_url text,
                 summary_additions integer,
                 summary_deletions integer,
                 summary_files integer,
                 summary_diffs text,
                 revert text,
                 permission text,
                 time_created integer NOT NULL,
                 time_updated integer NOT NULL,
                 time_compacting integer,
                 time_archived integer,
                 workspace_id text,
                 path text,
                 agent text,
                 model text,
                 cost real DEFAULT 0 NOT NULL,
                 tokens_input integer DEFAULT 0 NOT NULL,
                 tokens_output integer DEFAULT 0 NOT NULL,
                 tokens_reasoning integer DEFAULT 0 NOT NULL,
                 tokens_cache_read integer DEFAULT 0 NOT NULL,
                 tokens_cache_write integer DEFAULT 0 NOT NULL,
                 metadata text
             );
             CREATE TABLE message (
                 id text PRIMARY KEY,
                 session_id text NOT NULL,
                 time_created integer NOT NULL DEFAULT 0,
                 time_updated integer NOT NULL DEFAULT 0,
                 data text NOT NULL
             );
             CREATE TABLE part (
                 id text PRIMARY KEY,
                 message_id text NOT NULL,
                 session_id text NOT NULL,
                 time_created integer NOT NULL DEFAULT 0,
                 time_updated integer NOT NULL DEFAULT 0,
                 data text NOT NULL
             );",
        )
        .unwrap();
    }

    fn insert_session_v2(conn: &Connection, id: &str, dir: &str, title: &str, updated: i64) {
        conn.execute(
            "INSERT INTO session_v2 (id, project_id, directory, slug, title, version, time_created, time_updated, agent, model)
             VALUES (?1, 'p', ?2, 'slug', ?3, '2.0', ?4, ?5, 'build', '{\"id\":\"m1\",\"providerID\":\"p1\"}')",
            params![id, dir, title, updated - 1, updated],
        )
        .unwrap();
    }

    fn insert_session_v1(conn: &Connection, id: &str, dir: &str, title: &str, updated: i64) {
        conn.execute(
            "INSERT INTO session (id, project_id, directory, slug, title, version, time_created, time_updated, agent, model)
             VALUES (?1, 'p', ?2, 'slug', ?3, '1.18', ?4, ?5, 'build', '{\"id\":\"m1\",\"providerID\":\"p1\"}')",
            params![id, dir, title, updated - 1, updated],
        )
        .unwrap();
    }

    fn insert_session_message(
        conn: &Connection,
        id: &str,
        sid: &str,
        msg_type: &str,
        seq: i64,
        text: &str,
    ) {
        let data = format!("{{\"text\":\"{text}\"}}");
        conn.execute(
            "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, ?4, 1, 1, ?5)",
            params![id, sid, msg_type, seq, data],
        )
        .unwrap();
    }

    fn insert_legacy_message(
        conn: &Connection,
        session_id: &str,
        msg_id: &str,
        role: &str,
        text: &str,
    ) {
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data)
             VALUES (?1, ?2, 1, 1, ?3)",
            params![msg_id, session_id, format!("{{\"role\":\"{role}\"}}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, 1, 1, ?4)",
            params![
                format!("prt_{msg_id}"),
                msg_id,
                session_id,
                format!("{{\"type\":\"text\",\"text\":\"{text}\"}}")
            ],
        )
        .unwrap();
    }

    #[test]
    fn v2_sessions_and_messages_are_read() {
        let conn = Connection::open_in_memory().unwrap();
        create_v2_schema(&conn);

        insert_session_v2(&conn, "ses_new", "/repo", "new title", 2_000);
        insert_session_message(&conn, "msg_1", "ses_new", "user", 1, "hello v2");
        insert_session_message(&conn, "msg_2", "ses_new", "assistant", 2, "reply");
        insert_session_message(&conn, "msg_3", "ses_new", "system", 3, "system note");

        assert_eq!(detect_schema(&conn), "v2");

        let sessions = get_sessions_with_conn(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "ses_new");
        assert_eq!(sessions[0].first_message, "hello v2");

        let detail = get_session_detail_with_conn(&conn, "ses_new").unwrap();
        assert_eq!(detail.messages_count, Some(2));

        assert_eq!(
            get_first_user_text("ses_new", &conn).as_deref(),
            Some("hello v2")
        );
    }

    #[test]
    fn v2_rows_win_over_stale_v1_rows() {
        let conn = Connection::open_in_memory().unwrap();
        create_v2_schema(&conn);

        insert_session_v1(&conn, "ses_shared", "/repo", "old title", 1_000);
        insert_session_v2(&conn, "ses_shared", "/repo", "new title", 3_000);
        insert_legacy_message(&conn, "ses_shared", "msg_old", "user", "old v1 text");
        insert_session_message(&conn, "msg_1", "ses_shared", "user", 1, "new v2 text");

        let sessions = get_sessions_with_conn(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 1, "v2 row must not double with its v1 copy");
        assert_eq!(sessions[0].title, "new title");
        assert_eq!(sessions[0].first_message, "new v2 text");

        let groups = list_directory_groups_with_conn(&conn).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].session_count, 1);
    }

    #[test]
    fn v1_only_rows_are_preserved_when_both_tables_exist() {
        let conn = Connection::open_in_memory().unwrap();
        create_v2_schema(&conn);

        insert_session_v2(&conn, "ses_v2", "/repo", "v2 session", 3_000);
        insert_session_v1(&conn, "ses_left_behind", "/repo", "v1 session", 2_000);
        insert_session_message(&conn, "msg_1", "ses_v2", "user", 1, "v2 text");
        insert_legacy_message(&conn, "ses_left_behind", "msg_old", "user", "v1 text");

        let sessions = get_sessions_with_conn(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(
            sessions
                .iter()
                .find(|s| s.id == "ses_left_behind")
                .unwrap()
                .first_message,
            "v1 text"
        );

        let groups = list_directory_groups_with_conn(&conn).unwrap();
        assert_eq!(groups[0].session_count, 2);

        assert_eq!(
            get_first_user_text("ses_left_behind", &conn).as_deref(),
            Some("v1 text")
        );
    }

    #[test]
    fn v1_only_schema_still_uses_legacy_queries() {
        let conn = Connection::open_in_memory().unwrap();
        create_v1_schema(&conn);

        insert_session_v1(&conn, "ses_v1", "/repo", MIGRATION_NOTE, 1_000);
        insert_legacy_message(&conn, "ses_v1", "msg_1", "user", "legacy text");

        assert_eq!(detect_schema(&conn), "v1");

        let sessions = get_sessions_with_conn(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].first_message, "legacy text");

        let detail = get_session_detail_with_conn(&conn, "ses_v1").unwrap();
        assert_eq!(detail.messages_count, Some(1));

        assert_eq!(
            get_first_user_text("ses_v1", &conn).as_deref(),
            Some("legacy text")
        );
    }

    /// The real v2 database gains extra columns (fork, suspension tracking)
    /// that the v1 table does not have, so the union in `session_source` must
    /// align both sides explicitly.
    #[test]
    fn mixed_union_aligns_differing_real_schemas() {
        let conn = Connection::open_in_memory().unwrap();
        create_v2_schema(&conn);

        let columns = |table: &str| -> Vec<String> {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect()
        };
        let v1_columns = columns("session");
        let v2_columns = columns("session_v2");
        assert!(
            v2_columns.len() > v1_columns.len(),
            "fixture must reproduce the real column-count difference"
        );
        assert!(v2_columns.contains(&"fork_session_id".to_string()));
        assert!(!v1_columns.contains(&"fork_session_id".to_string()));

        insert_session_v2(&conn, "ses_new", "/repo", "new", 3_000);
        insert_session_v1(&conn, "ses_old", "/repo", "old", 2_000);
        insert_session_message(&conn, "msg_1", "ses_new", "user", 1, "new text");
        insert_legacy_message(&conn, "ses_old", "msg_old", "user", "old text");

        let sessions = get_sessions_with_conn(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 2);

        let groups = list_directory_groups_with_conn(&conn).unwrap();
        assert_eq!(groups[0].session_count, 2);

        let detail = get_session_detail_with_conn(&conn, "ses_old").unwrap();
        assert_eq!(detail.first_message, "old text");
    }

    #[test]
    fn v2_schema_is_detected() {
        let conn = Connection::open_in_memory().unwrap();
        create_v2_schema(&conn);
        assert!(is_opencode_v2(&conn));
    }

    #[test]
    fn v1_schema_is_not_v2() {
        let conn = Connection::open_in_memory().unwrap();
        create_v1_schema(&conn);
        assert!(!is_opencode_v2(&conn));
    }
}
