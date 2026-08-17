# OpenSession

Visual session manager for [OpenCode](https://opencode.ai), [Codex](https://developers.openai.com/codex/), and Reasonix — browse, search, filter, resume, and fork AI coding sessions with a native desktop UI.

![OpenSession screenshot](screenshot.png)

## Features

- **Unified directory browser** — merge OpenCode, Codex, and Reasonix sessions under one project directory
- **Session explorer** — virtual-scrolled list with All/OpenCode/Codex/Reasonix source filters
- **One-click resume and fork** — launch the owning CLI with the correct session command
- **Command copy** — copy source-aware resume commands for all three agents
- **Session detail** — full detail drawer showing model, timestamps, and command
- **Right-click actions** — open in Finder/Terminal, pin, hide, tag, rename
- **Smart summaries** — generated via OpenCode, with Codex and Reasonix fallbacks
- **Quick start** — open OpenCode, Codex, or Reasonix in any directory
- **Search & filter** — search directories by name, filter by tags
- **Dark theme** — native-looking dark UI

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (edition 2021)
- [OpenCode CLI](https://opencode.ai), [Codex CLI](https://developers.openai.com/codex/cli/), Reasonix CLI, or any combination on `PATH`

## Getting Started

```bash
npm install
npm run tauri dev
```

This starts Vite dev server with hot-reload on `http://localhost:1420` and launches the Tauri desktop window.

## Build

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/macos/OpenSession.app` (~13 MB standalone app).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Tailwind CSS, Vite 6 |
| Desktop | Tauri 2 (Rust) |
| Session storage | SQLite and Reasonix JSONL metadata (read-only) |
| Virtualization | @tanstack/react-virtual |
| Icons | lucide-react |

## Project Structure

```
src/                    React frontend
├── components/         UI components (Sidebar, DirectoryCard, SessionPanel, etc.)
├── hooks/              Tauri invoke wrappers
├── utils/              Clipboard helper
├── App.tsx             Main app, state management, event handling
├── types.ts            TypeScript interfaces
└── main.tsx            Entry point

src-tauri/              Rust backend
├── src/
│   ├── main.rs         Entry point (calls my_sessions_lib::run())
│   ├── lib.rs          Plugin registration, command handlers
│   ├── db.rs           Read-only OpenCode SQLite queries
│   ├── codex_db.rs     Read-only Codex SQLite discovery and queries
│   ├── reasonix_db.rs  Read-only Reasonix JSONL metadata discovery
│   ├── session_repository.rs  Cross-source aggregation
│   ├── session_sources.rs     Source identity and CLI command construction
│   ├── config.rs       DB path resolution, XDG dirs, migration
│   ├── models.rs       Data structures
│   └── commands/       Tauri commands (sessions, summary, terminal, meta)
├── Cargo.toml
└── tauri.conf.json
```

## License

MIT License — see [LICENSE](LICENSE).
