import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Search, Loader2, ChevronDown, Check, X, Plus, Trash2, FileText, Copy, FolderSearch, SquareTerminal } from "lucide-react";
import type { SearchMode, SearchScopeArg, SearchScopeDef, SearchResultsPayload } from "../types";

interface Props {
  onClose: () => void;
}

function splitPath(path: string): [string, string] {
  const idx = path.lastIndexOf("/");
  if (idx <= 0) return [path, ""];
  return [path.slice(idx + 1), path.slice(0, idx)];
}

function Highlighted({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>;
  // NFKD compare per char so 李雨(U+96E8) highlights 李⾬(U+2FAC) and vice versa
  const textChars = [...text];
  const nText = textChars.map(c => c.normalize("NFKD").toLowerCase());
  const nQuery = [...query].map(c => c.normalize("NFKD").toLowerCase());
  let idx = -1;
  for (let i = 0; i + nQuery.length <= nText.length; i++) {
    if (nQuery.every((c, j) => nText[i + j] === c)) {
      idx = i;
      break;
    }
  }
  if (idx < 0) return <>{text}</>;
  return (
    <>
      {textChars.slice(0, idx).join("")}
      <span className="text-indigo-300">{textChars.slice(idx, idx + nQuery.length).join("")}</span>
      {textChars.slice(idx + nQuery.length).join("")}
    </>
  );
}

export default function SearchOverlay({ onClose }: Props) {
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState<SearchMode>("name");
  const [scopeKey, setScopeKey] = useState("home");
  const [maxResults, setMaxResults] = useState(() => {
    const stored = Number(localStorage.getItem("opensession:max-results"));
    return stored > 0 ? stored : 200;
  });
  const [customScopes, setCustomScopes] = useState<SearchScopeDef[]>([]);
  const [scopePanelOpen, setScopePanelOpen] = useState(false);
  const [managingScope, setManagingScope] = useState(false);
  const [newScopeName, setNewScopeName] = useState("");
  const [newScopeDirs, setNewScopeDirs] = useState<string[]>([]);
  const [results, setResults] = useState<string[]>([]);
  const [searching, setSearching] = useState(false);
  const [done, setDone] = useState(false);
  const [truncated, setTruncated] = useState(false);
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const parentRef = useRef<HTMLDivElement>(null);
  const currentSearchId = useRef<string | null>(null);
  const startAt = useRef(0);

  const virtualizer = useVirtualizer({
    count: results.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 46,
    overscan: 8,
  });

  useEffect(() => {
    invoke<SearchScopeDef[]>("list_search_scopes").then(setCustomScopes).catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = listen<SearchResultsPayload>("search://results", (event) => {
      const payload = event.payload;
      if (payload.searchId !== currentSearchId.current) return;
      if (payload.paths.length > 0) setResults(prev => [...prev, ...payload.paths]);
      if (payload.done) {
        setSearching(false);
        setDone(true);
        setTruncated(payload.truncated);
        setElapsedMs(performance.now() - startAt.current);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  function cancelActive() {
    if (currentSearchId.current) {
      invoke("cancel_search", { searchId: currentSearchId.current }).catch(() => {});
      currentSearchId.current = null;
    }
  }

  useEffect(() => () => cancelActive(), []);

  function handleMaxResultsChange(value: number) {
    setMaxResults(value);
    localStorage.setItem("opensession:max-results", String(value));
  }

  async function runSearch(q: string, m: SearchMode, key: string) {
    cancelActive();
    const searchId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    currentSearchId.current = searchId;
    setResults([]);
    setSelected(0);
    setSearching(true);
    setDone(false);
    setTruncated(false);
    setError(null);
    startAt.current = performance.now();
    const scope: SearchScopeArg =
      key === "home" ? { kind: "home" } :
      key === "global" ? { kind: "global" } :
      { kind: "custom", id: key };
    try {
      await invoke("start_search", { searchId, query: q, mode: m, scope, limit: maxResults });
    } catch (e) {
      setSearching(false);
      setDone(true);
      setError(String(e));
    }
  }

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      cancelActive();
      setResults([]);
      setSearching(false);
      setDone(false);
      setElapsedMs(null);
      return;
    }
    const t = setTimeout(() => runSearch(q, mode, scopeKey), 300);
    return () => clearTimeout(t);
  }, [query, mode, scopeKey, maxResults]);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (managingScope) { setManagingScope(false); return; }
        if (scopePanelOpen) { setScopePanelOpen(false); return; }
        onClose();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected(s => Math.min(s + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected(s => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        const path = results[selected];
        if (path) {
          invoke("reveal_file", { path }).catch(() => {});
          onClose();
        }
      }
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [results, selected, scopePanelOpen, managingScope, onClose]);

  useEffect(() => {
    virtualizer.scrollToIndex(selected);
  }, [selected]);

  const scopeLabel =
    scopeKey === "home" ? "个人目录" :
    scopeKey === "global" ? "全局" :
    customScopes.find(s => s.id === scopeKey)?.name ?? "集合";

  function handleSelectScope(key: string) {
    setScopeKey(key);
    setScopePanelOpen(false);
    setManagingScope(false);
  }

  async function handleDeleteScope(id: string) {
    try {
      await invoke("delete_search_scope", { id });
      setCustomScopes(prev => prev.filter(s => s.id !== id));
      if (scopeKey === id) setScopeKey("home");
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSaveScope() {
    const name = newScopeName.trim();
    if (!name || newScopeDirs.length === 0) return;
    try {
      const scope = await invoke<SearchScopeDef>("add_search_scope", { name, directories: newScopeDirs });
      setCustomScopes(prev => [...prev, scope]);
      setManagingScope(false);
      setNewScopeName("");
      setNewScopeDirs([]);
      setScopeKey(scope.id);
      setScopePanelOpen(false);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePickDirs() {
    try {
      const picked = await open({ directory: true, multiple: true, title: "选择要包含的目录" });
      if (!picked) return;
      const list = Array.isArray(picked) ? picked : [picked];
      setNewScopeDirs(prev => [...prev, ...list.filter(d => !prev.includes(d))]);
    } catch (e) {
      setError(String(e));
    }
  }

  const statusText = !query.trim()
    ? "输入关键词搜索文件（基于 Spotlight 索引）"
    : searching
    ? "搜索中…"
    : done
    ? `${results.length} 个结果${truncated ? `（已达上限 ${maxResults}）` : ""}${elapsedMs !== null ? ` · ${(elapsedMs / 1000).toFixed(2)}s` : ""}`
    : "";

  return (
    <div className="fixed inset-0 z-50 bg-black/40 flex justify-center pt-[12vh] px-6" onClick={onClose}>
      <div
        className="h-fit max-h-[70vh] w-[640px] bg-surface-card border border-surface-border rounded-xl shadow-2xl shadow-black/50 flex flex-col overflow-hidden"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 px-4 py-3 border-b border-surface-border">
          <Search size={18} className="text-gray-500 shrink-0" />
          <input
            autoFocus
            className="flex-1 min-w-0 bg-transparent text-base text-gray-100 placeholder-gray-600 focus:outline-none"
            placeholder={mode === "name" ? "搜索文件名，支持 * 通配符，如 *.rs…" : "全文搜索…"}
            value={query}
            onChange={e => setQuery(e.target.value)}
          />
          <div className="flex items-center bg-surface-hover border border-surface-border rounded-md p-0.5 text-xs shrink-0">
            <button
              onClick={() => setMode("name")}
              className={`px-2 py-1 rounded transition-colors ${mode === "name" ? "bg-surface-border text-gray-100" : "text-gray-500 hover:text-gray-300"}`}
            >
              文件名
            </button>
            <button
              onClick={() => setMode("content")}
              className={`px-2 py-1 rounded transition-colors ${mode === "content" ? "bg-surface-border text-gray-100" : "text-gray-500 hover:text-gray-300"}`}
            >
              全文
            </button>
          </div>
          <button
            onClick={() => { setScopePanelOpen(v => !v); setManagingScope(false); }}
            className="flex items-center gap-1 px-2 py-1 text-xs text-gray-400 hover:text-gray-200 bg-surface-hover border border-surface-border rounded-md transition-colors shrink-0"
            title="搜索范围"
          >
            {scopeLabel}
            <ChevronDown size={12} />
          </button>
        </div>

        {scopePanelOpen && (
          <div className="border-b border-surface-border px-2 py-2 max-h-72 overflow-y-auto text-xs">
            {!managingScope ? (
              <>
                {[["home", "个人目录"], ["global", "全局"]].map(([key, label]) => (
                  <button
                    key={key}
                    onClick={() => handleSelectScope(key)}
                    className="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-surface-hover text-gray-300 transition-colors"
                  >
                    {label}
                    {scopeKey === key && <Check size={13} className="text-indigo-400" />}
                  </button>
                ))}
                {customScopes.length > 0 && <div className="mx-1 my-1 border-t border-surface-border" />}
                {customScopes.map(s => (
                  <div key={s.id} className="flex items-center gap-1 group">
                    <button
                      onClick={() => handleSelectScope(s.id)}
                      className="flex-1 flex items-center justify-between px-2 py-1.5 rounded hover:bg-surface-hover text-gray-300 transition-colors min-w-0"
                    >
                      <span className="truncate">{s.name}</span>
                      <span className="text-gray-600 shrink-0 ml-2">{s.directories.length} 个目录{scopeKey === s.id && <Check size={13} className="inline ml-1 text-indigo-400" />}</span>
                    </button>
                    <button
                      onClick={() => handleDeleteScope(s.id)}
                      className="p-1 rounded text-gray-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all shrink-0"
                      title="删除集合"
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                ))}
                <button
                  onClick={() => setManagingScope(true)}
                  className="w-full flex items-center gap-1 px-2 py-1.5 rounded hover:bg-surface-hover text-gray-500 hover:text-gray-300 transition-colors"
                >
                  <Plus size={12} />
                  添加集合（自定义目录组合）
                </button>
              </>
            ) : (
              <div className="px-2 py-1 space-y-2">
                <input
                  autoFocus
                  className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none"
                  placeholder="集合名称，如：工作项目"
                  value={newScopeName}
                  onChange={e => setNewScopeName(e.target.value)}
                />
                {newScopeDirs.map(dir => (
                  <div key={dir} className="flex items-center gap-1 text-gray-400">
                    <span className="flex-1 truncate font-mono">{dir}</span>
                    <button
                      onClick={() => setNewScopeDirs(prev => prev.filter(d => d !== dir))}
                      className="p-0.5 text-gray-600 hover:text-red-400 shrink-0"
                    >
                      <X size={12} />
                    </button>
                  </div>
                ))}
                <div className="flex items-center gap-2">
                  <button
                    onClick={handlePickDirs}
                    className="flex items-center gap-1 px-2 py-1 rounded bg-surface-hover border border-surface-border text-gray-300 hover:text-gray-100 transition-colors"
                  >
                    <Plus size={12} />
                    选择目录
                  </button>
                  <div className="flex-1" />
                  <button onClick={() => setManagingScope(false)} className="px-2 py-1 text-gray-500 hover:text-gray-300 transition-colors">
                    取消
                  </button>
                  <button
                    onClick={handleSaveScope}
                    disabled={!newScopeName.trim() || newScopeDirs.length === 0}
                    className="px-2 py-1 rounded bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white transition-colors"
                  >
                    保存
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        <div ref={parentRef} className="flex-1 overflow-y-auto min-h-[120px] max-h-[42vh]">
          {error ? (
            <div className="px-4 py-6 text-sm text-red-400">{error}</div>
          ) : results.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-10 text-gray-600 text-sm gap-2">
              {searching ? <Loader2 size={20} className="animate-spin" /> : <FileText size={24} className="opacity-40" />}
              {searching ? "搜索中…" : query.trim() && done ? "没有匹配的文件" : mode === "name" ? "按文件名搜索" : "按文件内容搜索"}
            </div>
          ) : (
            <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
              {virtualizer.getVirtualItems().map(item => {
                const path = results[item.index];
                const [name, parent] = splitPath(path);
                return (
                  <div
                    key={path}
                    style={{
                      position: "absolute", top: 0, left: 0, width: "100%",
                      height: `${item.size}px`, transform: `translateY(${item.start}px)`,
                    }}
                    className={`flex items-center gap-2.5 px-4 cursor-pointer group ${item.index === selected ? "bg-surface-hover" : ""}`}
                    onMouseEnter={() => setSelected(item.index)}
                    onClick={() => {
                      invoke("reveal_file", { path }).catch(() => {});
                      onClose();
                    }}
                  >
                    <FileText size={15} className="text-gray-500 shrink-0" />
                    <div className="min-w-0 flex-1">
                      <div className="text-sm text-gray-200 truncate">
                        <Highlighted text={name} query={mode === "name" ? query.trim() : ""} />
                      </div>
                      <div className="text-xs text-gray-500 truncate">{parent}</div>
                    </div>
                    <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all shrink-0">
                      <button
                        onClick={e => {
                          e.stopPropagation();
                          invoke("reveal_file", { path }).catch(() => {});
                          onClose();
                        }}
                        className="p-1 rounded text-gray-600 hover:text-gray-200"
                        title="在 Finder 中显示"
                      >
                        <FolderSearch size={13} />
                      </button>
                      <button
                        onClick={e => {
                          e.stopPropagation();
                          invoke("open_plain_terminal", { directory: parent }).catch(() => {});
                          onClose();
                        }}
                        className="p-1 rounded text-gray-600 hover:text-gray-200"
                        title="在终端打开所在目录"
                      >
                        <SquareTerminal size={13} />
                      </button>
                      <button
                        onClick={e => {
                          e.stopPropagation();
                          navigator.clipboard.writeText(path);
                        }}
                        className="p-1 rounded text-gray-600 hover:text-gray-200"
                        title="复制完整路径"
                      >
                        <Copy size={13} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <div className="px-4 py-2 border-t border-surface-border flex items-center justify-between text-xs text-gray-600">
          <span>{statusText}</span>
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-1" title="最大结果数">
              最多
              <select
                value={maxResults}
                onChange={e => handleMaxResultsChange(Number(e.target.value))}
                className="bg-surface-hover border border-surface-border rounded px-1 py-0.5 text-xs text-gray-400 hover:text-gray-200 focus:outline-none focus:border-indigo-500 transition-colors"
              >
                {[200, 500, 2000, 10000].map(n => (
                  <option key={n} value={n}>{n}</option>
                ))}
              </select>
              条
            </label>
            <span>↑↓ 选择 · ⏎ 在 Finder 中显示 · Esc 关闭</span>
          </div>
        </div>
      </div>
    </div>
  );
}
