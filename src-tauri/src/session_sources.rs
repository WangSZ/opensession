use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    OpenCode,
    Codex,
}

impl Default for SessionSource {
    fn default() -> Self {
        Self::OpenCode
    }
}

impl SessionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionRef {
    pub source: SessionSource,
    pub id: String,
}

impl SessionRef {
    pub fn new(source: SessionSource, id: impl Into<String>) -> Self {
        Self {
            source,
            id: id.into(),
        }
    }

    pub fn storage_key(&self) -> String {
        format!("{}:{}", self.source.as_str(), self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    Start,
    Resume,
    Fork,
}

pub fn session_command(
    source: SessionSource,
    session_id: Option<&str>,
    action: SessionAction,
) -> Vec<String> {
    let id = session_id.unwrap_or_default();
    match (source, action) {
        (SessionSource::OpenCode, SessionAction::Start) => vec!["opencode".into()],
        (SessionSource::Codex, SessionAction::Start) => vec!["codex".into()],
        (SessionSource::OpenCode, SessionAction::Resume) => {
            vec!["opencode".into(), "--session".into(), id.into()]
        }
        (SessionSource::OpenCode, SessionAction::Fork) => vec![
            "opencode".into(),
            "--session".into(),
            id.into(),
            "--fork".into(),
        ],
        (SessionSource::Codex, SessionAction::Resume) => {
            vec!["codex".into(), "resume".into(), id.into()]
        }
        (SessionSource::Codex, SessionAction::Fork) => {
            vec!["codex".into(), "fork".into(), id.into()]
        }
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn shell_command_line(directory: &str, args: &[String]) -> String {
    let command = args
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");
    format!("cd {} && {}", shell_quote(directory), command)
}

pub fn windows_command_line(directory: &str, args: &[String]) -> String {
    let command = args
        .iter()
        .map(|arg| format!("\"{}\"", arg))
        .collect::<Vec<_>>()
        .join(" ");
    format!("cd /d \"{}\" && {}", directory, command)
}

pub fn is_valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub fn is_resumable_codex_source(source: &str) -> bool {
    matches!(source, "cli" | "vscode" | "appServer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ref_uses_source_scoped_storage_key() {
        let session = SessionRef::new(SessionSource::Codex, "019f-test");

        assert_eq!(session.storage_key(), "codex:019f-test");
    }

    #[test]
    fn session_commands_match_the_owning_cli() {
        assert_eq!(
            session_command(SessionSource::OpenCode, None, SessionAction::Start),
            vec!["opencode"]
        );
        assert_eq!(
            session_command(SessionSource::Codex, None, SessionAction::Start),
            vec!["codex"]
        );
        assert_eq!(
            session_command(
                SessionSource::OpenCode,
                Some("ses_123"),
                SessionAction::Resume
            ),
            vec!["opencode", "--session", "ses_123"]
        );
        assert_eq!(
            session_command(
                SessionSource::OpenCode,
                Some("ses_123"),
                SessionAction::Fork
            ),
            vec!["opencode", "--session", "ses_123", "--fork"]
        );
        assert_eq!(
            session_command(
                SessionSource::Codex,
                Some("019f-test"),
                SessionAction::Resume
            ),
            vec!["codex", "resume", "019f-test"]
        );
        assert_eq!(
            session_command(SessionSource::Codex, Some("019f-test"), SessionAction::Fork),
            vec!["codex", "fork", "019f-test"]
        );
    }

    #[test]
    fn codex_filter_accepts_only_interactive_sources() {
        assert!(is_resumable_codex_source("cli"));
        assert!(is_resumable_codex_source("vscode"));
        assert!(is_resumable_codex_source("appServer"));
        assert!(!is_resumable_codex_source("exec"));
        assert!(!is_resumable_codex_source("subAgent"));
        assert!(!is_resumable_codex_source("subAgentReview"));
    }

    #[test]
    fn shell_command_quotes_directory_and_arguments() {
        let command = shell_command_line(
            "/tmp/project it's",
            &[
                "codex".to_string(),
                "resume".to_string(),
                "019f-test".to_string(),
            ],
        );

        assert_eq!(
            command,
            "cd '/tmp/project it'\"'\"'s' && 'codex' 'resume' '019f-test'"
        );
    }

    #[test]
    fn session_ids_reject_shell_metacharacters() {
        assert!(is_valid_session_id("019fe517-60ca-7bf1-93c5-913b12325d69"));
        assert!(is_valid_session_id("ses_abc-123"));
        assert!(!is_valid_session_id("id; rm -rf /"));
        assert!(!is_valid_session_id("id with spaces"));
    }

    #[test]
    fn windows_command_quotes_each_argument() {
        let command = windows_command_line(
            r"C:\Work & Code",
            &[
                "codex".to_string(),
                "resume".to_string(),
                "019f-test".to_string(),
            ],
        );

        assert_eq!(
            command,
            r#"cd /d "C:\Work & Code" && "codex" "resume" "019f-test""#
        );
    }
}
