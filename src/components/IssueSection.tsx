import { useState, useMemo } from "react";
import { Search, Bug, Loader2, EyeOff, Eye } from "lucide-react";
import IssueCard from "./IssueCard";
import type { IssueWithSessions } from "../types";

interface Props {
  issues: IssueWithSessions[];
  loading: boolean;
  selectedIssueId: string | null;
  onSelectIssue: (issue: IssueWithSessions) => void;
  onContextMenu?: (e: React.MouseEvent, issue: IssueWithSessions) => void;
}

const CLOSED_STATUSES = new Set(["closed", "wont_fix"]);

export default function IssueSection({
  issues, loading, selectedIssueId, onSelectIssue, onContextMenu,
}: Props) {
  const [search, setSearch] = useState("");
  const [showClosed, setShowClosed] = useState(false);

  const closedCount = useMemo(
    () => issues.filter(i => CLOSED_STATUSES.has(i.issue.status)).length,
    [issues],
  );

  const filtered = useMemo(() => {
    let list = issues;
    if (!showClosed) {
      list = list.filter(i => !CLOSED_STATUSES.has(i.issue.status));
    }
    const q = search.toLowerCase();
    if (q) {
      list = list.filter(i =>
        i.issue.title.toLowerCase().includes(q) ||
        (i.issue.description?.toLowerCase().includes(q))
      );
    }
    return list;
  }, [issues, search, showClosed]);

  return (
    <>
      <div className="p-3 border-b border-surface-border space-y-2">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" size={14} />
          <input
            className="w-full bg-surface-hover text-gray-200 rounded-lg pl-8 pr-3 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            placeholder="Search issues..."
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
        </div>
        {closedCount > 0 && (
          <button
            onClick={() => setShowClosed(v => !v)}
            className={`flex items-center gap-1 text-xs px-2 py-0.5 rounded-md transition-colors ${
              showClosed
                ? "bg-surface-hover text-gray-400"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {showClosed ? <Eye size={12} /> : <EyeOff size={12} />}
            {showClosed ? "隐藏已关闭" : `显示已关闭 (${closedCount})`}
          </button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-12 text-gray-500">
            <Loader2 className="animate-spin mr-2" size={18} />
            Loading...
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex flex-col items-center py-12 text-gray-500">
            <Bug size={32} className="mb-2 opacity-50" />
            <span className="text-sm">
              {closedCount > 0 && !showClosed
                ? `${closedCount} 个已关闭的 Issue 已隐藏`
                : "No matching issues"}
            </span>
          </div>
        ) : (
          filtered.map(iws => (
            <IssueCard
              key={iws.issue.id}
              issue={iws}
              selected={selectedIssueId === iws.issue.id}
              onSelect={() => onSelectIssue(iws)}
              onContextMenu={onContextMenu}
            />
          ))
        )}
      </div>
    </>
  );
}
