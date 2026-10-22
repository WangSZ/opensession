#[cfg(target_os = "windows")]
use crate::session_sources::windows_command_line;
use crate::session_sources::{
    is_valid_session_id, session_command, shell_command_line, SessionAction, SessionSource,
};

#[tauri::command]
pub fn open_plain_terminal(directory: String) -> Result<(), String> {
    check_directory_exists(&directory)?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Terminal", &directory])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let cmd = format!("cd /d \"{}\"", directory);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = shell_command_line(directory, &["exec".into(), "bash".into()]);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}

fn check_directory_exists(directory: &str) -> Result<(), String> {
    if !std::path::Path::new(directory).exists() {
        return Err("DIRECTORY_NOT_FOUND".into());
    }
    Ok(())
}

fn get_repo_root(directory: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["-C", directory, "rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        return Err("NOT_A_GIT_REPO".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn open_in_terminal(
    directory: String,
    session_source: SessionSource,
    session_id: Option<String>,
) -> Result<(), String> {
    check_directory_exists(&directory)?;
    if let Some(session_id) = session_id.as_deref() {
        validate_session_id(session_id)?;
    }
    let action = if session_id.is_some() {
        SessionAction::Resume
    } else {
        SessionAction::Start
    };
    let args = session_command(session_source, session_id.as_deref(), action);
    launch_cli_in_terminal(&directory, &args, "session")
}

fn launch_cli_in_terminal(directory: &str, args: &[String], prefix: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let script_content = format!(
            "#!/bin/sh\n{}\nrm \"$0\"\n",
            shell_command_line(directory, args)
        );
        let script_path = std::env::temp_dir().join(format!(
            "opensession_{}_{}.sh",
            prefix,
            uuid::Uuid::new_v4()
        ));
        {
            let mut file = std::fs::File::create(&script_path)
                .map_err(|e| format!("Failed to create temp script: {}", e))?;
            file.write_all(script_content.as_bytes())
                .map_err(|e| format!("Failed to write temp script: {}", e))?;
        }
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions on temp script: {}", e))?;
        let script = format!(
            "tell app \"Terminal\"\nactivate\ndo script \"bash \\\"{}\\\"\"\nend tell",
            script_path.display()
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let cmd = windows_command_line(directory, args);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = shell_command_line(directory, args);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_worktree(
    directory: String,
    branch_name: String,
    base: String,
    session_source: SessionSource,
) -> Result<(), String> {
    check_directory_exists(&directory)?;
    let repo_root = get_repo_root(&directory)?;
    let repo_path = std::path::Path::new(&repo_root);
    let repo_name = repo_path
        .file_name()
        .ok_or_else(|| "Invalid repo path".to_string())?
        .to_string_lossy()
        .to_string();

    let parent = repo_path
        .parent()
        .ok_or_else(|| "Invalid repo path".to_string())?;

    let worktree_path = parent
        .join(format!("{}-worktrees", repo_name))
        .join(&branch_name);

    if worktree_path.exists() {
        return Err("WORKTREE_PATH_EXISTS".into());
    }

    std::fs::create_dir_all(worktree_path.parent().unwrap())
        .map_err(|e| format!("Failed to create worktree parent dir: {}", e))?;

    let mut cmd = std::process::Command::new("git");
    cmd.args(["-C", &repo_root, "worktree", "add", "-b", &branch_name]);
    cmd.arg(&worktree_path);
    if base != "HEAD" {
        cmd.arg(&base);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run git worktree add: {}", e))?;

    if !status.success() {
        return Err("Failed to create worktree".into());
    }

    let target = worktree_path.to_string_lossy().to_string();
    crate::session_repository::invalidate_directory_cache();
    let args = session_command(session_source, None, SessionAction::Start);
    launch_cli_in_terminal(&target, &args, "worktree")
}

#[tauri::command]
pub fn list_git_bases(directory: String) -> Result<Vec<String>, String> {
    let root = get_repo_root(&directory)?;

    let output = std::process::Command::new("git")
        .args(["-C", &root, "branch", "--format=%(refname:short)"])
        .output()
        .map_err(|e| format!("Failed to list branches: {}", e))?;

    let mut bases: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let output = std::process::Command::new("git")
        .args(["-C", &root, "tag", "--list"])
        .output()
        .map_err(|e| format!("Failed to list tags: {}", e))?;

    let tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    bases.extend(tags);
    bases.sort();
    bases.insert(0, "HEAD".to_string());

    Ok(bases)
}

#[tauri::command]
pub fn remove_worktree(path: String, force: bool) -> Result<(), String> {
    let path_obj = std::path::Path::new(&path);
    let git_path = path_obj.join(".git");

    // 检查是 worktree（.git 是文件）
    if !git_path.exists() || !git_path.is_file() {
        // 目录已被手动删除 → git worktree prune 清理注册
        if !path_obj.exists() {
            if let Some(parent) = path_obj.parent().and_then(|p| p.to_str()) {
                let _ = std::process::Command::new("git")
                    .args(["-C", parent, "worktree", "prune"])
                    .output();
            }
            super::meta::mark_hidden(&[path.clone()]);
            super::git_info::clear_cache_entry(&path);
            crate::session_repository::invalidate_directory_cache();
            return Ok(());
        }
        return Err("NOT_A_WORKTREE".to_string());
    }

    let repo_root = get_repo_root(&path)?;
    let mut cmd = std::process::Command::new("git");
    cmd.args(["-C", &repo_root, "worktree", "remove"]);
    if force {
        cmd.arg("--force");
    }
    cmd.arg(&path);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run git worktree remove: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !force && (stderr.contains("is dirty") || stderr.contains("has unmerged files")) {
            return Err("WORKTREE_NOT_CLEAN".to_string());
        }
        return Err(format!("WORKTREE_REMOVE_FAILED: {}", stderr.trim()));
    }

    super::meta::mark_hidden(&[path.clone()]);
    super::git_info::clear_cache_entry(&path);
    crate::session_repository::invalidate_directory_cache();
    Ok(())
}

/// OpenCode v2 removed `--fork` from the top-level CLI. Fork via the HTTP API
/// (`opencode api session.fork`) and return resume args for the new session.
fn fork_opencode_v2_session(session_id: &str) -> Result<Vec<String>, String> {
    validate_session_id(session_id)?;

    let output = std::process::Command::new("opencode")
        .args([
            "api",
            "session.fork",
            "--param",
            &format!("sessionID={}", session_id),
            "--data",
            r#"{"before":null}"#,
        ])
        .output()
        .map_err(|e| format!("Failed to run opencode api fork: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "opencode api fork failed: {}",
            stderr.trim().trim_start_matches("Error: ")
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse opencode api fork response: {}", e))?;

    if let Some(error_tag) = parsed.get("_tag").and_then(|v| v.as_str()) {
        return Err(format!(
            "opencode api fork error: {} - {}",
            error_tag,
            parsed
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
        ));
    }

    let new_id = parsed
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| "Fork response missing session id".to_string())?;

    validate_session_id(new_id)?;
    Ok(session_command(
        SessionSource::OpenCode,
        Some(new_id),
        SessionAction::Resume,
    ))
}

#[tauri::command]
pub fn fork_session(
    directory: String,
    session_source: SessionSource,
    session_id: String,
) -> Result<(), String> {
    check_directory_exists(&directory)?;
    validate_session_id(&session_id)?;

    let args = if session_source == SessionSource::OpenCode {
        let conn = crate::db::open_connection()?;
        if crate::db::is_opencode_v2(&conn) {
            fork_opencode_v2_session(&session_id)?
        } else {
            session_command(session_source, Some(&session_id), SessionAction::Fork)
        }
    } else {
        session_command(session_source, Some(&session_id), SessionAction::Fork)
    };

    launch_cli_in_terminal(&directory, &args, "fork")
}

#[tauri::command]
pub fn bootstrap_session(directory: String, session_source: SessionSource) -> Result<(), String> {
    check_directory_exists(&directory)?;
    let args = session_command(session_source, None, SessionAction::Start);
    launch_cli_in_terminal(&directory, &args, "bootstrap")
}

fn validate_session_id(session_id: &str) -> Result<(), String> {
    if is_valid_session_id(session_id) {
        Ok(())
    } else {
        Err("INVALID_SESSION_ID".into())
    }
}
