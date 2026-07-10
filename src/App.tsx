import { useState, useEffect, useMemo } from "react";
import { listen } from "@tauri-apps/api/event";
import { useCommands } from "./hooks/useCommands";
import Sidebar from "./components/Sidebar";
import SessionPanel from "./components/SessionPanel";
import SessionDetail from "./components/SessionDetail";
import IssueDetail from "./components/IssueDetail";
import ContextMenu from "./components/ContextMenu";
import type { MenuItem } from "./components/ContextMenu";
import NewTagModal from "./components/NewTagModal";
import EditSummaryModal from "./components/EditSummaryModal";
import ConfirmDialog from "./components/ConfirmDialog";
import WorktreeModal from "./components/WorktreeModal";
import IssueModal from "./components/IssueModal";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { Database, Terminal, FolderOpen, Pin, Tag, EyeOff, Sparkles, GitBranch, GitFork, Play, Copy, Info, Bug, Pencil, Trash2, CodeXml, Braces } from "lucide-react";
import type { DirectoryGroup, Session, SessionDetail as SessionDetailType, Summary, Issue, IssueWithSessions } from "./types";

const isMac = navigator.platform.toLowerCase().includes("mac");
type SidebarView = "dirs" | "issues";

function App() {
  const {
    listDirectories, getSessions, getSessionDetail, generateSummary,
    setCachedSummary, deleteCachedSummary,
    openInTerminal, forkSession, setDirectoryTags, deleteTagGlobal, togglePin, openInFileManager, openInVSCode, openInJetBrains,
    toggleHidden, bootstrapSession, renameDirectoryMeta, openWorktree, removeWorktree,
    toggleSessionHidden, toggleSessionPin,
    createIssue, updateIssue, deleteIssue, linkSessionToIssue, unlinkSessionFromIssue, getAllIssues,
  } = useCommands(loadDirectories);

  const [directories, setDirectories] = useState<DirectoryGroup[]>([]);
  const [selectedDir, setSelectedDir] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [detailSession, setDetailSession] = useState<SessionDetailType | null>(null);
  const [loadingDirs, setLoadingDirs] = useState(true);
  const [loadingSessions, setLoadingSessions] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dbNotFound, setDbNotFound] = useState(false);
  const [generatingDirs, setGeneratingDirs] = useState<Set<string>>(new Set());
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [ctxMenu, setCtxMenu] = useState<{ path: string; x: number; y: number } | null>(null);
  const [newTagModal, setNewTagModal] = useState<string | null>(null);
  const [confirmDeleteTag, setConfirmDeleteTag] = useState<string | null>(null);
  const [editSummaryModal, setEditSummaryModal] = useState<{ path: string; summary: Summary } | null>(null);
  const [worktreeModal, setWorktreeModal] = useState<{ path: string; defaultName: string } | null>(null);
  const [confirmRemoveWorktree, setConfirmRemoveWorktree] = useState<string | null>(null);
  const [sessionCtxMenu, setSessionCtxMenu] = useState<{ sessionId: string; x: number; y: number } | null>(null);
  const [showHidden, setShowHidden] = useState(false);
  // Issue state
  const [sidebarView, setSidebarView] = useState<SidebarView>("dirs");
  const [issues, setIssues] = useState<IssueWithSessions[]>([]);
  const [loadingIssues, setLoadingIssues] = useState(false);
  const [issueModal, setIssueModal] = useState<{ sessionId: string; directory: string } | null>(null);
  const [editIssueModal, setEditIssueModal] = useState<IssueWithSessions | null>(null);
  const [confirmDeleteIssue, setConfirmDeleteIssue] = useState<string | null>(null);
  const [highlightedSessionId, setHighlightedSessionId] = useState<string | null>(null);
  const [issueCtxMenu, setIssueCtxMenu] = useState<{ issue: IssueWithSessions; x: number; y: number } | null>(null);
  const [selectedIssue, setSelectedIssue] = useState<IssueWithSessions | null>(null);

  const sessionIssueMap = useMemo(() => {
    const map: Record<string, Issue> = {};
    for (const iws of issues) {
      for (const s of iws.sessions) {
        map[s.session_id] = iws.issue;
      }
    }
    return map;
  }, [issues]);

  useEffect(() => {
    if (!selectedIssue) return;
    const updated = issues.find(i => i.issue.id === selectedIssue.issue.id);
    if (!updated) {
      setSelectedIssue(null);
    } else if (updated !== selectedIssue) {
      setSelectedIssue(updated);
    }
  }, [issues]);

  useEffect(() => {
    loadDirectories();
    const onFocus = () => loadDirectories();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  async function loadDirectories() {
    setDbNotFound(false);
    try {
      const [dirs, loadedIssues] = await Promise.all([
        listDirectories(),
        getAllIssues().catch(() => [] as IssueWithSessions[]),
      ]);
      setDirectories(dirs);
      setIssues(loadedIssues);
      setError(null);
    } catch (e) {
      const msg = String(e);
      if (msg === "OPENCODE_DB_NOT_FOUND") {
        setDbNotFound(true);
      } else {
        setError(msg);
      }
    } finally {
      setLoadingDirs(false);
    }
  }

  async function loadIssues() {
    setLoadingIssues(true);
    try {
      const loaded = await getAllIssues();
      setIssues(loaded);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingIssues(false);
    }
  }

  async function handleSelectDir(path: string) {
    setSelectedDir(path);
    setHighlightedSessionId(null);
    setLoadingSessions(true);
    try {
      const sess = await getSessions(path);
      setSessions(sess);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingSessions(false);
    }
  }

  async function handleDetail(sessionId: string) {
    try {
      const detail = await getSessionDetail(sessionId);
      setDetailSession(detail);
    } catch (e) {
      setError(String(e));
    }
  }

  function handleGenerateSummary(directory: string) {
    setGeneratingDirs(prev => new Set(prev).add(directory));
    generateSummary(directory).catch(e => setError(String(e)));
  }

  async function handleSaveSummary(title: string, description: string) {
    if (!editSummaryModal) return;
    const summary: Summary = {
      title,
      description,
      generated_at: new Date().toISOString(),
    };
    try {
      await setCachedSummary(editSummaryModal.path, summary);
    } catch (e) {
      setError(String(e));
    }
    setEditSummaryModal(null);
  }

  async function handleDeleteSummary() {
    if (!editSummaryModal) return;
    const path = editSummaryModal.path;
    setEditSummaryModal(null);
    try {
      await deleteCachedSummary(path);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleOpenInTerminal(directory: string, sessionId?: string) {
    try {
      await openInTerminal(directory, sessionId);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("DIRECTORY_NOT_FOUND")) {
        await openDirectoryMissingDialog(directory);
      } else {
        setError(msg);
      }
    }
  }

  async function handleForkSession(directory: string, sessionId: string) {
    try {
      await forkSession(directory, sessionId);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("DIRECTORY_NOT_FOUND")) {
        await openDirectoryMissingDialog(directory);
      } else {
        setError(msg);
      }
    }
  }

  function handleCreateWorktree(path: string, template: string) {
    const now = new Date();
    const y = now.getFullYear();
    const m = String(now.getMonth() + 1).padStart(2, "0");
    const d = String(now.getDate()).padStart(2, "0");
    const defaultName = template ? `${template}-${y}${m}${d}` : "";
    setWorktreeModal({ path, defaultName });
  }

  async function handleRemoveWorktree(path: string) {
    try {
      await removeWorktree(path, false);
    } catch (e) {
      if (String(e) === "WORKTREE_NOT_CLEAN") {
        setConfirmRemoveWorktree(path);
      } else {
        setError(String(e));
      }
    }
  }

  async function handleForceRemoveWorktree(path: string) {
    setConfirmRemoveWorktree(null);
    try {
      await removeWorktree(path, true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleConfirmWorktree(name: string, base: string) {
    if (!worktreeModal) return;
    const { path } = worktreeModal;
    setWorktreeModal(null);
    try {
      await openWorktree(path, name, base);
    } catch (e) {
      const msg = String(e);
      if (msg === "NOT_A_GIT_REPO") {
        await ask("该目录不是 git 仓库，无法创建 worktree。", { title: "提示", kind: "warning" });
      } else if (msg === "WORKTREE_PATH_EXISTS") {
        await ask(`分支 "${name}" 的 worktree 已存在，请选择其他名称。`, { title: "提示", kind: "warning" });
      } else {
        setError(msg);
      }
    }
  }

  async function openDirectoryMissingDialog(path: string) {
    const dirName = directories.find(d => d.path === path)?.name || path;
    const shouldHide = await ask(`目录 "${dirName}" 已不存在。\n\n点击"隐藏目录"将其从列表中隐藏。\n如果目录已改名或移动，点击"选择新目录"来保留标签和置顶设置。`, {
      title: "目录不存在",
      kind: "warning",
      okLabel: "隐藏目录",
      cancelLabel: "选择新目录",
    });

    if (shouldHide) {
      await toggleHidden(path);
      await loadDirectories();
      return;
    }

    const shouldRelocate = await ask("是否要选择一个新目录来继承该目录的标签和置顶设置？", {
      title: "保留配置",
      kind: "info",
      okLabel: "选择新目录",
      cancelLabel: "取消",
    });

    if (!shouldRelocate) return;

    const newPath = await open({
      directory: true,
      multiple: false,
      title: "选择新目录",
    });

    if (!newPath) return;

    try {
      await renameDirectoryMeta(path, newPath);
      await bootstrapSession(newPath);
      await loadDirectories();
      setSelectedDir(newPath);
      await handleSelectDir(newPath);
    } catch (e) {
      setError(`迁移失败: ${String(e)}`);
    }
  }

  const allTags = useMemo(
    () => Array.from(new Set(directories.flatMap(d => d.tags))).sort(),
    [directories],
  );

  const filteredDirs = useMemo(() => {
    let list = directories;
    if (!showHidden) {
      list = list.filter(d => !d.hidden);
    }
    const q = searchQuery.toLowerCase();
    if (q) {
      list = list.filter(d =>
        d.name.toLowerCase().includes(q) || d.path.toLowerCase().includes(q),
      );
    }
    if (selectedTags.length > 0) {
      list = list.filter(d => d.tags.some(t => selectedTags.includes(t)));
    }
    return [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned));
  }, [directories, searchQuery, selectedTags, showHidden]);

  useEffect(() => {
    if (!selectedDir && filteredDirs.length > 0 && !loadingDirs) {
      handleSelectDir(filteredDirs[0].path);
    }
  }, [filteredDirs.length, loadingDirs]);

  useEffect(() => {
    if (selectedDir && !filteredDirs.find(d => d.path === selectedDir) && filteredDirs.length > 0) {
      handleSelectDir(filteredDirs[0].path);
    }
  }, [searchQuery, selectedTags, showHidden]);

  useEffect(() => {
    const unlisten = listen<{ directory: string; summary?: Summary; error?: string }>(
      "summary://generated",
      (event) => {
        const { directory, summary, error } = event.payload;
        setGeneratingDirs(prev => {
          const next = new Set(prev);
          next.delete(directory);
          return next;
        });
        if (error) {
          setError(error);
        } else {
          loadDirectories();
        }
      },
    );
    return () => { unlisten.then(fn => fn()); };
  }, []);

  useEffect(() => {
    const unlisten = listen("git_info://updated", () => loadDirectories());
    return () => { unlisten.then(fn => fn()); };
  }, []);

  function handleToggleTag(tag: string) {
    setSelectedTags(prev =>
      prev.includes(tag) ? prev.filter(t => t !== tag) : [...prev, tag],
    );
  }

  function handleContextOpen(path: string, x: number, y: number) {
    setCtxMenu({ path, x, y });
  }

  function handleSessionContextOpen(sessionId: string, x: number, y: number) {
    setSessionCtxMenu({ sessionId, x, y });
  }

  function handleIssueContextOpen(e: React.MouseEvent, issue: IssueWithSessions) {
    e.preventDefault();
    setIssueCtxMenu({ issue, x: e.clientX, y: e.clientY });
  }

  async function handleToggleSessionHidden(sessionId: string) {
    await toggleSessionHidden(sessionId);
    if (selectedDir) {
      const sess = await getSessions(selectedDir);
      setSessions(sess);
    }
  }

  async function handleToggleSessionPin(sessionId: string) {
    await toggleSessionPin(sessionId);
    if (selectedDir) {
      const sess = await getSessions(selectedDir);
      setSessions(sess);
    }
  }

  // Issue handlers
  function handleSelectIssue(issue: IssueWithSessions) {
    setSelectedIssue(issue);
  }

  function handleIssueOpenSession(sessionId: string, directory: string) {
    setSidebarView("dirs");
    setSelectedIssue(null);
    setHighlightedSessionId(sessionId);
    setSelectedDir(directory);
    getSessions(directory).then(sess => {
      setSessions(sess);
    }).catch(e => setError(String(e)));
  }

  function handleIssueOpenTerminal(directory: string, sessionId?: string) {
    openInTerminal(directory, sessionId).catch(e => setError(String(e)));
  }

  function handleIssueOpenFileManager(directory: string) {
    openInFileManager(directory).catch(e => setError(String(e)));
  }

  function handleIssueEdit() {
    if (selectedIssue) setEditIssueModal(selectedIssue);
  }

  function handleIssueDelete() {
    if (selectedIssue) setConfirmDeleteIssue(selectedIssue.issue.id);
  }

  async function handleCreateIssueFromModal(title: string): Promise<string | null> {
    try {
      const issue = await createIssue(title, null, null, "medium", "open", null);
      await loadIssues();
      return issue.id;
    } catch (e) {
      setError(String(e));
      return null;
    }
  }

  async function handleCreateNewIssue() {
    setEditIssueModal({
      issue: { id: "", title: "", description: null, url: null, priority: "medium", status: "open", deadline: null, created_at: "", updated_at: "" },
      sessions: [],
    });
  }

  async function handleSaveEditIssue(issueId: string, title: string, description?: string | null, url?: string | null, priority?: string | null, status?: string | null, deadline?: string | null) {
    try {
      if (issueId) {
        await updateIssue(issueId, title, description ?? null, url ?? null, priority ?? null, status ?? null, deadline ?? null);
      } else {
        await createIssue(title, description ?? null, url ?? null, priority ?? null, status ?? null, deadline ?? null);
      }
      await loadIssues();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDeleteIssueConfirm(id: string) {
    setConfirmDeleteIssue(null);
    try {
      await deleteIssue(id);
      await loadIssues();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleLinkSession(sessionId: string, issueId: string) {
    if (!selectedDir) return;
    try {
      await linkSessionToIssue(sessionId, issueId, selectedDir);
      await loadIssues();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnlinkSession(sessionId: string) {
    try {
      await unlinkSessionFromIssue(sessionId);
      await loadIssues();
    } catch (e) {
      setError(String(e));
    }
  }

  const ctxDir = ctxMenu ? directories.find(d => d.path === ctxMenu.path) : undefined;

  const ctxSession = sessionCtxMenu ? sessions.find(s => s.id === sessionCtxMenu.sessionId) : null;
  const ctxSessionIssue = ctxSession ? sessionIssueMap[ctxSession.id] : null;

  const sessionCtxItems: MenuItem[] = useMemo(() => {
    if (!ctxSession || !selectedDir) return [];
    return [
      {
        type: "item" as const,
        label: "Resume",
        icon: <Play size={14} />,
        onClick: () => handleOpenInTerminal(selectedDir, ctxSession.id),
      },
      {
        type: "item" as const,
        label: "Fork 新会话",
        icon: <GitFork size={14} />,
        onClick: () => handleForkSession(selectedDir, ctxSession.id),
      },
      {
        type: "item" as const,
        label: "复制命令",
        icon: <Copy size={14} />,
        onClick: () => {
          navigator.clipboard.writeText(`cd "${selectedDir}" && opencode --session ${ctxSession.id}`);
        },
      },
      {
        type: "item" as const,
        label: "查看详情",
        icon: <Info size={14} />,
        onClick: () => handleDetail(ctxSession.id),
      },
      { type: "separator" as const },
      {
        type: "item" as const,
        label: ctxSessionIssue ? "取消关联 Issue" : "关联 Issue...",
        icon: <Bug size={14} />,
        onClick: () => {
          if (ctxSessionIssue) {
            handleUnlinkSession(ctxSession.id);
          } else {
            setIssueModal({ sessionId: ctxSession.id, directory: selectedDir });
          }
        },
        danger: !!ctxSessionIssue,
      },
      {
        type: "item" as const,
        label: ctxSession.pinned ? "取消置顶" : "置顶",
        icon: <Pin size={14} />,
        onClick: () => handleToggleSessionPin(ctxSession.id),
      },
      {
        type: "item" as const,
        label: ctxSession.hidden ? "取消隐藏" : "隐藏",
        icon: <EyeOff size={14} />,
        onClick: () => handleToggleSessionHidden(ctxSession.id),
      },
    ];
  }, [ctxSession, selectedDir, ctxSessionIssue]);

  const STATUS_OPTIONS = [
    { value: "open", label: "待处理" },
    { value: "pending", label: "待定" },
    { value: "in_progress", label: "进行中" },
    { value: "resolved", label: "已解决" },
    { value: "closed", label: "已关闭" },
    { value: "wont_fix", label: "不予处理" },
  ];

  const PRIORITY_OPTIONS = [
    { value: "high", label: "高", dot: "bg-red-500" },
    { value: "medium", label: "中", dot: "bg-amber-500" },
    { value: "low", label: "低", dot: "bg-green-500" },
  ];

  const ctxIssue = issueCtxMenu?.issue;
  const issueCtxItems: MenuItem[] = useMemo(() => {
    if (!ctxIssue) return [];
    const i = ctxIssue.issue;
    return [
      {
        type: "item" as const,
        label: "状态",
        icon: <span className={`w-2 h-2 rounded-full ${
          i.status === "in_progress" ? "bg-blue-400" :
          i.status === "resolved" ? "bg-green-400" :
          i.status === "closed" || i.status === "wont_fix" ? "bg-gray-500" :
          i.status === "pending" ? "bg-amber-400" :
          "bg-gray-400"
        }`} />,
        submenu: STATUS_OPTIONS.map(opt => ({
          type: "item" as const,
          label: opt.label,
          checked: i.status === opt.value,
          onClick: () => {
            updateIssue(i.id, null, null, null, null, opt.value, null);
            loadIssues();
          },
        })),
      },
      {
        type: "item" as const,
        label: "优先级",
        icon: <span className={`w-2 h-2 rounded-full ${
          i.priority === "high" ? "bg-red-500" :
          i.priority === "low" ? "bg-green-500" :
          "bg-amber-500"
        }`} />,
        submenu: PRIORITY_OPTIONS.map(opt => ({
          type: "item" as const,
          label: opt.label,
          icon: <span className={`w-2 h-2 rounded-full ${opt.dot}`} />,
          checked: i.priority === opt.value,
          onClick: () => {
            updateIssue(i.id, null, null, null, opt.value, null, null);
            loadIssues();
          },
        })),
      },
      { type: "separator" as const },
      {
        type: "item" as const,
        label: "编辑",
        icon: <Pencil size={14} />,
        onClick: () => {
          const matched = issues.find(iws => iws.issue.id === i.id);
          if (matched) setEditIssueModal(matched);
        },
      },
      {
        type: "item" as const,
        label: "删除",
        icon: <Trash2 size={14} />,
        danger: true,
        onClick: () => setConfirmDeleteIssue(i.id),
      },
    ];
  }, [ctxIssue, issues]);

  const ctxItems: MenuItem[] = useMemo(() => {
    if (!ctxDir) return [];
    const dir = ctxDir;
    const dirPath = dir.path;

    function buildTagSubmenu(): MenuItem[] {
      return [
        ...allTags.map(tag => ({
          type: "item" as const,
          label: tag,
          checked: dir.tags.includes(tag),
          onClick: () => {
            const newTags = dir.tags.includes(tag)
              ? dir.tags.filter(t => t !== tag)
              : [...dir.tags, tag];
            setDirectoryTags(dirPath, newTags);
          },
          onContextMenu: () => setConfirmDeleteTag(tag),
        })),
        ...(allTags.length > 0 ? [{ type: "separator" as const }] : []),
        {
          type: "item" as const,
          label: "+ 新增标签",
          onClick: () => setNewTagModal(dirPath),
        },
      ];
    }

    return [
      {
        type: "item" as const,
        label: isMac ? "在 Finder 中显示" : "在资源管理器中显示",
        icon: <FolderOpen size={14} />,
        onClick: () => openInFileManager(dirPath),
      },
      {
        type: "item" as const,
        label: "在 VS Code 中打开",
        icon: <CodeXml size={14} />,
        onClick: () => openInVSCode(dirPath).catch(e => setError(String(e))),
      },
      {
        type: "item" as const,
        label: "在 IntelliJ IDEA 中打开",
        icon: <Braces size={14} />,
        onClick: () => openInJetBrains(dirPath).catch(e => setError(String(e))),
      },
      {
        type: "item" as const,
        label: "打开终端",
        icon: <Terminal size={14} />,
        onClick: () => handleOpenInTerminal(dirPath),
      },
      {
        type: "item" as const,
        label: dir.summary ? "刷新摘要" : "生成摘要",
        icon: <Sparkles size={14} />,
        onClick: () => handleGenerateSummary(dirPath),
        disabled: generatingDirs.has(dirPath),
      },
      ...(dir.summary
        ? [{
            type: "item" as const,
            label: "编辑摘要",
            icon: <Pencil size={14} />,
            onClick: () => setEditSummaryModal({ path: dirPath, summary: dir.summary! }),
          }]
        : []),
      { type: "separator" as const },
      {
        type: "item" as const,
        label: "新建 worktree",
        icon: <GitBranch size={14} />,
        disabled: !dir.is_git_repo,
        title: dir.is_git_repo ? undefined : "不是 git 仓库",
        submenu: [
          { type: "item" as const, label: "feature", onClick: () => handleCreateWorktree(dirPath, "feature") },
          { type: "item" as const, label: "hotfix", onClick: () => handleCreateWorktree(dirPath, "hotfix") },
          { type: "item" as const, label: "bugfix", onClick: () => handleCreateWorktree(dirPath, "bugfix") },
          { type: "item" as const, label: "release", onClick: () => handleCreateWorktree(dirPath, "release") },
          { type: "separator" as const },
          { type: "item" as const, label: "自定义…", onClick: () => handleCreateWorktree(dirPath, "") },
        ],
      },
      ...(dir.is_worktree
        ? [
            { type: "separator" as const },
            {
              type: "item" as const,
              label: "删除 worktree",
              danger: true,
              onClick: () => handleRemoveWorktree(dirPath),
            },
          ]
        : []),
      {
        type: "item" as const,
        label: dir.pinned ? "取消置顶" : "置顶",
        icon: <Pin size={14} />,
        onClick: () => togglePin(dirPath),
      },
      {
        type: "item" as const,
        label: dir.hidden ? "取消隐藏" : "隐藏目录",
        icon: <EyeOff size={14} />,
        onClick: () => toggleHidden(dirPath),
      },
      {
        type: "item" as const,
        label: "标签",
        icon: <Tag size={14} />,
        submenu: buildTagSubmenu(),
      },
    ];
  }, [ctxDir, allTags]);

  return (
    <div className="flex h-screen bg-surface text-gray-100 overflow-hidden">
      <Sidebar
        directories={filteredDirs}
        selectedDir={selectedDir}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        onSelect={handleSelectDir}
        onOpen={handleOpenInTerminal}
        onContextMenuOpen={handleContextOpen}
        generatingDirs={generatingDirs}
        loading={loadingDirs}
        allTags={allTags}
        selectedTags={selectedTags}
        onToggleTag={handleToggleTag}
        showHidden={showHidden}
        onToggleShowHidden={() => setShowHidden(v => !v)}
        sidebarView={sidebarView}
        onViewChange={setSidebarView}
        issues={issues}
        loadingIssues={loadingIssues}
        selectedIssueId={selectedIssue?.issue.id ?? null}
        onSelectIssue={handleSelectIssue}
        onCreateIssue={handleCreateNewIssue}
        onIssueContextMenu={handleIssueContextOpen}
      />
      {dbNotFound ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center max-w-md px-8">
            <Database className="w-16 h-16 mx-auto mb-4 text-gray-600" />
            <h2 className="text-xl font-semibold text-gray-300 mb-2">No opencode data found</h2>
            <p className="text-gray-500 leading-relaxed mb-6">
              This app displays sessions from <code className="text-gray-400 bg-surface-card px-1.5 py-0.5 rounded text-sm">opencode</code>.
              Make sure you have <strong className="text-gray-400">opencode CLI</strong> installed and have started at least one session.
            </p>
            <p className="text-gray-600 text-sm flex items-center justify-center gap-2">
              <Terminal className="w-4 h-4" />
              Run <code className="text-gray-400 bg-surface-card px-1.5 py-0.5 rounded">opencode</code> in a project to get started
            </p>
          </div>
        </div>
      ) : sidebarView === "issues" && selectedIssue ? (
        <IssueDetail
          issue={selectedIssue}
          onEdit={handleIssueEdit}
          onDelete={handleIssueDelete}
          onOpenSession={handleIssueOpenSession}
          onOpenFileManager={handleIssueOpenFileManager}
          onOpenInTerminal={handleIssueOpenTerminal}
        />
      ) : (
        <SessionPanel
          directory={selectedDir}
          sessions={sessions}
          loading={loadingSessions}
          onResume={(sid) => selectedDir && handleOpenInTerminal(selectedDir, sid)}
          onDetail={handleDetail}
          onContextMenu={handleSessionContextOpen}
          sessionIssueMap={sessionIssueMap}
          highlightedSessionId={highlightedSessionId}
        />
      )}
      {detailSession && (
        <SessionDetail
          detail={detailSession}
          directory={selectedDir ?? ""}
          onClose={() => setDetailSession(null)}
        />
      )}
      {ctxMenu && ctxItems.length > 0 && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          items={ctxItems}
          onClose={() => setCtxMenu(null)}
        />
      )}
      {sessionCtxMenu && sessionCtxItems.length > 0 && (
        <ContextMenu
          x={sessionCtxMenu.x}
          y={sessionCtxMenu.y}
          items={sessionCtxItems}
          onClose={() => setSessionCtxMenu(null)}
        />
      )}
      {issueCtxMenu && issueCtxItems.length > 0 && (
        <ContextMenu
          x={issueCtxMenu.x}
          y={issueCtxMenu.y}
          items={issueCtxItems}
          onClose={() => setIssueCtxMenu(null)}
        />
      )}
      {worktreeModal !== null && (
        <WorktreeModal
          directory={worktreeModal.path}
          defaultName={worktreeModal.defaultName}
          onConfirm={handleConfirmWorktree}
          onClose={() => setWorktreeModal(null)}
        />
      )}
      {confirmRemoveWorktree !== null && (
        <ConfirmDialog
          title="强制删除 worktree？"
          message="该 worktree 包含未合并的代码，强制删除可能导致代码丢失。"
          confirmLabel="强制删除"
          danger
          onConfirm={() => handleForceRemoveWorktree(confirmRemoveWorktree)}
          onClose={() => setConfirmRemoveWorktree(null)}
        />
      )}
      {newTagModal !== null && (
        <NewTagModal
          onConfirm={name => {
            setDirectoryTags(newTagModal, [
              ...(directories.find(d => d.path === newTagModal)?.tags ?? []),
              name,
            ]);
            setNewTagModal(null);
          }}
          onClose={() => setNewTagModal(null)}
        />
      )}
      {editSummaryModal !== null && (
        <EditSummaryModal
          title={editSummaryModal.summary.title}
          description={editSummaryModal.summary.description}
          onSave={handleSaveSummary}
          onDelete={handleDeleteSummary}
          onClose={() => setEditSummaryModal(null)}
        />
      )}
      {confirmDeleteTag !== null && (
        <ConfirmDialog
          title={`删除标签 "${confirmDeleteTag}"`}
          message="此操作会从所有目录移除该标签。"
          confirmLabel="删除"
          danger
          onConfirm={() => {
            deleteTagGlobal(confirmDeleteTag);
            setConfirmDeleteTag(null);
          }}
          onClose={() => setConfirmDeleteTag(null)}
        />
      )}
      {issueModal !== null && (
        <IssueModal
          issues={issues}
          currentIssueId={sessionIssueMap[issueModal.sessionId]?.id}
          onSelect={(issueId) => handleLinkSession(issueModal.sessionId, issueId)}
          onCreate={handleCreateIssueFromModal}
          onClose={() => setIssueModal(null)}
        />
      )}
      {editIssueModal !== null && (
        <EditIssueModal
          issue={editIssueModal}
          onSave={handleSaveEditIssue}
          onClose={() => setEditIssueModal(null)}
        />
      )}
      {confirmDeleteIssue !== null && (
        <ConfirmDialog
          title="删除 Issue"
          message="此操作将删除该 Issue 及其所有关联关系。"
          confirmLabel="删除"
          danger
          onConfirm={() => handleDeleteIssueConfirm(confirmDeleteIssue)}
          onClose={() => setConfirmDeleteIssue(null)}
        />
      )}
      {error && (
        <div className="fixed bottom-4 right-4 bg-red-600 text-white px-4 py-2 rounded-lg shadow-lg text-sm">
          {error}
          <button className="ml-2 underline" onClick={() => setError(null)}>Close</button>
        </div>
      )}
    </div>
  );
}

function EditIssueModal({ issue, onSave, onClose }: {
  issue: IssueWithSessions;
  onSave: (id: string, title: string, description?: string | null, url?: string | null, priority?: string | null, status?: string | null, deadline?: string | null) => void;
  onClose: () => void;
}) {
  const [title, setTitle] = useState(issue.issue.title);
  const [description, setDescription] = useState(issue.issue.description ?? "");
  const [priority, setPriority] = useState(issue.issue.priority || "medium");
  const [status, setStatus] = useState(issue.issue.status || "open");
  const [deadline, setDeadline] = useState(issue.issue.deadline ? issue.issue.deadline.slice(0, 10) : "");

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl shadow-black/40 w-[400px] p-4"
        onClick={e => e.stopPropagation()}
      >
        <h3 className="text-sm font-medium mb-3">{issue.issue.id ? "编辑 Issue" : "新建 Issue"}</h3>
        <input
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-2"
          placeholder="Issue 标题"
          value={title}
          onChange={e => setTitle(e.target.value)}
          autoFocus
        />
        <textarea
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-3 resize-none"
          placeholder="描述（可选）"
          value={description}
          onChange={e => setDescription(e.target.value)}
          rows={3}
        />

        <div className="flex gap-3 mb-3">
          <div className="flex-1">
            <label className="text-xs text-gray-500 block mb-1">优先级</label>
            <select value={priority} onChange={e => setPriority(e.target.value)}
              className="w-full bg-surface-hover text-gray-200 rounded-lg px-2.5 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            >
              <option value="high">🔴 高</option>
              <option value="medium">🟡 中</option>
              <option value="low">🟢 低</option>
            </select>
          </div>
          <div className="flex-1">
            <label className="text-xs text-gray-500 block mb-1">状态</label>
            <select value={status} onChange={e => setStatus(e.target.value)}
              className="w-full bg-surface-hover text-gray-200 rounded-lg px-2.5 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            >
              <option value="open">待处理</option>
              <option value="pending">待定</option>
              <option value="in_progress">进行中</option>
              <option value="resolved">已解决</option>
              <option value="closed">已关闭</option>
              <option value="wont_fix">不予处理</option>
            </select>
          </div>
        </div>

        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">截止日期</label>
          <div className="flex gap-2 items-center">
            <input type="date" value={deadline}
              onChange={e => setDeadline(e.target.value)}
              className="flex-1 bg-surface-hover text-gray-200 rounded-lg px-2.5 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            />
            {deadline && (
              <button onClick={() => setDeadline("")} className="text-xs text-gray-500 hover:text-gray-200 transition-colors">
                清除
              </button>
            )}
          </div>
        </div>

        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 transition-colors">取消</button>
          <button
            onClick={() => {
              if (title.trim()) {
                onSave(
                  issue.issue.id,
                  title.trim(),
                  description.trim() || null,
                  null,
                  priority,
                  status,
                  deadline ? new Date(deadline + "T23:59:59").toISOString() : null,
                );
                onClose();
              }
            }}
            disabled={!title.trim()}
            className="px-3 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white rounded-md transition-colors"
          >
            {issue.issue.id ? "保存" : "创建"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
