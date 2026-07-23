import { useState, useMemo, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Bug, Terminal, Play, FolderOpen, Trash2, Pencil, Send, Clock, AlertTriangle, X, MessageSquare, GitFork, Check, Loader2, Unlink } from "lucide-react";
import type { IssueWithSessions, IssueComment, Session } from "../types";
import ContextMenu from "./ContextMenu";
import type { MenuItem } from "./ContextMenu";

interface Props {
  issue: IssueWithSessions;
  onEdit: () => void;
  onDelete: () => void;
  onOpenSession: (sessionId: string, directory: string) => void;
  onOpenFileManager: (directory: string) => void;
  onOpenInTerminal: (directory: string, sessionId?: string) => void;
  onUnlinkSession: (sessionId: string) => void;
  onOpenDirectory: (directory: string) => void;
  onUnlinkDirectory: (directory: string) => void;
}

const STATUS_LABELS: Record<string, string> = {
  open: "待处理", pending: "待定", in_progress: "进行中",
  resolved: "已解决", closed: "已关闭", wont_fix: "不予处理",
};

function deadlineInfo(deadline?: string | null): { dateStr: string; reminder: string | null; cls: string; show: boolean } {
  if (!deadline) return { dateStr: "", reminder: null, cls: "", show: false };
  const d = new Date(deadline);
  const now = Date.now();
  const diff = d.getTime() - now;
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  const dateStr = `${m}-${day}`;
  if (diff < 0) return { dateStr, reminder: "已逾期", cls: "text-red-400", show: true };
  if (diff < 86400000) return { dateStr, reminder: "即将截止", cls: "text-red-400", show: true };
  if (diff < 259200000) return { dateStr, reminder: "临近截止", cls: "text-amber-400", show: true };
  return { dateStr, reminder: null, cls: "text-gray-300", show: true };
}

