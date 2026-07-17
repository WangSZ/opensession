import { useState, useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MessageSquare, Loader2, Search, Eye, EyeOff } from "lucide-react";
import SessionCard from "./SessionCard";
import CopyButton from "./CopyButton";
import type { Session, Issue } from "../types";

interface Props {
  directory: string | null;
  sessions: Session[];
  loading: boolean;
  onResume: (sessionId: string) => void;
  onDetail: (sessionId: string) => void;
  onContextMenu: (sessionId: string, x: number, y: number) => void;
  onSetNote: (sessionId: string, note: string) => void;
  sessionIssueMap?: Record<string, Issue>;
  dirIssueMap?: Record<string, Issue>;
  highlightedSessionId?: string | null;
}

export default function SessionPanel({ directory, sessions, loading, onResume, onDetail, onContextMenu, onSetNote, sessionIssueMap, dirIssueMap, highlightedSessionId }: Props) {
  const [searchQuery, setSearchQuery] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const parentRef = useRef<HTMLDivElement>(null);

  const filteredSessions = useMemo(() => {
    let list = sessions;
    if (!showHidden) {
      list = list.filter(s => !s.hidden);
    }
    const q = searchQuery.toLowerCase();
    if (q) {
      list = list.filter(s =>
        s.title.toLowerCase().includes(q) ||
        s.slug.toLowerCase().includes(q) ||
        s.first_message.toLowerCase().includes(q) ||
        s.note.toLowerCase().includes(q)
      );
    }
    return [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned));
  }, [sessions, searchQuery, showHidden]);

  const virtualizer = useVirtualizer({
    count: filteredSessions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 120,
    overscan: 10,
  });

  if (!directory) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500">
        <div className="text-center">
          <MessageSquare size={48} className="mx-auto mb-3 opacity-30" />
          <p className="text-lg">Select a directory to view sessions</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-w-0">
      <div className="px-5 py-3 border-b border-surface-border flex items-center gap-2">
        <code className="text-xs text-gray-300 font-mono truncate flex-1 min-w-0">
          cd &quot;{directory}&quot; &amp;&amp; opencode
        </code>
        <CopyButton text={`cd "${directory}" && opencode`} />
      </div>

      <div className="px-5 py-2 border-b border-surface-border flex items-center gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" size={14} />
          <input
            className="w-full bg-surface-hover text-gray-200 rounded-lg pl-8 pr-3 py-1.5 text-xs border border-surface-border focus:border-indigo-500 focus:outline-none transition-colors"
            placeholder="Search sessions by title, message or note..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
          />
        </div>
        <button
          onClick={() => setShowHidden(v => !v)}
          className={`flex items-center gap-1 px-2 py-1 text-xs rounded-md transition-colors ${
            showHidden
              ? "bg-indigo-600/20 text-indigo-400"
              : "text-gray-500 hover:text-gray-300"
          }`}
          title={showHidden ? "隐藏隐藏的会话" : "显示隐藏的会话"}
        >
          {showHidden ? <EyeOff size={14} /> : <Eye size={14} />}
        </button>
        <span className="text-xs text-gray-400 bg-surface-border rounded-full px-2 py-0.5 flex-shrink-0">
          {sessions.length > 0 && searchQuery.trim()
            ? `${filteredSessions.length}/${sessions.length}`
            : sessions.length}
        </span>
      </div>

      <div ref={parentRef} className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-16 text-gray-500">
            <Loader2 className="animate-spin mr-2" size={20} />
            Loading sessions...
          </div>
        ) : filteredSessions.length === 0 ? (
          <div className="flex flex-col items-center py-16 text-gray-500">
            <MessageSquare size={36} className="mb-2 opacity-30" />
            <p className="text-sm">{sessions.length === 0 ? "No sessions in this directory" : "No matching sessions"}</p>
          </div>
        ) : (
          <div
            style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}
          >
            {virtualizer.getVirtualItems().map(item => (
              <div
                key={item.key}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: `${item.size}px`,
                  transform: `translateY(${item.start}px)`,
                }}
              >
                <SessionCard
                  session={filteredSessions[item.index]}
                  directory={directory}
                  onResume={() => onResume(filteredSessions[item.index].id)}
                  onDetail={() => onDetail(filteredSessions[item.index].id)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    onContextMenu(filteredSessions[item.index].id, e.clientX, e.clientY);
                  }}
                  onSetNote={(note) => onSetNote(filteredSessions[item.index].id, note)}
                  issue={sessionIssueMap?.[filteredSessions[item.index].id] ?? dirIssueMap?.[directory] ?? null}
                  highlighted={highlightedSessionId === filteredSessions[item.index].id}
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
