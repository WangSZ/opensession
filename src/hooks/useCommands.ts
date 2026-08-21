import { invoke } from "@tauri-apps/api/core";
import type { DirectoryListResponse, Session, SessionDetail, SessionSource, SourceStatuses, Summary, Issue, IssueWithSessions, IssueComment } from "../types";

export function useCommands(onAfterCommand?: () => void) {
  async function wrap<T>(fn: () => Promise<T>): Promise<T> {
    const result = await fn();
    onAfterCommand?.();
    return result;
  }

  return {
    listDirectories: () => invoke<DirectoryListResponse>("list_directories"),
    getSourceStatuses: () => invoke<SourceStatuses>("get_source_statuses"),
    getSessions: (directory: string) => invoke<Session[]>("get_sessions", { directory }),
    getSessionDetail: (sessionSource: SessionSource, sessionId: string) => invoke<SessionDetail>("get_session_detail", { sessionSource, sessionId }),
    generateSummary: (directory: string) => invoke<void>("generate_summary", { directory }),
    setCachedSummary: (directory: string, summary: Summary) =>
      wrap(() => invoke<void>("set_cached_summary", { directory, summary })),
    deleteCachedSummary: (directory: string) =>
      wrap(() => invoke<void>("delete_cached_summary", { directory })),
    openInTerminal: (directory: string, sessionSource: SessionSource, sessionId?: string) =>
      invoke<void>("open_in_terminal", { directory, sessionSource, sessionId: sessionId ?? null }),
    openPlainTerminal: (directory: string) =>
      invoke<void>("open_plain_terminal", { directory }),
    forkSession: (directory: string, sessionSource: SessionSource, sessionId: string) =>
      invoke<void>("fork_session", { directory, sessionSource, sessionId }),
    bootstrapSession: (directory: string, sessionSource: SessionSource) =>
      wrap(() => invoke<void>("bootstrap_session", { directory, sessionSource })),
    renameDirectoryMeta: (old: string, new_: string) =>
      wrap(() => invoke<void>("rename_directory_meta", { old, new: new_ })),
    setDirectoryTags: (path: string, tags: string[]) =>
      wrap(() => invoke<void>("set_directory_tags", { path, tags })),
    deleteTagGlobal: (tag: string) =>
      wrap(() => invoke<void>("delete_tag_global", { tag })),
    togglePin: (path: string) =>
      wrap(() => invoke<boolean>("toggle_directory_pin", { path })),
    openInFileManager: (directory: string) =>
      wrap(() => invoke<void>("open_in_file_manager", { directory })),
    openInVSCode: (directory: string) =>
      wrap(() => invoke<void>("open_in_vscode", { directory })),
    openInJetBrains: (directory: string) =>
      wrap(() => invoke<void>("open_in_jetbrains", { directory })),
    toggleSessionHidden: (sessionSource: SessionSource, sessionId: string) =>
      wrap(() => invoke<boolean>("toggle_session_hidden", { sessionSource, sessionId })),
    toggleSessionPin: (sessionSource: SessionSource, sessionId: string) =>
      wrap(() => invoke<boolean>("toggle_session_pin", { sessionSource, sessionId })),
    setSessionNote: (sessionSource: SessionSource, sessionId: string, note: string) =>
      wrap(() => invoke<void>("set_session_note", { sessionSource, sessionId, note })),
    toggleHidden: (path: string) =>
      wrap(() => invoke<boolean>("toggle_directory_hidden", { path })),
    openWorktree: (directory: string, branchName: string, base: string, sessionSource: SessionSource) =>
      invoke<void>("open_worktree", { directory, branchName, base, sessionSource }),
    listGitBases: (directory: string) =>
      invoke<string[]>("list_git_bases", { directory }),
    removeWorktree: (path: string, force: boolean) =>
      wrap(() => invoke<void>("remove_worktree", { path, force })),
    createIssue: (title: string, description?: string | null, url?: string | null, priority?: string | null, status?: string | null, deadline?: string | null) =>
      wrap(() => invoke<Issue>("create_issue", { title, description: description ?? null, url: url ?? null, priority: priority ?? null, status: status ?? null, deadline: deadline ?? null })),
    updateIssue: (id: string, title?: string | null, description?: string | null, url?: string | null, priority?: string | null, status?: string | null, deadline?: string | null) =>
      wrap(() => invoke<Issue>("update_issue", { id, title: title ?? null, description: description ?? null, url: url ?? null, priority: priority ?? null, status: status ?? null, deadline: deadline ?? null })),
    deleteIssue: (id: string) =>
      wrap(() => invoke<void>("delete_issue", { id })),
    linkSessionToIssue: (sessionSource: SessionSource, sessionId: string, issueId: string, directory: string) =>
      wrap(() => invoke<void>("link_session_to_issue", { sessionSource, sessionId, issueId, directory })),
    unlinkSessionFromIssue: (sessionSource: SessionSource, sessionId: string) =>
      wrap(() => invoke<void>("unlink_session_from_issue", { sessionSource, sessionId })),
    getSessionIssue: (sessionSource: SessionSource, sessionId: string) =>
      wrap(() => invoke<Issue | null>("get_session_issue", { sessionSource, sessionId })),
    getAllIssues: () =>
      invoke<IssueWithSessions[]>("get_all_issues"),
    addComment: (issueId: string, content: string) =>
      wrap(() => invoke<IssueComment>("add_comment", { issueId, content })),
    deleteComment: (issueId: string, commentId: string) =>
      wrap(() => invoke<void>("delete_comment", { issueId, commentId })),
    getIssueComments: (issueId: string) =>
      invoke<IssueComment[]>("get_issue_comments", { issueId }),
    linkDirectoryToIssue: (directory: string, issueId: string) =>
      wrap(() => invoke<void>("link_directory_to_issue", { directory, issueId })),
    unlinkDirectoryFromIssue: (directory: string) =>
      wrap(() => invoke<void>("unlink_directory_from_issue", { directory })),
    getDirectoryIssue: (directory: string) =>
      wrap(() => invoke<Issue | null>("get_directory_issue", { directory })),
  };
}
