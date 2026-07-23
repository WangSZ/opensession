#[tauri::command]
pub fn open_plain_terminal(directory: String) -> Result<(), String> {
    check_directory_exists(&directory)?;

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell app \"Terminal\"\nactivate\ndo script \"cd \\\"{}\\\"\"\nend tell",
            directory
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
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
        let cmd = format!("cd \"{}\" && exec bash", directory);
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
pub fn open_in_terminal(directory: String, session_id: Option<String>) -> Result<(), String> {
    check_directory_exists(&directory)?;

    let session_arg = session_id
        .as_ref()
        .map(|s| format!("--session {}", s))
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let script_content = format!(
            "#!/bin/sh\ncd \"{}\" && opencode {}\nrm \"$0\"\n",
            directory, session_arg
        );

        let script_path = std::env::temp_dir()
            .join(format!("opencode_launch_{}.sh", std::process::id()));

        {
            let mut f = std::fs::File::create(&script_path)
                .map_err(|e| format!("Failed to create temp script: {}", e))?;
            f.write_all(script_content.as_bytes())
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
        let cmd = format!("cd /d \"{}\" && opencode {}", directory, session_arg);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = format!("cd \"{}\" && opencode {}", directory, session_arg);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_worktree(directory: String, branch_name: String, base: String) -> Result<(), String> {
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

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let script_content = format!(
            "#!/bin/sh\ncd \"{}\" && opencode\nrm \"$0\"\n",
            target
        );

        let script_path = std::env::temp_dir()
            .join(format!("opencode_worktree_{}.sh", std::process::id()));

        {
            let mut f = std::fs::File::create(&script_path)
                .map_err(|e| format!("Failed to create temp script: {}", e))?;
            f.write_all(script_content.as_bytes())
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
        let cmd = format!("cd /d \"{}\" && opencode", target);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = format!("cd \"{}\" && opencode", target);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
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
    Ok(())
}

#[tauri::command]
pub fn fork_session(directory: String, session_id: String) -> Result<(), String> {
    check_directory_exists(&directory)?;

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let script_content = format!(
            "#!/bin/sh\ncd \"{}\" && opencode --session {} --fork\nrm \"$0\"\n",
            directory, session_id
        );

        let script_path = std::env::temp_dir()
            .join(format!("opencode_fork_{}.sh", std::process::id()));

        {
            let mut f = std::fs::File::create(&script_path)
                .map_err(|e| format!("Failed to create temp script: {}", e))?;
            f.write_all(script_content.as_bytes())
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
        let cmd = format!("cd /d \"{}\" && opencode --session {} --fork", directory, session_id);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = format!("cd \"{}\" && opencode --session {} --fork", directory, session_id);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn bootstrap_session(directory: String) -> Result<(), String> {
    check_directory_exists(&directory)?;

    let script_content = format!(
        "#!/bin/sh\ncd \"{}\" && opencode run \"hi\"\necho \"\"\necho \"--- Session created ---\"\nrm \"$0\"\n",
        directory
    );

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let script_path = std::env::temp_dir()
            .join(format!("opencode_bootstrap_{}.sh", std::process::id()));

        {
            let mut f = std::fs::File::create(&script_path)
                .map_err(|e| format!("Failed to create temp script: {}", e))?;
            f.write_all(script_content.as_bytes())
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
        let cmd = format!("cd /d \"{}\" && opencode run \"hi\"", directory);
        std::process::Command::new("cmd")
            .args(["/c", "start", "wt", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let cmd = format!("cd \"{}\" && opencode run \"hi\"", directory);
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}
