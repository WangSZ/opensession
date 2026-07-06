import { Bug, Clock, AlertTriangle, DollarSign } from "lucide-react";
import type { IssueWithSessions } from "../types";

interface Props {
  issue: IssueWithSessions;
  selected?: boolean;
  onSelect: () => void;
  onContextMenu?: (e: React.MouseEvent, issue: IssueWithSessions) => void;
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
  return { dateStr, reminder: null, cls: "text-gray-400", show: true };
}

export default function IssueCard({ issue, selected, onSelect, onContextMenu }: Props) {
  const dl = deadlineInfo(issue.issue.deadline);
  const totalCost = issue.sessions.reduce((sum, s) => sum + s.cost, 0);

  const priorityDot = issue.issue.priority === "high" ? "bg-red-500" : issue.issue.priority === "low" ? "bg-green-500" : "bg-amber-500";

  return (
    <div
      className={`px-3 py-2 cursor-pointer border-l-2 transition-colors ${
        selected
          ? "bg-surface-hover border-l-indigo-500"
          : "border-l-transparent hover:bg-surface-hover"
      }`}
      onClick={onSelect}
      onContextMenu={e => { e.preventDefault(); onContextMenu?.(e, issue); }}
    >
      <div className="flex items-center gap-1.5">
        <span className={`w-2 h-2 rounded-full flex-shrink-0 self-start mt-1 ${priorityDot}`} />
        <Bug size={14} className="text-amber-400 flex-shrink-0 self-start mt-0.5" />
        <span className="text-sm font-medium truncate flex-1">{issue.issue.title}</span>
      </div>
      <div className="flex items-center gap-1.5 ml-[22px] mt-1">
        <span className={`text-xs px-1.5 py-0.5 rounded whitespace-nowrap ${
          issue.issue.status === "in_progress" ? "text-blue-400 bg-blue-500/15" :
          issue.issue.status === "resolved" ? "text-green-400 bg-green-500/15" :
          issue.issue.status === "closed" ? "text-gray-500 bg-gray-500/15 line-through" :
          issue.issue.status === "wont_fix" ? "text-red-400/70 bg-red-500/10" :
          issue.issue.status === "pending" ? "text-amber-400 bg-amber-500/15" :
          "text-gray-400 bg-surface-hover"
        }`}>{STATUS_LABELS[issue.issue.status] || issue.issue.status}</span>
        {totalCost > 0 && (
          <span className="text-xs text-emerald-400 bg-emerald-500/10 rounded px-1.5 py-0.5 whitespace-nowrap flex items-center gap-0.5">
            <DollarSign size={9} />
            {totalCost.toFixed(2)}
          </span>
        )}
        {dl.show && (
          <span className={`text-xs whitespace-nowrap flex items-center gap-0.5 ${dl.cls}`}>
            {dl.reminder ? <AlertTriangle size={10} /> : <Clock size={10} />}
            {dl.reminder ? `${dl.reminder} ` : "截止 "}{dl.dateStr}
          </span>
        )}
      </div>
    </div>
  );
}
