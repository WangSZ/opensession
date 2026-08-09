export type SessionSource = "opencode" | "codex";

export interface SourceCounts {
  opencode: number;
  codex: number;
}

export type SourceStatus = "available" | "not_installed" | "no_sessions" | "unsupported_schema" | "read_error";

export interface SourceStatuses {
  opencode: SourceStatus;
  codex: SourceStatus;
}

export interface DirectoryGroup {
  path: string;
  name: string;
  session_count: number;
  last_active: string;
  last_active_epoch: number;
  summary: Summary | null;
  has_agents_md: boolean;
  tags: string[];
  pinned: boolean;
  hidden: boolean;
  is_missing?: boolean;
  is_git_repo: boolean;
  is_worktree?: boolean;
  repo_name?: string | null;
  branch_name?: string | null;
  total_cost: number;
  source_counts: SourceCounts;
}

export interface Session {
  source: SessionSource;
  id: string;
  directory: string;
  title: string;
  slug: string;
  agent: string;
  model_name: string | null;
  cost: number | null;
  tokens_input: number | null;
  tokens_output: number | null;
  tokens_total: number | null;
  time_created: string;
  time_updated: string;
  time_ago: string;
  parent_id: string | null;
  file_changes: number | null;
  first_message: string;
  hidden: boolean;
  pinned: boolean;
  note: string;
}

export interface SessionDetail {
  session: Session;
  first_message: string;
  messages_count: number | null;
  model_provider: string | null;
}

export interface Summary {
  title: string;
  description: string;
  generated_at: string;
}

export interface Issue {
  id: string;
  title: string;
  description?: string | null;
  url?: string | null;
  created_at: string;
  updated_at: string;
  priority: string;
  status: string;
  deadline?: string | null;
}

export interface IssueComment {
  id: string;
  issue_id: string;
  content: string;
  created_at: string;
}

export interface LinkedSession {
  session_source: SessionSource;
  session_id: string;
  session_title: string;
  directory: string;
  directory_name: string;
  time_created: string;
  time_ago: string;
  cost: number | null;
}

export interface LinkedDirectory {
  directory: string;
  directory_name: string;
  session_count: number;
}

export interface IssueWithSessions {
  issue: Issue;
  sessions: LinkedSession[];
  directories: LinkedDirectory[];
}