export default function IssueDetail({ issue, onEdit, onDelete, onOpenSession, onOpenFileManager, onOpenInTerminal, onUnlinkSession, onOpenDirectory, onUnlinkDirectory }: Props) {
  const [comments, setComments] = useState<IssueComment[] | null>(null);
  const [loadingComments, setLoadingComments] = useState(false);
  const [commentText, setCommentText] = useState("");
  const [sending, setSending] = useState(false);
  const [forkingId, setForkingId] = useState<string | null>(null);
  const [forkSuccess, setForkSuccess] = useState<Record<string, boolean>>({});
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const [sessionCtx, setSessionCtx] = useState<{ sessionId: string; x: number; y: number } | null>(null);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set(issue.directories.map(d => d.directory)));
  const [dirSessions, setDirSessions] = useState<Record<string, Session[]>>({});
  const [loadingDirSessions, setLoadingDirSessions] = useState<Set<string>>(new Set());

  const grouped = useMemo(() => {
    const dirLinked = new Set(issue.directories.map(d => d.directory));
    const map = new Map<string, typeof issue.sessions>();
    for (const s of issue.sessions) {
      if (dirLinked.has(s.directory)) continue;
      const existing = map.get(s.directory);
      if (existing) existing.push(s);
      else map.set(s.directory, [s]);
    }
    return Array.from(map.entries());
  }, [issue.sessions, issue.directories]);

  const dl = deadlineInfo(issue.issue.deadline);

  async function loadComments() {
    if (comments !== null) return;
    setLoadingComments(true);
    try {
      const result = await invoke<IssueComment[]>("get_issue_comments", { issueId: issue.issue.id });
      setComments(result);
    } catch (e) {
      console.error("Load comments error:", e);
    }
    setLoadingComments(false);
  }

  useEffect(() => { loadComments(); }, []);

  useEffect(() => {
    for (const d of issue.directories) {
      if (!dirSessions[d.directory]) {
        setLoadingDirSessions(prev => { const n = new Set(prev); n.add(d.directory); return n; });
        invoke<Session[]>("get_sessions", { directory: d.directory })
          .then(sessions => {
            setDirSessions(prev => ({ ...prev, [d.directory]: sessions }));
          })
          .catch(e => console.error("Load directory sessions error:", e))
          .finally(() => {
            setLoadingDirSessions(prev => { const n = new Set(prev); n.delete(d.directory); return n; });
          });
      }
    }
  }, [issue.issue.id]);

  async function handleSend() {
    const text = commentText.trim();
    if (!text) return;
    setSending(true);
    try {
      const newComment = await invoke<IssueComment>("add_comment", { issueId: issue.issue.id, content: text });
      setComments(prev => prev ? [...prev, newComment] : [newComment]);
      setCommentText("");
    } catch (e) {
      console.error("Send comment error:", e);
    }
    setSending(false);
    inputRef.current?.focus();
  }

  async function handleDeleteComment(commentId: string) {
    try {
      await invoke<void>("delete_comment", { issueId: issue.issue.id, commentId });
      setComments(prev => prev ? prev.filter(c => c.id !== commentId) : []);
    } catch (e) {
      console.error("Delete comment error:", e);
    }
  }

  async function toggleDir(directory: string) {
    if (expandedDirs.has(directory)) {
      setExpandedDirs(prev => { const n = new Set(prev); n.delete(directory); return n; });
      return;
    }
    setExpandedDirs(prev => { const n = new Set(prev); n.add(directory); return n; });
    if (!dirSessions[directory]) {
      setLoadingDirSessions(prev => { const n = new Set(prev); n.add(directory); return n; });
      try {
        const sessions = await invoke<Session[]>("get_sessions", { directory });
        setDirSessions(prev => ({ ...prev, [directory]: sessions }));
      } catch (e) {
        console.error("Load directory sessions error:", e);
      }
      setLoadingDirSessions(prev => { const n = new Set(prev); n.delete(directory); return n; });
    }
  }

  async function handleFork(sessionId: string, directory: string) {
    setForkingId(sessionId);
    try {
      await invoke<void>("fork_session", { directory, sessionId });

      const forkStartTime = Date.now();
      while (true) {
        try {
          const sessions = await invoke<Session[]>("get_sessions", { directory });
          const match = sessions.find(s =>
            s.parent_id === sessionId ||
            (new Date(s.time_created).getTime() > forkStartTime && s.id !== sessionId)
          );
          if (match) {
            await invoke<void>("link_session_to_issue", { sessionId: match.id, issueId: issue.issue.id, directory });
            setForkSuccess(prev => ({ ...prev, [sessionId]: true }));
            return;
          }
        } catch (e) {
          console.error("Fork polling error:", e);
        }
        if (Date.now() - forkStartTime > 30000) break;
        await new Promise(r => setTimeout(r, 1000));
      }
    } catch (e) {
      console.error("Fork session error:", e);
    }
    setForkingId(null);
  }

  return (
    <div className="flex-1 flex flex-col min-w-0">
      <div className="px-5 py-3 border-b border-surface-border flex items-center gap-3">
        <Bug size={16} className="text-amber-400 flex-shrink-0" />
        <h2 className="text-sm font-semibold truncate flex-1">{issue.issue.title}</h2>
        <span className={`text-xs px-1.5 py-0.5 rounded whitespace-nowrap ${
          issue.issue.status === "in_progress" ? "text-blue-400 bg-blue-500/15" :
          issue.issue.status === "resolved" ? "text-green-400 bg-green-500/15" :
          issue.issue.status === "closed" ? "text-gray-400 bg-gray-500/15 line-through" :
          issue.issue.status === "wont_fix" ? "text-red-400/70 bg-red-500/10" :
          issue.issue.status === "pending" ? "text-amber-400 bg-amber-500/15" :
          "text-gray-400 bg-surface-hover"
        }`}>{STATUS_LABELS[issue.issue.status] || issue.issue.status}</span>
        <span className="text-xs text-gray-400 whitespace-nowrap">{issue.sessions.length} 个会话</span>
        {dl.show && (
          <span className={`text-xs whitespace-nowrap flex items-center gap-0.5 ${dl.cls}`}>
            {dl.reminder ? <AlertTriangle size={10} /> : <Clock size={10} />}
            {dl.reminder ? `${dl.reminder} ` : "截止 "}{dl.dateStr}
          </span>
        )}
        <button onClick={onEdit} className="text-gray-400 hover:text-white transition-colors p-0.5" title="编辑">
          <Pencil size={13} />
        </button>
        <button onClick={onDelete} className="text-gray-400 hover:text-red-400 transition-colors p-0.5" title="删除">
          <Trash2 size={13} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
        {issue.issue.description && (
          <p className="text-sm text-gray-200 leading-relaxed whitespace-pre-wrap">{issue.issue.description}</p>
        )}

        {grouped.length === 0 && issue.directories.length === 0 ? (
          <p className="text-sm text-gray-400">暂未关联会话</p>
        ) : (
          <div className="space-y-3">
            {issue.directories.length > 0 && (
              <div>
                <h3 className="text-xs text-gray-400 font-medium mb-2">关联的目录</h3>
                {issue.directories.map(d => {
                  const isExpanded = expandedDirs.has(d.directory);
                  const isLoading = loadingDirSessions.has(d.directory);
                  const sessions = dirSessions[d.directory] || [];
                  return (
                    <div key={d.directory} className="bg-surface-card rounded-lg border border-surface-border overflow-hidden mb-1">
                      <div className="group flex items-center gap-2 px-3 py-2 text-xs text-gray-300 hover:bg-surface-hover cursor-pointer transition-colors"
                        onClick={() => toggleDir(d.directory)}
                      >
                        <FolderOpen size={12} className="text-amber-400 flex-shrink-0" />
                        <span className="truncate flex-1">{d.directory_name}</span>
                        <span className="text-gray-500 flex-shrink-0">{d.session_count} 个会话</span>
                        <span className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                          <button onClick={e => { e.stopPropagation(); onOpenFileManager(d.directory); }}
                            className="text-gray-400 hover:text-white transition-colors p-0.5" title="在 Finder 中打开"
                          >
                            <FolderOpen size={11} />
                          </button>
                          <button onClick={e => { e.stopPropagation(); onOpenInTerminal(d.directory); }}
                            className="text-gray-400 hover:text-white transition-colors p-0.5" title="打开 OpenCode"
                          >
                            <Terminal size={11} />
                          </button>
                          <button onClick={e => { e.stopPropagation(); onUnlinkDirectory(d.directory); }}
                            className="text-gray-400 hover:text-red-400 transition-colors p-0.5" title="取消关联目录"
                          >
                            <Unlink size={11} />
                          </button>
                        </span>
                      </div>
                      {isExpanded && (
                        <div className="border-t border-surface-border">
                          {isLoading ? (
                            <div className="flex items-center justify-center py-4 text-gray-500">
                              <Loader2 size={14} className="animate-spin" />
                            </div>
                          ) : sessions.length === 0 ? (
                            <div className="px-3 py-3 text-xs text-gray-500">暂无会话</div>
                          ) : (
                            sessions.map(s => (
                              <div key={s.id}
                                className="flex items-center gap-2 px-3 py-2 text-xs text-gray-300 hover:bg-surface-hover cursor-pointer transition-colors border-t border-surface-border"
                                onClick={() => onOpenSession(s.id, d.directory)}
                              >
                                <Play size={10} className="text-indigo-400 flex-shrink-0" />
                                <span className="truncate flex-1">{s.title || s.slug}</span>
                                {s.cost > 0 && (
                                  <span className="text-xs text-emerald-400 flex-shrink-0">${s.cost.toFixed(3)}</span>
                                )}
                                <span className="text-gray-500 flex-shrink-0">{s.time_ago}</span>
                              </div>
                            ))
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          {grouped.length > 0 && (
            <div>
              <h3 className="text-xs text-gray-400 font-medium mb-2">关联的会话</h3>
            {grouped.map(([dir, sessions]) => (
              <div key={dir} className="bg-surface-card rounded-lg border border-surface-border overflow-hidden">
                <div className="flex items-center gap-1.5 px-3 py-2 bg-surface-hover text-xs text-gray-300">
                  <FolderOpen size={12} className="flex-shrink-0" />
                  <span className="truncate flex-1">{sessions[0].directory_name}</span>
                  <button onClick={() => onOpenFileManager(dir)}
                    className="text-gray-500 hover:text-gray-200 transition-colors p-0.5" title="在 Finder 中打开"
                  >
                    <FolderOpen size={11} />
                  </button>
                  <button onClick={() => onOpenInTerminal(dir)}
                    className="text-gray-500 hover:text-gray-200 transition-colors p-0.5" title="打开 OpenCode"
                  >
                    <Terminal size={11} />
                  </button>
                </div>
                {sessions.map(s => {
                  const isForking = forkingId === s.session_id;
                  const isSuccess = forkSuccess[s.session_id];
                  return (
                    <div key={s.session_id}
                      className="group flex items-center gap-2 px-3 py-2 text-xs text-gray-300 hover:bg-surface-hover cursor-pointer transition-colors border-t border-surface-border"
                      onClick={() => onOpenSession(s.session_id, s.directory)}
                      onContextMenu={e => {
                        e.preventDefault();
                        e.stopPropagation();
                        setSessionCtx({ sessionId: s.session_id, x: e.clientX, y: e.clientY });
                      }}
                    >
                      <Play size={10} className="text-indigo-400 flex-shrink-0" />
                      <span className="truncate flex-1">{s.session_title || "Untitled"}</span>
                      {s.cost > 0 && (
                        <span className="text-xs text-emerald-400 flex-shrink-0">${s.cost.toFixed(3)}</span>
                      )}
                      <span className="text-gray-500 flex-shrink-0">{s.time_ago}</span>
                      <span className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                        <button onClick={e => { e.stopPropagation(); onOpenFileManager(s.directory); }}
                          className="text-gray-400 hover:text-white transition-colors p-0.5" title="在 Finder 中打开"
                        >
                          <FolderOpen size={11} />
                        </button>
                        <button onClick={e => { e.stopPropagation(); onOpenInTerminal(s.directory, s.session_id); }}
                          className="text-gray-400 hover:text-white transition-colors p-0.5" title="打开 OpenCode"
                        >
                          <Terminal size={11} />
                        </button>
                        {isSuccess ? (
                          <span className="text-green-400 p-0.5" title="已 Fork 并关联到此 Issue">
                            <Check size={11} />
                          </span>
                        ) : (
                          <button onClick={e => { e.stopPropagation(); handleFork(s.session_id, s.directory); }}
                            disabled={!!isForking}
                            className="text-gray-400 hover:text-indigo-300 disabled:text-gray-600 transition-colors p-0.5" title="Fork 新会话并关联到此 Issue"
                          >
                            {isForking ? <Loader2 size={11} className="animate-spin" /> : <GitFork size={11} />}
                          </button>
                        )}
                      </span>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
            )}
          </div>
        )}

        <div className="border-t border-surface-border pt-4">
          <div className="flex items-center gap-2 mb-3">
            <MessageSquare size={14} className="text-gray-400" />
            <span className="text-xs text-gray-400">
              评论 ({loadingComments ? "..." : comments?.length ?? 0})
            </span>
          </div>

          {comments && comments.length > 0 && (
            <div className="space-y-2 mb-3">
              {comments.map(c => (
                <div key={c.id} className="bg-surface-card rounded-lg border border-surface-border p-3">
                  <div className="flex items-center justify-between text-gray-400 mb-1">
                    <span className="text-xs text-gray-500">
                      {new Date(c.created_at).toLocaleString("zh-CN", {
                        year: "numeric", month: "short", day: "numeric",
                        hour: "2-digit", minute: "2-digit",
                      })}
                    </span>
                    <button onClick={() => handleDeleteComment(c.id)}
                      className="text-gray-500 hover:text-red-400 transition-colors p-0.5" title="删除评论"
                    >
                      <X size={11} />
                    </button>
                  </div>
                  <p className="text-sm text-gray-200 whitespace-pre-wrap break-words">{c.content}</p>
                </div>
              ))}
            </div>
          )}

          <div className="flex items-start gap-2">
            <textarea ref={inputRef}
              className="flex-1 bg-surface-card text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors resize-none"
              placeholder="添加评论..."
              value={commentText}
              onChange={e => setCommentText(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSend(); } }}
              rows={2}
            />
            <button onClick={handleSend} disabled={sending || !commentText.trim()}
              className="flex items-center gap-1 px-3 py-2 text-xs bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white rounded-md transition-colors"
            >
              <Send size={12} />
            </button>
          </div>
        </div>
      </div>
      {sessionCtx && (
        <ContextMenu
          x={sessionCtx.x}
          y={sessionCtx.y}
          items={[
            {
              type: "item",
              label: "取消关联 Issue",
              icon: <Unlink size={14} />,
              danger: true,
              onClick: () => onUnlinkSession(sessionCtx.sessionId),
            },
          ]}
          onClose={() => setSessionCtx(null)}
        />
      )}
    </div>
  );
}
