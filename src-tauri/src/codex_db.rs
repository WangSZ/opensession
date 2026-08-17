use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::{epoch_to_iso, time_ago};
use crate::models::{DirectoryGroup, Session, SessionDetail, SourceCounts};
use crate::session_sources::{is_resumable_codex_source, SessionSource};

fn table_columns(conn: &Connection) -> Option<HashSet<String>> {
    let mut stmt = conn.prepare("PRAGMA table_info(threads)").ok()?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1)).ok()?;
    Some(rows.filter_map(Result::ok).collect())
}

fn timestamp_expressions(conn: &Connection) -> Result<(&'static str, &'static str), String> {
    let columns = table_columns(conn).ok_or_else(|| "CODEX_DB_UNSUPPORTED_SCHEMA".to_string())?;
    if columns.contains("created_at_ms") && columns.contains("updated_at_ms") {
        Ok(("created_at_ms", "updated_at_ms"))
    } else if columns.contains("created_at") && columns.contains("updated_at") {
        Ok(("created_at * 1000", "updated_at * 1000"))
    } else {
        Err("CODEX_DB_UNSUPPORTED_SCHEMA".to_string())
    }
}

fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<(String, String, Session)> {
    let source_kind: String = row.get(4)?;
    let model_provider: String = row.get(5)?;
    let created_at: i64 = row.get::<_, Option<i64>>(8)?.unwrap_or_default();
    let updated_at: i64 = row.get::<_, Option<i64>>(9)?.unwrap_or(created_at);
    Ok((
        source_kind,
        model_provider,
        Session {
            source: SessionSource::Codex,
            id: row.get(0)?,
            directory: row.get(1)?,
            title: row.get(2)?,
            slug: String::new(),
            agent: "codex".to_string(),
            model_name: row.get(6)?,
            cost: None,
            tokens_input: None,
            tokens_output: None,
            tokens_total: Some(row.get(7)?),
            time_created: epoch_to_iso(created_at),
            time_updated: epoch_to_iso(updated_at),
            time_ago: time_ago(updated_at),
            parent_id: None,
            file_changes: None,
            first_message: row.get(3)?,
            hidden: false,
            pinned: false,
            note: String::new(),
        },
    ))
}

pub fn query_sessions(conn: &Connection, directory: &str) -> Result<Vec<Session>, String> {
    let (created_at, updated_at) = timestamp_expressions(conn)?;
    let sql = format!(
        "SELECT id, cwd, title, preview, source, model_provider, model, tokens_used,
                {created_at}, {updated_at}
         FROM threads
         WHERE cwd = ?1 AND archived = 0 AND TRIM(preview) <> ''
         ORDER BY {updated_at} DESC"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare Codex query: {}", e))?;

    let rows = stmt
        .query_map([directory], map_session_row)
        .map_err(|e| format!("Failed to query Codex sessions: {}", e))?;

    Ok(rows
        .filter_map(Result::ok)
        .filter(|(source, _, _)| is_resumable_codex_source(source))
        .map(|(_, _, session)| session)
        .collect())
}

pub fn query_directory_groups(conn: &Connection) -> Result<Vec<DirectoryGroup>, String> {
    let (_, updated_at) = timestamp_expressions(conn)?;
    let sql = format!(
        "SELECT cwd, COUNT(*), MAX({updated_at})
         FROM threads
         WHERE archived = 0 AND TRIM(preview) <> ''
           AND source IN ('cli', 'vscode', 'appServer')
         GROUP BY cwd
         ORDER BY MAX({updated_at}) DESC"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare Codex directory query: {}", e))?;

    let groups = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let session_count: i64 = row.get(1)?;
            let last_active_epoch: i64 = row.get::<_, Option<i64>>(2)?.unwrap_or_default();
            let path_ref = std::path::Path::new(&path);
            let name = path_ref
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&path)
                .to_string();

            Ok(DirectoryGroup {
                path: path.clone(),
                name,
                session_count,
                last_active: epoch_to_iso(last_active_epoch),
                last_active_epoch,
                summary: None,
                has_agents_md: path_ref.join("AGENTS.md").exists(),
                tags: Vec::new(),
                pinned: false,
                hidden: false,
                is_missing: !path_ref.exists(),
                is_git_repo: false,
                is_worktree: false,
                repo_name: None,
                branch_name: None,
                total_cost: 0.0,
                source_counts: SourceCounts {
                    opencode: 0,
                    codex: session_count,
                    reasonix: 0,
                },
            })
        })
        .map_err(|e| format!("Failed to query Codex directories: {}", e))?
        .filter_map(Result::ok)
        .collect();

    Ok(groups)
}

