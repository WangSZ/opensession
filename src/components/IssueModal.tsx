import { useState, useEffect } from "react";
import { Search, Plus, Bug, Loader2, X } from "lucide-react";
import type { Issue, IssueWithSessions } from "../types";

interface Props {
  issues: IssueWithSessions[];
  currentIssueId?: string | null;
  onSelect: (issueId: string) => void;
  onCreate: (title: string) => Promise<string | null>;
  onClose: () => void;
}

export default function IssueModal({ issues, currentIssueId, onSelect, onCreate, onClose }: Props) {
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const [newTitle, setNewTitle] = useState("");

  const filtered = issues.filter(i =>
    !search || i.issue.title.toLowerCase().includes(search.toLowerCase())
  );

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  async function handleCreate() {
    const title = newTitle.trim();
    if (!title) return;
    setCreating(true);
    const id = await onCreate(title);
    setCreating(false);
    if (id) {
      onSelect(id);
      onClose();
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl shadow-black/40 w-[420px] max-h-[500px] flex flex-col"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-surface-border">
          <h3 className="text-sm font-medium">关联 Issue</h3>
          <button onClick={onClose} className="text-gray-500 hover:text-gray-200 transition-colors">
            <X size={16} />
          </button>
        </div>

        <div className="p-3 border-b border-surface-border">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" size={14} />
            <input
              className="w-full bg-surface-hover text-gray-200 rounded-lg pl-8 pr-3 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
              placeholder="搜索 Issue..."
              value={search}
              onChange={e => setSearch(e.target.value)}
            />
          </div>
        </div>

        <div className="flex items-center gap-2 px-3 py-2 border-b border-surface-border">
          <input
            className="flex-1 bg-surface-hover text-gray-200 rounded-lg px-3 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            placeholder="新建 Issue 标题..."
            value={newTitle}
            onChange={e => setNewTitle(e.target.value)}
            onKeyDown={e => { if (e.key === "Enter") handleCreate(); }}
          />
          <button
            onClick={handleCreate}
            disabled={creating || !newTitle.trim()}
            className="flex items-center gap-1 px-2.5 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white rounded-md transition-colors"
          >
            {creating ? <Loader2 size={12} className="animate-spin" /> : <Plus size={12} />}
            创建
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="flex flex-col items-center py-8 text-gray-500">
              <Bug size={24} className="mb-2 opacity-50" />
              <span className="text-xs">暂无 Issue</span>
            </div>
          ) : (
            filtered.map(iws => (
              <div
                key={iws.issue.id}
                className={`flex items-center gap-2 px-4 py-2.5 cursor-pointer transition-colors text-sm ${
                  iws.issue.id === currentIssueId
                    ? "bg-indigo-600/20 text-indigo-300"
                    : "text-gray-200 hover:bg-surface-hover"
                }`}
                onClick={() => { onSelect(iws.issue.id); onClose(); }}
              >
                <Bug size={14} className="text-amber-400 flex-shrink-0" />
                <span className="truncate flex-1">{iws.issue.title}</span>
                <span className="text-xs text-gray-500 flex-shrink-0">{iws.sessions.length} 个会话</span>
                {iws.issue.id === currentIssueId && (
                  <span className="text-xs text-indigo-400 flex-shrink-0">当前</span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
