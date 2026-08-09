import type { SessionSource } from "../types";

export function sessionKey(source: SessionSource, id: string): string {
  return `${source}:${id}`;
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\"'\"'`)}'`;
}

export function startCommand(directory: string, source: SessionSource): string {
  return `cd ${shellQuote(directory)} && ${source === "codex" ? "codex" : "opencode"}`;
}

export function resumeCommand(directory: string, source: SessionSource, id: string): string {
  return source === "codex"
    ? `cd ${shellQuote(directory)} && codex resume ${shellQuote(id)}`
    : `cd ${shellQuote(directory)} && opencode --session ${shellQuote(id)}`;
}

export function forkCommand(directory: string, source: SessionSource, id: string): string {
  return source === "codex"
    ? `cd ${shellQuote(directory)} && codex fork ${shellQuote(id)}`
    : `cd ${shellQuote(directory)} && opencode --session ${shellQuote(id)} --fork`;
}
