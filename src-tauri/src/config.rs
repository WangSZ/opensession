use std::path::PathBuf;
use std::sync::OnceLock;

fn resolve_opencode_data_dir() -> PathBuf {
    if let Ok(output) = std::process::Command::new("opencode")
        .args(["db", "path"])
        .output()
    {
        if output.status.success() {
            let db_path =
                PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string());
            if db_path.exists() {
                if let Some(data_dir) = db_path.parent().and_then(|p| p.parent()) {
                    return data_dir.to_path_buf();
                }
            }
        }
    }

    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(dir);
        if p.join("opencode").join("opencode.db").exists() {
            return p;
        }
    }

    if let Some(home) = dirs::home_dir() {
        let p = home.join(".local").join("share");
        if p.join("opencode").join("opencode.db").exists() {
            return p;
        }
    }

    dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn opencode_data_dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(resolve_opencode_data_dir)
}

pub fn opencode_db_path() -> Option<PathBuf> {
    let path = opencode_data_dir().join("opencode").join("opencode.db");
    path.exists().then_some(path)
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir().or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
}

fn xdg_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(dir);
        if p.is_absolute() {
            return p;
        }
    }
    home_dir()
        .map(|h| h.join(".config"))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn xdg_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(dir);
        if p.is_absolute() {
            return p;
        }
    }
    home_dir()
        .map(|h| h.join(".local").join("share"))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn app_config_dir() -> PathBuf {
    xdg_config_dir().join("opensession")
}

pub fn app_data_dir() -> PathBuf {
    xdg_data_dir().join("opensession")
}

pub fn migrate_summaries_if_needed() {
    let old_dir = opencode_data_dir().join("opensession");
    let old_path = old_dir.join("summaries.json");
    if !old_path.exists() {
        return;
    }

    let new_dir = app_data_dir();
    let new_path = new_dir.join("summaries.json");

    if old_path == new_path {
        return;
    }

    if new_path.exists() {
        let _ = std::fs::remove_file(&old_path);
        if old_dir.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false) {
            let _ = std::fs::remove_dir(&old_dir);
        }
        return;
    }

    if std::fs::create_dir_all(&new_dir).is_ok() {
        if std::fs::copy(&old_path, &new_path).is_ok() {
            let _ = std::fs::remove_file(&old_path);
            if old_dir.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false) {
                let _ = std::fs::remove_dir(&old_dir);
            }
        }
    }
}
