import { useState, useEffect, useRef } from "react";

interface Props {
  tags: string[];
  allTags: string[];
  x: number;
  y: number;
  onToggleTag: (tag: string) => void;
  onNewTag: () => void;
  onClose: () => void;
}

export default function TagPopup({ tags, allTags, x, y, onToggleTag, onNewTag, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useEffect(() => {
    if (ref.current) {
      const rect = ref.current.getBoundingClientRect();
      setPos({
        x: Math.max(8, Math.min(x, window.innerWidth - rect.width - 8)),
        y: Math.max(8, Math.min(y, window.innerHeight - rect.height - 8)),
      });
    }
  }, [x, y]);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50" onClick={onClose}>
      <div
        ref={ref}
        className="fixed bg-surface-card border border-white/10 rounded-lg shadow-2xl shadow-black/40 py-1 min-w-[160px] animate-context-menu"
        style={{ left: pos.x, top: pos.y }}
        onClick={e => e.stopPropagation()}
      >
        <div className="px-3 py-1.5 text-xs text-gray-500 font-medium">标签</div>
        {allTags.length === 0 ? (
          <div className="px-3 py-2 text-xs text-gray-600">暂无标签</div>
        ) : (
          allTags.map(tag => (
            <div
              key={tag}
              className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer text-gray-200 hover:bg-surface-hover transition-colors"
              onClick={() => onToggleTag(tag)}
            >
              <span className="w-4 text-indigo-400">{tags.includes(tag) ? "✓" : ""}</span>
              <span>{tag}</span>
            </div>
          ))
        )}
        <div className="h-px bg-surface-border my-1" />
        <div
          className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer text-gray-200 hover:bg-surface-hover transition-colors"
          onClick={() => { onNewTag(); onClose(); }}
        >
          <span className="w-4 text-indigo-400">+</span>
          <span>新增标签</span>
        </div>
      </div>
    </div>
  );
}