import type { SessionSource } from "../types";

export function sessionKey(source: SessionSource, id: string): string {
  return `${source}:${id}`;
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\"'\"'`)}'`;
}

const SOURCE_COMMANDS: Record<SessionSource, {
  start: string;
  resume: (id: string) => string;
  fork: (id: string) => string;
}> = {
  opencode: {
    start: "opencode",
    resume: id => `opencode --session ${shellQuote(id)}`,
    fork: id => `opencode --session ${shellQuote(id)} --fork`,
  },
  codex: {
    start: "codex",
    resume: id => `codex resume ${shellQuote(id)}`,
    fork: id => `codex fork ${shellQuote(id)}`,
  },
  reasonix: {
    start: "reasonix",
    resume: id => `reasonix --resume ${shellQuote(id)}`,
    fork: id => `reasonix --resume ${shellQuote(id)}`,
  },
};

export function startCommand(directory: string, source: SessionSource): string {
  return `cd ${shellQuote(directory)} && ${SOURCE_COMMANDS[source].start}`;
}

export function resumeCommand(directory: string, source: SessionSource, id: string): string {
  return `cd ${shellQuote(directory)} && ${SOURCE_COMMANDS[source].resume(id)}`;
}

export function forkCommand(directory: string, source: SessionSource, id: string): string {
  return `cd ${shellQuote(directory)} && ${SOURCE_COMMANDS[source].fork(id)}`;
}