pub fn query_session_detail(conn: &Connection, session_id: &str) -> Result<SessionDetail, String> {
    let (created_at, updated_at) = timestamp_expressions(conn)?;
    let sql = format!(
        "SELECT id, cwd, title, preview, source, model_provider, model, tokens_used,
                {created_at}, {updated_at}
         FROM threads
         WHERE id = ?1 AND archived = 0 AND TRIM(preview) <> ''"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare Codex detail query: {}", e))?;
    let (source_kind, model_provider, session) = stmt
        .query_row([session_id], map_session_row)
        .map_err(|e| format!("Codex session not found: {}", e))?;

    if !is_resumable_codex_source(&source_kind) {
        return Err("CODEX_SESSION_NOT_RESUMABLE".to_string());
    }

    Ok(SessionDetail {
        first_message: session.first_message.clone(),
        session,
        messages_count: None,
        model_provider: Some(model_provider),
    })
}

pub fn has_supported_schema(conn: &Connection) -> bool {
    let required: HashSet<&str> = [
        "id",
        "cwd",
        "title",
        "preview",
        "source",
        "model_provider",
        "model",
        "tokens_used",
        "archived",
    ]
    .into_iter()
    .collect();

    let Some(columns) = table_columns(conn) else {
        return false;
    };
    let has_timestamps = (columns.contains("created_at_ms") && columns.contains("updated_at_ms"))
        || (columns.contains("created_at") && columns.contains("updated_at"));
    has_timestamps && required.iter().all(|column| columns.contains(*column))
}

fn state_db_version(path: &Path) -> Option<u64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("state_")?
        .strip_suffix(".sqlite")?
        .parse()
        .ok()
}

pub fn find_supported_db_in(root: &Path) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|_| "CODEX_DB_NOT_FOUND".to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| state_db_version(path).is_some())
        .collect();

    candidates.sort_by_key(|path| std::cmp::Reverse(state_db_version(path).unwrap_or_default()));
    if candidates.is_empty() {
        return Err("CODEX_DB_NOT_FOUND".to_string());
    }

    for path in candidates {
        if let Ok(conn) = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            if has_supported_schema(&conn) {
                return Ok(path);
            }
        }
    }

    Err("CODEX_DB_UNSUPPORTED_SCHEMA".to_string())
}

pub fn codex_home() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Some(path);
        }
    }
    dirs::home_dir().map(|home| home.join(".codex"))
}

pub fn open_connection() -> Result<Connection, String> {
    let home = codex_home().ok_or_else(|| "CODEX_DB_NOT_FOUND".to_string())?;
    let path = find_supported_db_in(&home)?;
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("CODEX_DB_READ_ERROR: {}", e))
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    query_directory_groups(&open_connection()?)
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    query_sessions(&open_connection()?, directory)
}

