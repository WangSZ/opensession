import { useState, useEffect, useRef } from "react";

interface Props {
  onConfirm: (name: string) => void;
  onClose: () => void;
}

export default function NewTagModal({ onConfirm, onClose }: Props) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  function handleSubmit() {
    const trimmed = value.trim();
    if (trimmed) {
      onConfirm(trimmed);
    }
  }

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-xl shadow-2xl p-5 w-80"
        onClick={e => e.stopPropagation()}
      >
        <h3 className="text-sm font-medium text-gray-200 mb-3">新建标签</h3>
        <input
          ref={inputRef}
          className="w-full bg-surface-hover text-gray-200 rounded-lg px-3 py-2 text-sm border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
          placeholder="输入标签名称"
          value={value}
          onChange={e => setValue(e.target.value)}
          onKeyDown={e => { if (e.key === "Enter") handleSubmit(); }}
        />
        <div className="flex justify-end gap-2 mt-4">
          <button
            className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            onClick={onClose}
          >
            取消
          </button>
          <button
            className="px-3 py-1.5 text-sm bg-indigo-500 hover:bg-indigo-400 text-white rounded-lg transition-colors disabled:opacity-40"
            disabled={!value.trim()}
            onClick={handleSubmit}
          >
            确定
          </button>
        </div>
      </div>
    </div>
  );
}
