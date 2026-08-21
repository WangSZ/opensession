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

pub fn merge_directory_groups(sources: Vec<Vec<DirectoryGroup>>) -> Vec<DirectoryGroup> {
    let mut groups = HashMap::<String, DirectoryGroup>::new();

    for mut group in sources.into_iter().flatten() {
        let normalized_path = normalize_directory_path(&group.path);
        group.path = normalized_path.clone();
        if let Some(existing) = groups.get_mut(&normalized_path) {
            existing.session_count += group.session_count;
            existing.source_counts.opencode += group.source_counts.opencode;
            existing.source_counts.codex += group.source_counts.codex;
            existing.source_counts.reasonix += group.source_counts.reasonix;
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

pub fn merge_sessions(sources: Vec<Vec<Session>>) -> Vec<Session> {
    let mut sessions: Vec<_> = sources.into_iter().flatten().collect();
    sessions.sort_by(|a, b| b.time_updated.cmp(&a.time_updated));
    sessions
}

fn collect_available<T>(
    results: Vec<(&'static str, Result<Vec<T>, String>)>,
) -> Result<Vec<Vec<T>>, String> {
    let mut available = Vec::new();
    let mut errors = Vec::new();
    for (source, result) in results {
        match result {
            Ok(items) => available.push(items),
            Err(error) => errors.push(format!("{}={}", source, error)),
        }
    }
    if available.is_empty() {
        Err(format!("NO_SESSION_DATA: {}", errors.join("; ")))
    } else {
        Ok(available)
    }
}

pub fn status_from_directory_result(result: Result<Vec<DirectoryGroup>, String>) -> SourceStatus {
    status_from_directory_ref(&result)
}

fn status_from_directory_ref(result: &Result<Vec<DirectoryGroup>, String>) -> SourceStatus {
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
        reasonix: status_from_directory_result(crate::reasonix_db::list_directory_groups()),
    }
}

pub fn list_directory_groups_with_statuses() -> Result<(Vec<DirectoryGroup>, SourceStatuses), String> {
    let opencode = crate::db::list_directory_groups();
    let codex = crate::codex_db::list_directory_groups();
    let reasonix = crate::reasonix_db::list_directory_groups();

    // Derive per-source status from the same results we are about to merge,
    // so `list_directories` + `get_source_statuses` no longer need to query
    // each source twice.
    let statuses = SourceStatuses {
        opencode: status_from_directory_ref(&opencode),
        codex: status_from_directory_ref(&codex),
        reasonix: status_from_directory_ref(&reasonix),
    };

    let mut groups = merge_directory_groups(collect_available(vec![
        ("opencode", opencode),
        ("codex", codex),
        ("reasonix", reasonix),
    ])?);

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
    Ok((groups, statuses))
}

pub fn list_directory_groups() -> Result<Vec<DirectoryGroup>, String> {
    Ok(list_directory_groups_with_statuses()?.0)
}

fn get_source_sessions(
    target: &str,
    list_groups: fn() -> Result<Vec<DirectoryGroup>, String>,
    get_sessions: fn(&str) -> Result<Vec<Session>, String>,
) -> Result<Vec<Session>, String> {
    // Fast path: the source almost always stores the already-canonical
    // directory, so a direct equality query hits without enumerating every
    // directory group. That avoids a full `GROUP BY directory` aggregation,
    // re-reading/normalizing summaries, and stat'ing every directory on disk
    // (and for Reasonix, scanning all transcript files) — work that is wasted
    // when the stored path already matches the normalized target.
    let mut sessions = get_sessions(target)?;

    // Fallback for stored paths that are not byte-identical to the normalized
    // target (symlinks, trailing ".", "..", casing on case-insensitive FS).
    if sessions.is_empty() {
        let alt_paths: Vec<String> = list_groups()?
            .into_iter()
            .filter(|group| normalize_directory_path(&group.path) == target)
            .map(|group| group.path)
            .filter(|path| path != target)
            .collect();
        for path in alt_paths {
            sessions.extend(get_sessions(&path)?);
        }
    }

    Ok(sessions)
}

pub fn get_sessions(directory: &str) -> Result<Vec<Session>, String> {
    let target = normalize_directory_path(directory);

    // Query the three independent read-only sources in parallel so the total
    // latency is the max of the sources rather than their sum (Reasonix file
    // scanning in particular dominates otherwise).
    let (opencode, codex, reasonix) = std::thread::scope(|scope| {
        let oc = scope.spawn(|| {
            get_source_sessions(&target, crate::db::list_directory_groups, crate::db::get_sessions)
        });
        let cx = scope.spawn(|| {
            get_source_sessions(
                &target,
                crate::codex_db::list_directory_groups,
                crate::codex_db::get_sessions,
            )
        });
        let rx = scope.spawn(|| {
            get_source_sessions(
                &target,
                crate::reasonix_db::list_directory_groups,
                crate::reasonix_db::get_sessions,
            )
        });
        (oc.join(), cx.join(), rx.join())
    });

    let opencode = opencode.unwrap_or_else(|e| Err(format!("opencode thread panicked: {e:?}")));
    let codex = codex.unwrap_or_else(|e| Err(format!("codex thread panicked: {e:?}")));
    let reasonix = reasonix.unwrap_or_else(|e| Err(format!("reasonix thread panicked: {e:?}")));

    let mut sessions = merge_sessions(collect_available(vec![
        ("opencode", opencode),
        ("codex", codex),
        ("reasonix", reasonix),
    ])?);
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
        SessionSource::Reasonix => crate::reasonix_db::get_session_detail(session_id),
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
                reasonix: 0,
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
                reasonix: 0,
            },
        );
        let reasonix = group(
            "/repo",
            1,
            3_000,
            0.0,
            SourceCounts {
                opencode: 0,
                codex: 0,
                reasonix: 1,
            },
        );

        let merged = merge_directory_groups(vec![vec![opencode], vec![codex], vec![reasonix]]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].session_count, 6);
        assert_eq!(merged[0].source_counts.opencode, 2);
        assert_eq!(merged[0].source_counts.codex, 3);
        assert_eq!(merged[0].source_counts.reasonix, 1);
        assert_eq!(merged[0].last_active_epoch, 3_000);
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

        let merged = merge_sessions(vec![
            vec![session(
                SessionSource::OpenCode,
                "old",
                "2026-01-01T00:00:00Z",
            )],
            vec![session(SessionSource::Codex, "new", "2026-02-01T00:00:00Z")],
        ]);

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

        let merged = merge_directory_groups(vec![
            vec![group(
                repo.to_str().unwrap(),
                1,
                1_000,
                0.0,
                SourceCounts {
                    opencode: 1,
                    codex: 0,
                    reasonix: 0,
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
                    reasonix: 0,
                },
            )],
        ]);

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
                    codex: 0,
                    reasonix: 0,
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
