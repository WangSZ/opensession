import { Play, Info, FileCode, EyeOff, Bug } from "lucide-react";
import CopyButton from "./CopyButton";
import type { Session, Issue } from "../types";

interface Props {
  session: Session;
  directory: string;
  onResume: () => void;
  onDetail: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  issue?: Issue | null;
  highlighted?: boolean;
}

export default function SessionCard({ session, directory, onResume, onDetail, onContextMenu, issue, highlighted }: Props) {
  return (
    <div
      className={`mx-4 my-1.5 bg-surface-card rounded-lg border p-3.5 transition-all hover:-translate-y-0.5 hover:shadow-lg ${
        highlighted
          ? "border-amber-500/60 shadow-lg shadow-amber-500/10"
          : "border-surface-border hover:border-gray-600"
      } ${session.hidden ? "opacity-50" : ""}`}
      onContextMenu={onContextMenu}
    >
      <div className="flex items-start justify-between mb-1.5">
        <h3 className="font-medium text-sm truncate flex-1 min-w-0 mr-2">
          {session.title || session.slug}
        </h3>
        <span className="text-xs text-gray-500 flex-shrink-0 whitespace-nowrap flex items-center gap-1">
          {session.hidden && <EyeOff size={11} className="text-gray-600" />}
          {issue && (
            <span className="flex items-center gap-0.5 text-amber-400/80 mr-1" title={issue.title}>
              <Bug size={11} />
              <span className="max-w-[100px] truncate hidden sm:inline">{issue.title}</span>
            </span>
          )}
          {session.time_ago}
        </span>
      </div>

      <div className="flex items-center gap-2 text-xs text-gray-400 mb-2">
        <span className="bg-surface-hover px-1.5 py-0.5 rounded">{session.agent}</span>
        <span>·</span>
        <span className="truncate max-w-[200px]">{session.model_name}</span>
        <span>·</span>
        <span>${session.cost.toFixed(3)}</span>
        {session.file_changes > 0 && (
          <>
            <span>·</span>
            <FileCode size={12} className="text-green-500" />
            <span className="text-green-400">{session.file_changes} files</span>
          </>
        )}
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3 text-xs text-gray-500">
          <span>↗ {formatNum(session.tokens_input)}</span>
          <span>↙ {formatNum(session.tokens_output)}</span>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            onClick={onResume}
            className="flex items-center gap-1 px-2.5 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 text-white rounded-md transition-colors"
          >
            <Play size={12} />
            Resume
          </button>
          <CopyButton text={`cd "${directory}" && opencode --session ${session.id}`} />
          <button
            onClick={onDetail}
            className="flex items-center gap-1 px-2.5 py-1 text-xs bg-surface-hover hover:bg-surface-border text-gray-300 rounded-md transition-colors"
          >
            <Info size={12} />
            Detail
          </button>
        </div>
      </div>
    </div>
  );
}

function formatNum(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return n.toString();
}
