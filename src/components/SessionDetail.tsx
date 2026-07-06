import { X, MessageSquare, DollarSign, Cpu, User, Hash, FileCode } from "lucide-react";
import CopyButton from "./CopyButton";
import type { SessionDetail as SessionDetailType } from "../types";

interface Props {
  detail: SessionDetailType;
  directory: string;
  onClose: () => void;
}

export default function SessionDetail({ detail, directory, onClose }: Props) {
  const s = detail.session;

  return (
    <div className="fixed inset-0 z-50 flex">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />
      <div className="relative ml-auto w-96 bg-surface-card border-l border-surface-border h-full overflow-y-auto animate-slide-in">
        <div className="sticky top-0 bg-surface-card border-b border-surface-border p-4 flex items-center justify-between">
          <h2 className="font-semibold text-sm truncate">Session Detail</h2>
          <button onClick={onClose} className="p-1 hover:bg-surface-hover rounded transition-colors">
            <X size={18} />
          </button>
        </div>

        <div className="p-4 space-y-4">
          <div>
            <h3 className="font-medium text-base mb-1">{s.title || s.slug}</h3>
            <span className="text-xs text-gray-500 font-mono">{s.id}</span>
          </div>

          <div>
            <h4 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Command</h4>
            <div className="bg-surface rounded-lg p-3 flex items-center gap-2">
              <code className="text-xs font-mono text-gray-300 flex-1 min-w-0 truncate leading-relaxed">
                cd &quot;{directory}&quot; &amp;&amp; opencode --session {s.id}
              </code>
              <CopyButton variant="full" text={`cd "${directory}" && opencode --session ${s.id}`} />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <Stat icon={Cpu} label="Agent" value={s.agent} />
            <Stat icon={Hash} label="Model" value={s.model_name} />
            <Stat icon={DollarSign} label="Cost" value={`$${s.cost.toFixed(4)}`} />
            <Stat icon={MessageSquare} label="Messages" value={String(detail.messages_count)} />
            <Stat icon={User} label="Provider" value={detail.model_provider} />
            <Stat icon={FileCode} label="File Changes" value={String(s.file_changes)} />
          </div>

          <div>
            <h4 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Tokens</h4>
            <div className="bg-surface rounded-lg p-3 space-y-1.5 text-sm font-mono">
              <div className="flex justify-between"><span className="text-gray-400">Input</span><span>{formatNum(s.tokens_input)}</span></div>
              <div className="flex justify-between"><span className="text-gray-400">Output</span><span>{formatNum(s.tokens_output)}</span></div>
            </div>
          </div>

          <div>
            <h4 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Timeline</h4>
            <div className="bg-surface rounded-lg p-3 space-y-1.5 text-sm">
              <div className="flex justify-between"><span className="text-gray-400">Created</span><span className="text-xs">{s.time_created}</span></div>
              <div className="flex justify-between"><span className="text-gray-400">Updated</span><span className="text-xs">{s.time_updated}</span></div>
            </div>
          </div>

          {detail.first_message && (
            <div>
              <h4 className="text-xs text-gray-500 uppercase tracking-wider mb-2">First Message</h4>
              <div className="bg-surface rounded-lg p-3 text-sm text-gray-300 leading-relaxed">
                {detail.first_message.length > 300
                  ? detail.first_message.slice(0, 300) + "..."
                  : detail.first_message
                }
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function Stat({ icon: Icon, label, value }: { icon: any; label: string; value: string }) {
  return (
    <div className="bg-surface rounded-lg p-3">
      <div className="flex items-center gap-1.5 text-xs text-gray-500 mb-1">
        <Icon size={13} />
        {label}
      </div>
      <div className="text-sm font-medium truncate" title={value}>{value}</div>
    </div>
  );
}

function formatNum(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return n.toString();
}
