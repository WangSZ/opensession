# OpenSession

Visual session manager for [opencode](https://opencode.ai) — browse, search, and resume AI coding sessions with a native desktop UI.

![OpenSession screenshot](screenshot.png)

## Features

- **Directory browser** — browse opencode session directories with summaries and tags
- **Session explorer** — virtual-scrolled list of past sessions per directory
- **One-click resume** — open any session in Terminal via `opencode --session <id>`
- **Command copy** — copy `cd /path && opencode --session <id>` with one click
- **Session detail** — full detail drawer showing model, timestamps, and command
- **Right-click actions** — open in Finder/Terminal, pin, hide, tag, rename
- **Smart summaries** — AI-powered directory summaries (generated via `opencode`)
- **Bootstrap** — quick-start a new opencode session in any directory
- **Search & filter** — search directories by name, filter by tags
- **Dark theme** — native-looking dark UI

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (edition 2021)
- [opencode CLI](https://opencode.ai) on `PATH` (required for summarization and Terminal launch)

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
| Database | SQLite (read-only via rusqlite, bundled) |
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
│   ├── db.rs           Read-only SQLite queries
│   ├── config.rs       DB path resolution, XDG dirs, migration
│   ├── models.rs       Data structures
│   └── commands/       Tauri commands (sessions, summary, terminal, meta)
├── Cargo.toml
└── tauri.conf.json
```

## License

MIT License — see [LICENSE](LICENSE).
