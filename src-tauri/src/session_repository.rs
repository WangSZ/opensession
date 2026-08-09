use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::models::{DirectoryGroup, Session, SessionDetail, SourceStatus, SourceStatuses};
use crate::session_sources::SessionSource;

pub fn normalize_directory_path(path: &str) -> String {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical.to_string_lossy().to_string();
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized.to_string_lossy().to_string()
}

pub fn normalize_summary_keys(
    summaries: HashMap<String, crate::models::Summary>,
) -> HashMap<String, crate::models::Summary> {
    summaries
        .into_iter()
        .map(|(path, summary)| (normalize_directory_path(&path), summary))
        .collect()
}

pub fn merge_directory_groups(
    opencode: Vec<DirectoryGroup>,
    codex: Vec<DirectoryGroup>,
) -> Vec<DirectoryGroup> {
    let mut groups = HashMap::<String, DirectoryGroup>::new();

    for mut group in opencode.into_iter().chain(codex) {
        let normalized_path = normalize_directory_path(&group.path);
        group.path = normalized_path.clone();
        if let Some(existing) = groups.get_mut(&normalized_path) {
            existing.session_count += group.session_count;
            existing.source_counts.opencode += group.source_counts.opencode;
            existing.source_counts.codex += group.source_counts.codex;
            existing.total_cost += group.total_cost;
            if group.last_active_epoch > existing.last_active_epoch {
                existing.last_active_epoch = group.last_active_epoch;
                existing.last_active = group.last_active;
            }
        } else {
            groups.insert(normalized_path, group);
        }
    }

    let mut result: Vec<_> = groups.into_values().collect();
    result.sort_by(|a, b| b.last_active_epoch.cmp(&a.last_active_epoch));
    result
}

pub fn merge_sessions(opencode: Vec<Session>, codex: Vec<Session>) -> Vec<Session> {
    let mut sessions: Vec<_> = opencode.into_iter().chain(codex).collect();
    sessions.sort_by(|a, b| b.time_updated.cmp(&a.time_updated));
    sessions
}

fn combined_error(opencode: String, codex: String) -> String {
    format!("NO_SESSION_DATA: opencode={}; codex={}", opencode, codex)
}

pub fn status_from_directory_result(result: Result<Vec<DirectoryGroup>, String>) -> SourceStatus {
    match result {
        Ok(groups) if groups.is_empty() => SourceStatus::NoSessions,
        Ok(_) => SourceStatus::Available,
        Err(error) if error.contains("UNSUPPORTED_SCHEMA") => SourceStatus::UnsupportedSchema,
        Err(error) if error.contains("NOT_FOUND") => SourceStatus::NotInstalled,
        Err(_) => SourceStatus::ReadError,
    }
}

pub fn get_source_statuses() -> SourceStatuses {
    SourceStatuses {
        opencode: status_from_directory_result(crate::db::list_directory_groups()),
        codex: status_from_directory_result(crate::codex_db::list_directory_groups()),
    }
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    let mut groups = match (
        crate::db::list_directory_groups(),
        crate::codex_db::list_directory_groups(),
    ) {
        (Ok(opencode), Ok(codex)) => merge_directory_groups(opencode, codex),
        (Ok(opencode), Err(_)) => opencode,
        (Err(_), Ok(codex)) => codex,
        (Err(opencode), Err(codex)) => return Err(combined_error(opencode, codex)),
    };

    let cache_path = crate::config::app_data_dir().join("summaries.json");
    let summaries: HashMap<String, crate::models::Summary> = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .map(normalize_summary_keys)
        .unwrap_or_default();
    for group in &mut groups {
        if group.summary.is_none() {
            group.summary = summaries.get(&group.path).cloned();
        }
    }
    Ok(groups)
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    let target = normalize_directory_path(directory);
    let opencode = crate::db::list_directory_groups().and_then(|groups| {
        let paths: Vec<_> = groups
            .into_iter()
            .filter(|group| normalize_directory_path(&group.path) == target)
            .map(|group| group.path)
            .collect();
        let mut sessions = Vec::new();
        for path in paths {
            sessions.extend(crate::db::get_sessions(&path)?);
        }
        Ok(sessions)
    });
    let codex = crate::codex_db::list_directory_groups().and_then(|groups| {
        let paths: Vec<_> = groups
            .into_iter()
            .filter(|group| normalize_directory_path(&group.path) == target)
            .map(|group| group.path)
            .collect();
        let mut sessions = Vec::new();
        for path in paths {
            sessions.extend(crate::codex_db::get_sessions(&path)?);
        }
        Ok(sessions)
    });

    let mut sessions = match (opencode, codex) {
        (Ok(opencode), Ok(codex)) => Ok(merge_sessions(opencode, codex)),
        (Ok(opencode), Err(_)) => Ok(opencode),
        (Err(_), Ok(codex)) => Ok(codex),
        (Err(opencode), Err(codex)) => Err(combined_error(opencode, codex)),
    }?;
    for session in &mut sessions {
        session.directory = target.clone();
    }
    Ok(sessions)
}

