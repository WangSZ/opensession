import type { SessionSource } from "../types";

export const SESSION_SOURCES: SessionSource[] = ["opencode", "codex", "reasonix"];

const SOURCE_CONFIG: Record<SessionSource, {
  label: string;
  badgeClass: string;
  startButtonClass: string;
}> = {
  opencode: {
    label: "OpenCode",
    badgeClass: "bg-indigo-500/15 text-indigo-300",
    startButtonClass: "bg-indigo-500/15 text-indigo-300 hover:bg-indigo-500/25",
  },
  codex: {
    label: "Codex",
    badgeClass: "bg-emerald-500/15 text-emerald-300",
    startButtonClass: "bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25",
  },
  reasonix: {
    label: "Reasonix",
    badgeClass: "bg-amber-500/15 text-amber-300",
    startButtonClass: "bg-amber-500/15 text-amber-300 hover:bg-amber-500/25",
  },
};

export function isSessionSource(value: string | null): value is SessionSource {
  return SESSION_SOURCES.some(source => source === value);
}

export function sourceLabel(source: SessionSource): string {
  return SOURCE_CONFIG[source].label;
}

export function sourceBadgeClass(source: SessionSource): string {
  return SOURCE_CONFIG[source].badgeClass;
}

export function sourceStartButtonClass(source: SessionSource): string {
  return SOURCE_CONFIG[source].startButtonClass;
}