pub fn get_session_detail(session_id: &str) -> Result<SessionDetail, String> {
    query_session_detail(&open_connection()?, session_id)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::session_sources::SessionSource;

    fn connection_with_threads() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                cwd TEXT NOT NULL,
                title TEXT NOT NULL,
                preview TEXT NOT NULL,
                source TEXT NOT NULL,
                model_provider TEXT NOT NULL,
                model TEXT,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                created_at_ms INTEGER,
                updated_at_ms INTEGER,
                archived INTEGER NOT NULL DEFAULT 0,
                has_user_event INTEGER NOT NULL DEFAULT 0,
                agent_nickname TEXT,
                agent_role TEXT
            );
            INSERT INTO threads VALUES
                ('interactive', '/repo', 'Interactive', 'first prompt', 'cli', 'openai', 'gpt-5.6', 4200, 1000, 2000, 0, 0, NULL, NULL),
                ('exec', '/repo', 'Exec', 'automated', 'exec', 'openai', 'gpt-5.6', 10, 1000, 3000, 0, 1, NULL, NULL),
                ('archived', '/repo', 'Archived', 'old prompt', 'cli', 'openai', 'gpt-5.6', 20, 1000, 4000, 1, 1, NULL, NULL),
                ('empty', '/repo', 'Empty', '', 'cli', 'openai', 'gpt-5.6', 0, 1000, 5000, 0, 0, NULL, NULL),
                ('other-dir', '/other', 'Other', 'other prompt', 'vscode', 'openai', 'gpt-5.5', 30, 1000, 6000, 0, 1, NULL, NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn query_sessions_returns_only_resumable_threads_for_directory() {
        let conn = connection_with_threads();

        let sessions = query_sessions(&conn, "/repo").unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source, SessionSource::Codex);
        assert_eq!(sessions[0].id, "interactive");
        assert_eq!(sessions[0].title, "Interactive");
        assert_eq!(sessions[0].first_message, "first prompt");
        assert_eq!(sessions[0].model_name.as_deref(), Some("gpt-5.6"));
        assert_eq!(sessions[0].tokens_total, Some(4200));
    }

    #[test]
    fn list_directory_groups_counts_only_resumable_threads() {
        let conn = connection_with_threads();

        let groups = query_directory_groups(&conn).unwrap();

        assert_eq!(groups.len(), 2);
        let repo = groups.iter().find(|group| group.path == "/repo").unwrap();
        assert_eq!(repo.session_count, 1);
        assert_eq!(repo.source_counts.opencode, 0);
        assert_eq!(repo.source_counts.codex, 1);
        assert_eq!(repo.last_active_epoch, 2_000);
    }

    #[test]
    fn schema_validation_rejects_missing_required_columns() {
        let valid = Connection::open_in_memory().unwrap();
        valid
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY, cwd TEXT NOT NULL, title TEXT NOT NULL,
                    preview TEXT NOT NULL, source TEXT NOT NULL, model_provider TEXT NOT NULL,
                    model TEXT, tokens_used INTEGER NOT NULL, created_at_ms INTEGER,
                    updated_at_ms INTEGER, archived INTEGER NOT NULL, has_user_event INTEGER NOT NULL
                );",
            )
            .unwrap();
        let invalid = Connection::open_in_memory().unwrap();
        invalid
            .execute_batch("CREATE TABLE threads (id TEXT PRIMARY KEY, cwd TEXT NOT NULL);")
            .unwrap();

        assert!(has_supported_schema(&valid));
        assert!(!has_supported_schema(&invalid));
    }

    #[test]
    fn session_detail_exposes_codex_model_provider_without_fake_message_count() {
        let conn = connection_with_threads();

        let detail = query_session_detail(&conn, "interactive").unwrap();

        assert_eq!(detail.model_provider.as_deref(), Some("openai"));
        assert_eq!(detail.messages_count, None);
        assert_eq!(detail.session.source, SessionSource::Codex);
    }

    #[test]
    fn discovery_uses_highest_supported_state_database() {
        let root = std::env::temp_dir().join(format!("opensession-codex-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let unsupported_path = root.join("state_6.sqlite");
        Connection::open(&unsupported_path)
            .unwrap()
            .execute_batch("CREATE TABLE threads (id TEXT PRIMARY KEY);")
            .unwrap();

        let supported_path = root.join("state_5.sqlite");
        Connection::open(&supported_path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY, cwd TEXT NOT NULL, title TEXT NOT NULL,
                    preview TEXT NOT NULL, source TEXT NOT NULL, model_provider TEXT NOT NULL,
                    model TEXT, tokens_used INTEGER NOT NULL, created_at_ms INTEGER,
                    updated_at_ms INTEGER, archived INTEGER NOT NULL, has_user_event INTEGER NOT NULL,
                    agent_nickname TEXT, agent_role TEXT
                );",
            )
            .unwrap();

        let discovered = find_supported_db_in(&root).unwrap();

        assert_eq!(discovered, supported_path);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queries_fall_back_to_second_precision_timestamps() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY, cwd TEXT NOT NULL, title TEXT NOT NULL,
                preview TEXT NOT NULL, source TEXT NOT NULL, model_provider TEXT NOT NULL,
                model TEXT, tokens_used INTEGER NOT NULL, created_at INTEGER,
                updated_at INTEGER, archived INTEGER NOT NULL
            );
            INSERT INTO threads VALUES
                ('legacy', '/repo', 'Legacy', 'legacy prompt', 'cli', 'openai',
                 'gpt-5.5', 50, 1700000000, 1700000100, 0);",
        )
        .unwrap();

        assert!(has_supported_schema(&conn));
        let sessions = query_sessions(&conn, "/repo").unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].time_updated.starts_with("2023-"));
    }
}
