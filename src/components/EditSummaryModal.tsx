import { useState, useEffect, useRef } from "react";

interface Props {
  title: string;
  description: string;
  onSave: (title: string, description: string) => void;
  onDelete: () => void;
  onClose: () => void;
}

export default function EditSummaryModal({ title: initialTitle, description: initialDescription, onSave, onDelete, onClose }: Props) {
  const [title, setTitle] = useState(initialTitle);
  const [description, setDescription] = useState(initialDescription);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl p-5 w-96"
        onClick={e => e.stopPropagation()}
      >
        <h3 className="text-sm font-medium text-gray-200 mb-3">编辑摘要</h3>
        <input
          ref={inputRef}
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-2"
          placeholder="摘要标题"
          value={title}
          onChange={e => setTitle(e.target.value)}
          onKeyDown={e => { if (e.key === "Enter") e.currentTarget.blur(); }}
        />
        <textarea
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors mb-4 resize-none"
          placeholder="详细描述（可选）"
          value={description}
          onChange={e => setDescription(e.target.value)}
          rows={4}
        />
        <div className="flex justify-between items-center">
          <button
            className="px-3 py-1.5 text-sm text-red-400 hover:text-red-300 transition-colors"
            onClick={onDelete}
          >
            删除摘要
          </button>
          <div className="flex gap-2">
            <button
              className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
              onClick={onClose}
            >
              取消
            </button>
            <button
              className="px-3 py-1.5 text-sm bg-indigo-500 hover:bg-indigo-400 text-white rounded-lg transition-colors disabled:opacity-40"
              disabled={!title.trim()}
              onClick={() => onSave(title.trim(), description.trim())}
            >
              保存
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
