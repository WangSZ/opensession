use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub path: String,
    pub name: String,
    pub session_count: i64,
    pub last_active: String,
    pub last_active_epoch: i64,
    pub summary: Option<Summary>,
    pub has_agents_md: bool,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub hidden: bool,
    #[serde(default)]
    pub is_missing: bool,
    #[serde(default)]
    pub is_git_repo: bool,
    #[serde(default)]
    pub is_worktree: bool,
    #[serde(default)]
    pub repo_name: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub agent: String,
    pub model_name: String,
    pub cost: f64,
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub time_created: String,
    pub time_updated: String,
    pub time_ago: String,
    pub parent_id: Option<String>,
    pub file_changes: i64,
    pub first_message: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: Session,
    pub first_message: String,
    pub messages_count: i64,
    pub model_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub title: String,
    pub description: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryMeta {
    pub tags: Vec<String>,
    pub pinned: bool,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMeta {
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub deadline: Option<String>,
}

fn default_priority() -> String { "medium".to_string() }
fn default_status() -> String { "open".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: String,
    pub issue_id: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLink {
    pub session_id: String,
    pub issue_id: String,
    pub directory: String,
    pub linked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedSession {
    pub session_id: String,
    pub session_title: String,
    pub directory: String,
    pub directory_name: String,
    pub time_created: String,
    pub time_ago: String,
    #[serde(default)]
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWithSessions {
    pub issue: Issue,
    pub sessions: Vec<LinkedSession>,
}
