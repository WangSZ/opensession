import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  directory: string;
  defaultName: string;
  onConfirm: (name: string, base: string) => void;
  onClose: () => void;
}

export default function WorktreeModal({ directory, defaultName, onConfirm, onClose }: Props) {
  const [name, setName] = useState(defaultName);
  const [selectedBase, setSelectedBase] = useState("HEAD");
  const [customBase, setCustomBase] = useState("");
  const [bases, setBases] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  useEffect(() => {
    invoke<string[]>("list_git_bases", { directory })
      .then(setBases)
      .catch(() => setBases(["HEAD"]))
      .finally(() => setLoading(false));
  }, [directory]);

  const isCustom = selectedBase === "__custom__";
  const resolvedBase = isCustom ? customBase.trim() || "HEAD" : selectedBase;

  function handleSubmit() {
    const trimmed = name.trim();
    if (trimmed) {
      onConfirm(trimmed, resolvedBase);
    }
  }

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl p-5 w-96"
        onClick={e => e.stopPropagation()}
      >
        <h3 className="text-sm font-medium text-gray-200 mb-4">新建 Worktree</h3>

        <label className="text-xs text-gray-400 mb-1 block">分支名称</label>
        <input
          ref={inputRef}
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-4"
          placeholder="输入分支名"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => { if (e.key === "Enter") handleSubmit(); }}
        />

        <label className="text-xs text-gray-400 mb-1 block">基于</label>
        <select
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-4 appearance-none cursor-pointer disabled:opacity-50"
          value={selectedBase}
          onChange={e => setSelectedBase(e.target.value)}
          disabled={loading}
        >
          {loading ? (
            <option value="HEAD">加载中...</option>
          ) : (
            <>
              {bases.map(b => (
                <option key={b} value={b}>{b}</option>
              ))}
              <option value="__custom__">自定义</option>
            </>
          )}
        </select>

        {isCustom && (
          <input
            className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-4"
            placeholder="输入 base 分支/标签"
            value={customBase}
            onChange={e => setCustomBase(e.target.value)}
          />
        )}

        <div className="flex justify-end gap-2 mt-2">
          <button
            className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            onClick={onClose}
          >
            取消
          </button>
          <button
            className="px-3 py-1.5 text-sm bg-indigo-500 hover:bg-indigo-400 text-white rounded-lg transition-colors disabled:opacity-40"
            disabled={!name.trim() || (isCustom && !customBase.trim())}
            onClick={handleSubmit}
          >
            确定
          </button>
        </div>
      </div>
    </div>
  );
}
