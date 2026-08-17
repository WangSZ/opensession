import { useState, useMemo, useRef, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MessageSquare, Loader2, Search, Eye, EyeOff } from "lucide-react";
import SessionCard from "./SessionCard";
import type { Session, SessionSource, Issue } from "../types";
import { sessionKey } from "../utils/sessionCommand";
import { SESSION_SOURCES, sourceLabel, sourceStartButtonClass } from "../utils/sessionSource";

interface Props {
  directory: string | null;
  sessions: Session[];
  loading: boolean;
  onStart: (source: SessionSource) => void;
  onResume: (session: Session) => void;
  onDetail: (session: Session) => void;
  onContextMenu: (session: Session, x: number, y: number) => void;
  onSetNote: (session: Session, note: string) => void;
  sessionIssueMap?: Record<string, Issue>;
  dirIssueMap?: Record<string, Issue>;
  highlightedSessionKey?: string | null;
}

export default function SessionPanel({ directory, sessions, loading, onStart, onResume, onDetail, onContextMenu, onSetNote, sessionIssueMap, dirIssueMap, highlightedSessionKey }: Props) {
  const [searchQuery, setSearchQuery] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const [sourceFilter, setSourceFilter] = useState<"all" | SessionSource>("all");
  const parentRef = useRef<HTMLDivElement>(null);

  const sourceCounts = useMemo(() => Object.fromEntries(
    SESSION_SOURCES.map(source => [source, sessions.filter(session => session.source === source).length]),
  ) as Record<SessionSource, number>, [sessions]);

  useEffect(() => {
    if (sourceFilter !== "all" && sourceCounts[sourceFilter] === 0) {
      setSourceFilter("all");
    }
  }, [sourceCounts, sourceFilter]);

  const filteredSessions = useMemo(() => {
    let list = sessions;
    if (sourceFilter !== "all") {
      list = list.filter(session => session.source === sourceFilter);
    }
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
  }, [sessions, searchQuery, showHidden, sourceFilter]);

  const virtualizer = useVirtualizer({
    count: filteredSessions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 120,
    overscan: 10,
  });
  const sourceFilters: Array<["all" | SessionSource, string, number]> = [
    ["all", "全部", sessions.length],
    ...SESSION_SOURCES.map(source => [source, sourceLabel(source), sourceCounts[source]] as [SessionSource, string, number]),
  ];

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
          {directory}
        </code>
        {SESSION_SOURCES.map(source => (
          <button
            key={source}
            onClick={() => onStart(source)}
            className={`px-2 py-1 text-xs rounded ${sourceStartButtonClass(source)}`}
            title={`Open ${sourceLabel(source)}`}
          >
            {sourceLabel(source)}
          </button>
        ))}
      </div>

      <div className="px-5 py-2 border-b border-surface-border flex items-center gap-2">
        <div className="flex items-center bg-surface-hover border border-surface-border rounded-md p-0.5 flex-shrink-0">
          {sourceFilters.map(([value, label, count]) => (
            <button
              key={value}
              onClick={() => setSourceFilter(value)}
              disabled={value !== "all" && count === 0}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                sourceFilter === value ? "bg-surface-border text-gray-100" : "text-gray-500 hover:text-gray-300"
              } disabled:opacity-30 disabled:cursor-not-allowed`}
            >
              {label} {count}
            </button>
          ))}
        </div>
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
                key={sessionKey(filteredSessions[item.index].source, filteredSessions[item.index].id)}
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
                  onResume={() => onResume(filteredSessions[item.index])}
                  onDetail={() => onDetail(filteredSessions[item.index])}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    onContextMenu(filteredSessions[item.index], e.clientX, e.clientY);
                  }}
                  onSetNote={(note) => onSetNote(filteredSessions[item.index], note)}
                  issue={sessionIssueMap?.[sessionKey(filteredSessions[item.index].source, filteredSessions[item.index].id)] ?? dirIssueMap?.[directory] ?? null}
                  highlighted={highlightedSessionKey === sessionKey(filteredSessions[item.index].source, filteredSessions[item.index].id)}
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
