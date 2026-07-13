import { FolderKanban, Terminal, Loader2, Pin, AlertCircle, EyeOff } from "lucide-react";
import type { DirectoryGroup } from "../types";

const MAX_VISIBLE_TAGS = 3;

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
  const displayName = directory.is_worktree
    ? `${directory.repo_name ?? directory.name} (${directory.branch_name ?? directory.name})`
    : directory.name;

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
          <span className="font-medium text-sm truncate">{displayName}</span>
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

      <div className="flex items-center justify-between min-w-0 gap-2">
        <div className="flex items-center min-w-0 flex-1">
          {isGenerating ? (
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <Loader2 size={12} className="animate-spin" />
              生成中...
            </span>
          ) : directory.summary ? (
            <p className="text-xs text-gray-400 truncate">{directory.summary.title}</p>
          ) : null}
        </div>

        <div className="flex items-center gap-1.5 flex-shrink-0">
          <button
            onClick={e => { e.stopPropagation(); onOpen(); }}
            className="text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1 transition-colors"
          >
            <Terminal size={12} />
            {directory.is_missing ? "修复…" : "Open"}
          </button>
          {directory.tags.length > 0 && (
            <div className="flex items-center gap-1">
              {directory.tags.slice(0, MAX_VISIBLE_TAGS).map(tag => (
                <span key={tag} className="text-xs bg-indigo-500/20 text-indigo-300 rounded-full px-2 py-0.5">
                  {tag}
                </span>
              ))}
              {directory.tags.length > MAX_VISIBLE_TAGS && (
                <span className="text-xs text-gray-500 bg-surface-border rounded-full px-2 py-0.5">
                  +{directory.tags.length - MAX_VISIBLE_TAGS}
                </span>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
