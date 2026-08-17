import { useEffect, useState } from "react";
import type { SessionSource } from "../types";
import { SESSION_SOURCES, sourceLabel } from "../utils/sessionSource";

interface Props {
  title: string;
  directory: string;
  initialSource: SessionSource;
  confirmLabel?: string;
  onConfirm: (source: SessionSource) => void;
  onClose: () => void;
}

export default function SourcePickerModal({
  title,
  directory,
  initialSource,
  confirmLabel = "打开",
  onConfirm,
  onClose,
}: Props) {
  const [source, setSource] = useState<SessionSource>(initialSource);

  useEffect(() => {
    function handleKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
      if (event.key === "Enter") onConfirm(source);
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose, onConfirm, source]);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-surface-card border border-surface-border rounded-lg shadow-2xl p-5 w-96 max-w-[calc(100vw-2rem)]"
        onClick={event => event.stopPropagation()}
      >
        <h3 className="text-sm font-medium text-gray-200 mb-2">{title}</h3>
        <p className="text-xs text-gray-500 font-mono truncate mb-4" title={directory}>{directory}</p>

        <div className="grid grid-cols-3 bg-surface-hover border border-surface-border rounded-md p-0.5">
          {SESSION_SOURCES.map(value => (
            <button
              key={value}
              type="button"
              onClick={() => setSource(value)}
              className={`px-3 py-2 text-xs rounded transition-colors ${
                source === value
                  ? "bg-surface-border text-gray-100"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {sourceLabel(value)}
            </button>
          ))}
        </div>

        <div className="flex justify-end gap-2 mt-4">
          <button
            className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            onClick={onClose}
          >
            取消
          </button>
          <button
            className="px-3 py-1.5 text-sm bg-indigo-500 hover:bg-indigo-400 text-white rounded-md transition-colors"
            onClick={() => onConfirm(source)}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
