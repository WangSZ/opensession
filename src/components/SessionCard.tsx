import { useState, useRef, useEffect } from "react";
import { Play, Info, FileCode, EyeOff, Bug, Pencil } from "lucide-react";
import CopyButton from "./CopyButton";
import type { Session, Issue } from "../types";

interface Props {
  session: Session;
  directory: string;
  onResume: () => void;
  onDetail: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onSetNote: (note: string) => void;
  issue?: Issue | null;
  highlighted?: boolean;
}

export default function SessionCard({ session, directory, onResume, onDetail, onContextMenu, onSetNote, issue, highlighted }: Props) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.selectionStart = inputRef.current.value.length;
      inputRef.current.selectionEnd = inputRef.current.value.length;
    }
  }, [editing]);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.style.height = "auto";
      inputRef.current.style.height = inputRef.current.scrollHeight + "px";
    }
  }, [editing, draft]);

  function startEdit() {
    setDraft(session.note);
    setEditing(true);
  }

  function save() {
    if (draft !== session.note) onSetNote(draft);
    setEditing(false);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") setEditing(false);
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") save();
  }
  return (
    <div
      className={`mx-4 my-1.5 bg-surface-card rounded-lg border p-3.5 transition-all hover:-translate-y-0.5 hover:shadow-lg group ${
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

      <div className="flex items-center justify-between gap-2 text-xs text-gray-400 mb-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="bg-surface-hover px-1.5 py-0.5 rounded whitespace-nowrap">{session.agent}</span>
          <span>·</span>
          <span className="truncate">{session.model_name}</span>
          <span>·</span>
          <span className="whitespace-nowrap">${session.cost.toFixed(3)}</span>
          {session.file_changes > 0 && (
            <>
              <span>·</span>
              <FileCode size={12} className="text-green-500 shrink-0" />
              <span className="text-green-400 whitespace-nowrap">{session.file_changes} files</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-3 text-xs text-gray-500 shrink-0">
          <span>↗ {formatNum(session.tokens_input)}</span>
          <span>↙ {formatNum(session.tokens_output)}</span>
        </div>
      </div>

      <div className="flex items-start gap-2 text-xs min-h-0">
        <div className="flex-1 min-w-0">
          {editing ? (
            <div className="flex items-start gap-1">
              <textarea
                ref={inputRef}
                className="flex-1 bg-surface-hover text-gray-200 rounded px-2 py-1 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none resize-none leading-relaxed"
                value={draft}
                onChange={e => setDraft(e.target.value)}
                onKeyDown={handleKeyDown}
                onBlur={save}
                placeholder="添加备注... (⌘+Enter 保存)"
                rows={1}
              />
              <div className="flex gap-0.5 pt-0.5">
                <button onMouseDown={e => { e.preventDefault(); save(); }} className="text-green-500 hover:text-green-400 p-0.5" title="Save">✓</button>
                <button onMouseDown={e => { e.preventDefault(); setEditing(false); }} className="text-gray-500 hover:text-gray-300 p-0.5" title="Cancel">✗</button>
              </div>
            </div>
          ) : session.note ? (
            <div className="flex items-start gap-1 group">
              <span
                className="text-gray-300 cursor-pointer whitespace-pre-wrap leading-relaxed"
                onClick={startEdit}
                title="Click to edit"
              >
                {session.note}
              </span>
              <button onClick={startEdit} className="text-gray-600 hover:text-gray-300 opacity-0 group-hover:opacity-100 transition-opacity p-0.5 pt-1">
                <Pencil size={11} />
              </button>
            </div>
          ) : (
            <button onClick={startEdit} className="text-gray-600 hover:text-gray-400 italic opacity-0 group-hover:opacity-100 transition-opacity">
              + 备注
            </button>
          )}
        </div>
        {!editing && (
          <div className="flex items-center gap-1.5 shrink-0">
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
        )}
      </div>

    </div>
  );
}

function formatNum(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return n.toString();
}
