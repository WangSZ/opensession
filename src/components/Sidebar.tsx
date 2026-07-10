import { Search, FolderKanban, Loader2, Eye, EyeOff, Bug, Plus } from "lucide-react";
import DirectoryCard from "./DirectoryCard";
import IssueSection from "./IssueSection";
import type { DirectoryGroup, IssueWithSessions } from "../types";

type SidebarView = "dirs" | "issues";

interface Props {
  directories: DirectoryGroup[];
  selectedDir: string | null;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  onSelect: (path: string) => void;
  onOpen: (path: string, sessionId?: string) => void;
  onContextMenuOpen: (path: string, x: number, y: number) => void;
  generatingDirs: Set<string>;
  loading: boolean;
  allTags: string[];
  selectedTags: string[];
  onToggleTag: (tag: string) => void;
  showHidden: boolean;
  onToggleShowHidden: () => void;
  // Issues
  sidebarView: SidebarView;
  onViewChange: (view: SidebarView) => void;
  issues: IssueWithSessions[];
  loadingIssues: boolean;
  selectedIssueId: string | null;
  onSelectIssue: (issue: IssueWithSessions) => void;
  onCreateIssue: () => void;
  onIssueContextMenu?: (e: React.MouseEvent, issue: IssueWithSessions) => void;
  onNewDirectory: () => void;
}

export default function Sidebar({
  directories, selectedDir, searchQuery, onSearchChange,
  onSelect, onOpen, onContextMenuOpen,
  generatingDirs, loading,
  allTags, selectedTags, onToggleTag,
  showHidden, onToggleShowHidden,
  sidebarView, onViewChange, issues, loadingIssues,
  selectedIssueId, onSelectIssue, onCreateIssue, onIssueContextMenu,
  onNewDirectory,
}: Props) {
  return (
    <aside className="w-72 flex-shrink-0 border-r border-surface-border bg-surface-card flex flex-col">
      <div className="flex border-b border-surface-border">
        <button
          className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors ${
            sidebarView === "dirs"
              ? "text-indigo-400 border-b-2 border-indigo-500 bg-surface-hover"
              : "text-gray-500 hover:text-gray-300"
          }`}
          onClick={() => onViewChange("dirs")}
        >
          <FolderKanban size={14} />
          目录
        </button>
        <button
          className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors ${
            sidebarView === "issues"
              ? "text-indigo-400 border-b-2 border-indigo-500 bg-surface-hover"
              : "text-gray-500 hover:text-gray-300"
          }`}
          onClick={() => onViewChange("issues")}
        >
          <Bug size={14} />
          Issues {issues.length > 0 && <span className="text-gray-600 font-normal">({issues.length})</span>}
        </button>
        {sidebarView === "dirs" ? (
          <button
            className="px-3 text-gray-500 hover:text-gray-300 hover:bg-surface-hover transition-colors flex items-center justify-center"
            onClick={onNewDirectory}
            title="新建目录"
          >
            <Plus size={16} />
          </button>
        ) : (
          <button
            className="px-3 text-gray-500 hover:text-gray-300 hover:bg-surface-hover transition-colors flex items-center justify-center"
            onClick={onCreateIssue}
            title="新建 Issue"
          >
            <Plus size={16} />
          </button>
        )}
      </div>

      {sidebarView === "issues" ? (
        <IssueSection
          issues={issues}
          loading={loadingIssues}
          selectedIssueId={selectedIssueId}
          onSelectIssue={onSelectIssue}
          onContextMenu={onIssueContextMenu}
        />
      ) : (
        <>
          <div className="p-3 border-b border-surface-border space-y-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" size={16} />
              <input
                className="w-full bg-surface-hover text-gray-200 rounded-lg pl-9 pr-9 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
                placeholder="Search directories..."
                value={searchQuery}
                onChange={e => onSearchChange(e.target.value)}
              />
              <button
                className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-200 transition-colors p-0.5"
                onClick={onToggleShowHidden}
                title={showHidden ? "隐藏已隐藏的目录" : "显示已隐藏的目录"}
              >
                {showHidden ? <EyeOff size={15} /> : <Eye size={15} />}
              </button>
            </div>

            {allTags.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5">
                <button
                  className={`text-xs px-2 py-0.5 rounded-full transition-colors ${
                    selectedTags.length === 0
                      ? "bg-indigo-500/30 text-indigo-200"
                      : "bg-surface-hover text-gray-400 hover:text-gray-200"
                  }`}
                  onClick={() => selectedTags.forEach(t => onToggleTag(t))}
                >
                  全部
                </button>
                {allTags.map(tag => (
                  <button
                    key={tag}
                    className={`text-xs px-2 py-0.5 rounded-full flex items-center gap-1 transition-colors ${
                      selectedTags.includes(tag)
                        ? "bg-indigo-500/30 text-indigo-200"
                        : "bg-surface-hover text-gray-400 hover:text-gray-200"
                    }`}
                    onClick={() => onToggleTag(tag)}
                  >
                    {tag}
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="flex-1 overflow-y-auto">
            {directories.length === 0 ? (
              loading ? (
                <div className="flex items-center justify-center py-12 text-gray-500">
                  <Loader2 className="animate-spin mr-2" size={18} />
                  Loading...
                </div>
              ) : (
                <div className="flex flex-col items-center py-12 text-gray-500">
                  <FolderKanban size={32} className="mb-2 opacity-50" />
                  <span className="text-sm">No directories found</span>
                </div>
              )
            ) : (
              directories.map(dir => (
                <DirectoryCard
                  key={dir.path}
                  directory={dir}
                  isSelected={selectedDir === dir.path}
                  isGenerating={generatingDirs.has(dir.path)}
                  onSelect={() => onSelect(dir.path)}
                  onOpen={() => onOpen(dir.path)}
                  onContextMenuOpen={onContextMenuOpen}
                />
              ))
            )}
          </div>
        </>
      )}
    </aside>
  );
}
