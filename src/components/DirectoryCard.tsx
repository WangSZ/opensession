import { FolderKanban, Terminal, FileText, Loader2, Pin, AlertCircle, GitBranch, EyeOff } from "lucide-react";
import type { DirectoryGroup } from "../types";

interface Props {
  directory: DirectoryGroup;
  isSelected: boolean;
  isGenerating: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onContextMenuOpen: (path: string, x: number, y: number) => void;
}

export default function DirectoryCard({
  directory, isSelected, isGenerating, onSelect, onOpen, onContextMenuOpen,
}: Props) {
  return (
    <div
      className={`px-3 py-2.5 cursor-pointer border-l-2 transition-colors ${
        isSelected
          ? "bg-surface-hover border-l-indigo-500"
          : "border-l-transparent hover:bg-surface-hover"
      } ${directory.hidden && !isSelected ? "opacity-50" : ""} ${directory.is_missing && !isSelected ? "opacity-40" : ""}`}
      onClick={onSelect}
      onContextMenu={e => { e.preventDefault(); onContextMenuOpen(directory.path, e.clientX, e.clientY); }}
    >
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center min-w-0 gap-2">
          <FolderKanban size={18} className="text-indigo-400 flex-shrink-0" />
          <span className="font-medium text-sm truncate">
            {directory.is_worktree ? (directory.repo_name ?? directory.name) : directory.name}
          </span>
          {directory.pinned && <Pin size={12} className="text-amber-400 flex-shrink-0" />}
          {directory.hidden && <EyeOff size={12} className="text-gray-500 flex-shrink-0" />}
          {directory.is_missing && (
            <span className="text-xs text-red-400 flex items-center gap-1 flex-shrink-0" title="目录已不存在">
              <AlertCircle size={12} />
              已删除
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5 flex-shrink-0">
          <span className="text-xs text-gray-500 bg-surface-border rounded-full px-2 py-0.5">
            {directory.session_count}
          </span>
          {directory.total_cost > 0 && (
            <span className="text-xs text-emerald-400 bg-emerald-500/10 rounded-full px-2 py-0.5">
              ${directory.total_cost.toFixed(2)}
            </span>
          )}
        </div>
      </div>

      {directory.is_worktree && (
        <div className="flex items-center gap-1 ml-6 mb-1.5 text-xs text-indigo-300">
          <GitBranch size={12} className="flex-shrink-0" />
          <span className="truncate">{directory.branch_name ?? directory.name}</span>
        </div>
      )}

      {directory.tags.length > 0 && (
        <div className="flex items-center gap-1 ml-6 mb-1.5 flex-wrap">
          {directory.tags.map(tag => (
            <span key={tag} className="text-xs bg-indigo-500/20 text-indigo-300 rounded-full px-2 py-0.5">
              {tag}
            </span>
          ))}
        </div>
      )}

      {isGenerating ? (
        <span className="text-xs text-gray-500 ml-6 mb-1.5 flex items-center gap-1">
          <Loader2 size={12} className="animate-spin" />
          生成中...
        </span>
      ) : directory.summary ? (
        <p className="text-xs text-gray-400 truncate ml-6 mb-1.5">{directory.summary.title}</p>
      ) : null}

      <div className="flex items-center gap-2 ml-6">
        <button
          onClick={e => { e.stopPropagation(); onOpen(); }}
          className="text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1 transition-colors"
        >
          <Terminal size={12} />
          {directory.is_missing ? "修复…" : "Open"}
        </button>
        {directory.has_agents_md && (
          <span className="text-xs text-gray-600 flex items-center gap-1">
            <FileText size={11} />
            AGENTS.md
          </span>
        )}
      </div>
    </div>
  );
}