pub fn get_session_detail(
    source: SessionSource,
    session_id: &str,
) -> Result<SessionDetail, String> {
    match source {
        SessionSource::OpenCode => crate::db::get_session_detail(session_id),
        SessionSource::Codex => crate::codex_db::get_session_detail(session_id),
    }
}

pub fn get_recent_session_titles(directory: &str, limit: usize) -> Result<Vec<String>, String> {
    Ok(get_sessions(directory)?
        .into_iter()
        .filter_map(|session| {
            let title = session.title.trim();
            (!title.is_empty()).then(|| title.to_string())
        })
        .take(limit)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DirectoryGroup, Session, SourceCounts, SourceStatus};
    use crate::session_sources::SessionSource;

    fn group(
        path: &str,
        session_count: i64,
        last_active_epoch: i64,
        total_cost: f64,
        source_counts: SourceCounts,
    ) -> DirectoryGroup {
        DirectoryGroup {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            session_count,
            last_active: crate::db::epoch_to_iso(last_active_epoch),
            last_active_epoch,
            summary: None,
            has_agents_md: false,
            tags: Vec::new(),
            pinned: false,
            hidden: false,
            is_missing: false,
            is_git_repo: false,
            is_worktree: false,
            repo_name: None,
            branch_name: None,
            total_cost,
            source_counts,
        }
    }

    #[test]
    fn merge_directory_groups_combines_sources_for_same_path() {
        let opencode = group(
            "/repo",
            2,
            1_000,
            1.25,
            SourceCounts {
                opencode: 2,
                codex: 0,
            },
        );
        let codex = group(
            "/repo",
            3,
            2_000,
            0.0,
            SourceCounts {
                opencode: 0,
                codex: 3,
            },
        );

        let merged = merge_directory_groups(vec![opencode], vec![codex]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].session_count, 5);
        assert_eq!(merged[0].source_counts.opencode, 2);
        assert_eq!(merged[0].source_counts.codex, 3);
        assert_eq!(merged[0].last_active_epoch, 2_000);
        assert_eq!(merged[0].total_cost, 1.25);
    }

    #[test]
    fn merge_sessions_orders_sources_by_updated_time() {
        let session = |source, id: &str, updated: &str| Session {
            source,
            id: id.to_string(),
            directory: "/repo".to_string(),
            title: id.to_string(),
            slug: String::new(),
            agent: source.as_str().to_string(),
            model_name: None,
            cost: None,
            tokens_input: None,
            tokens_output: None,
            tokens_total: None,
            time_created: updated.to_string(),
            time_updated: updated.to_string(),
            time_ago: String::new(),
            parent_id: None,
            file_changes: None,
            first_message: String::new(),
            hidden: false,
            pinned: false,
            note: String::new(),
        };

        let merged = merge_sessions(
            vec![session(
                SessionSource::OpenCode,
                "old",
                "2026-01-01T00:00:00Z",
            )],
            vec![session(SessionSource::Codex, "new", "2026-02-01T00:00:00Z")],
        );

        assert_eq!(merged[0].id, "new");
        assert_eq!(merged[0].source, SessionSource::Codex);
        assert_eq!(merged[1].id, "old");
    }

    #[test]
    fn merge_directory_groups_normalizes_equivalent_existing_paths() {
        let root = std::env::temp_dir().join(format!("opensession-path-{}", uuid::Uuid::new_v4()));
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let alternate = repo.join(".").to_string_lossy().to_string();

        let merged = merge_directory_groups(
            vec![group(
                repo.to_str().unwrap(),
                1,
                1_000,
                0.0,
                SourceCounts {
                    opencode: 1,
                    codex: 0,
                },
            )],
            vec![group(
                &alternate,
                1,
                2_000,
                0.0,
                SourceCounts {
                    opencode: 0,
                    codex: 1,
                },
            )],
        );

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].session_count, 2);
        assert_eq!(
            merged[0].path,
            std::fs::canonicalize(&repo).unwrap().to_string_lossy()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_status_distinguishes_empty_unsupported_and_available_sources() {
        assert_eq!(
            status_from_directory_result(Ok(Vec::new())),
            SourceStatus::NoSessions
        );
        assert_eq!(
            status_from_directory_result(Err("CODEX_DB_UNSUPPORTED_SCHEMA".to_string())),
            SourceStatus::UnsupportedSchema
        );
        assert_eq!(
            status_from_directory_result(Ok(vec![group(
                "/repo",
                1,
                1,
                0.0,
                SourceCounts {
                    opencode: 1,
                    codex: 0
                },
            )])),
            SourceStatus::Available
        );
    }

    #[test]
    fn summary_keys_follow_directory_normalization() {
        let root =
            std::env::temp_dir().join(format!("opensession-summary-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let alias = root.join(".").to_string_lossy().to_string();
        let summary = crate::models::Summary {
            title: "title".into(),
            description: "description".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
        };
        let normalized = normalize_summary_keys(HashMap::from([(alias, summary)]));
        let key = std::fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(normalized.contains_key(&key));
        std::fs::remove_dir_all(root).unwrap();
    }
}
