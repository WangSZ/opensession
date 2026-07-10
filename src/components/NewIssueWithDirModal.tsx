import { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderKanban, Loader2, X } from "lucide-react";

interface Props {
  onCreate: (title: string, description: string | null, priority: string, status: string, deadline: string | null, directory: string) => Promise<void>;
  onClose: () => void;
}

export default function NewIssueWithDirModal({ onCreate, onClose }: Props) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState("medium");
  const [status, setStatus] = useState("open");
  const [deadline, setDeadline] = useState("");
  const [directory, setDirectory] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  async function handlePickDir() {
    const dir = await open({ directory: true, multiple: false, title: "选择目录" });
    if (dir) setDirectory(dir);
  }

  async function handleCreate() {
    if (!title.trim() || !directory) return;
    setCreating(true);
    try {
      await onCreate(
        title.trim(),
        description.trim() || null,
        priority,
        status,
        deadline ? new Date(deadline + "T23:59:59").toISOString() : null,
        directory,
      );
    } finally {
      onClose();
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl shadow-black/40 w-[420px] p-4"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium">新建 Issue 并打开目录</h3>
          <button onClick={onClose} className="text-gray-500 hover:text-gray-200 transition-colors">
            <X size={16} />
          </button>
        </div>

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

        <div className="mb-4">
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

        <div className="flex items-center gap-2 mb-4">
          <button
            onClick={handlePickDir}
            className="flex items-center gap-1.5 px-3 py-2 text-xs bg-surface-hover hover:bg-surface-hover/80 text-gray-300 rounded-lg border border-surface-border transition-colors"
          >
            <FolderKanban size={14} />
            选择目录
          </button>
          <span className="text-xs text-gray-500 truncate flex-1">
            {directory || "未选择目录"}
          </span>
        </div>

        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 transition-colors">取消</button>
          <button
            onClick={handleCreate}
            disabled={creating || !title.trim() || !directory}
            className="flex items-center gap-1 px-3 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white rounded-md transition-colors"
          >
            {creating ? <Loader2 size={12} className="animate-spin" /> : <FolderKanban size={12} />}
            {creating ? "创建中…" : "创建并打开"}
          </button>
        </div>
      </div>
    </div>
  );
}
