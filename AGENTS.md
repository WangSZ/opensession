# OpenSession — AGENTS.md

## Build & Run

```bash
npm run tauri dev       # hot-reload dev
npm run tauri build     # production build → src-tauri/target/release/bundle/macos/OpenSession.app
npm run build           # frontend only (tsc && vite build)
cargo check             # Rust compile check (in src-tauri/)
```

`tauri build` runs `npm run build` first (via `beforeBuildCommand` in `tauri.conf.json`). Output is standalone `.app` (~13MB).

## Architecture

```
React 19 (src/)  ──invoke()──▶  Rust Tauri 2 (src-tauri/)
                                   ├── db.rs           — read-only SQLite (rusqlite bundled)
                                   ├── config.rs       — opencode DB resolution, app dirs, summaries migration
                                   ├── commands/sessions.rs  — directory/session queries
                                   ├── commands/summary.rs   — opencode CLI summarization
                                   ├── commands/terminal.rs  — cross-platform terminal launcher + bootstrap
                                   └── commands/meta.rs      — tags, pin, hidden, rename, file manager
```

Entrypoint: `main.rs` → `opensession_lib::run()` (lib crate `opensession_lib`, crate-type `["lib", "cdylib", "staticlib"]`).

Frontend uses `@tauri-apps/api/core` `invoke()`. Parameter names must match `#[tauri::command]` function arguments exactly.

All commands registered in `lib.rs` via `generate_handler![]`.

## Database

- Resolved via `opencode db path` CLI command, fallback to `$XDG_DATA_HOME/opencode/opencode.db` → `~/.local/share/opencode/opencode.db` → `dirs::data_dir()/opencode/opencode.db`
- Opened read-only (`SQLITE_OPEN_READ_ONLY`), never writes
- `session` table `model` column is JSON `{"id":"...","providerID":"..."}`, parsed via `serde_json`
- `part` / `message` tables contain conversation messages and tool calls

## Persistent State (Outside DB)

| File | Location | Format |
|------|----------|--------|
| Directory metadata | `$XDG_CONFIG_HOME/opensession/directory_meta.json` | `HashMap<String, DirectoryMeta>` (tags, pinned, hidden) |
| Summary cache | `$XDG_DATA_HOME/opensession/summaries.json` | `HashMap<String, Summary>` — migrated from old `{opencode_data_dir}/opensession/summaries.json` on first run |

## Registered Tauri Commands

| invoke name | Parameters | Returns |
|-------------|-----------|---------|
| `list_directories` | — | `Vec<DirectoryGroup>` |
| `get_sessions` | `directory: String` | `Vec<Session>` |
| `get_session_detail` | `sessionId: String` | `SessionDetail` |
| `search_directories` | `query: String` | `Vec<DirectoryGroup>` |
| `generate_summary` | `directory: String` | `()` — fire-and-forget, result via event |
| `get_cached_summary` | `directory: String` | `Option<Summary>` |
| `open_in_terminal` | `directory: String`, `sessionId?: String` | `()` |
| `bootstrap_session` | `directory: String` | `()` — runs `opencode run "hi"` in Terminal |
| `set_directory_tags` | `path: String`, `tags: Vec<String>` | `()` |
| `delete_tag_global` | `tag: String` | `()` |
| `toggle_directory_pin` | `path: String` | `bool` (new state) |
| `toggle_directory_hidden` | `path: String` | `bool` (new state) |
| `rename_directory_meta` | `old: String`, `new: String` | `()` — preserves tags/pin, resets hidden |
| `open_in_file_manager` | `directory: String` | `()` — Finder/Explorer/xdg-open |

## Platform Details

- macOS: writes temp shell script → `osascript` → Terminal.app (`temp_dir()/opencode_launch_{PID}.sh`, self-deletes with `rm "$0"`). Same pattern for `bootstrap_session`.
- Windows: `cmd /c start wt cmd /k "cd /d ... && opencode ..."`
- Linux: `x-terminal-emulator -e bash -c "cd ... && opencode ..."`
- `opencode` CLI must be on `PATH` (summarization and terminal launch both depend on it)
- Clipboard: `navigator.clipboard.writeText()` with `textarea` fallback — no Tauri plugin needed

## Tauri Events

| Event | Payload | When |
|-------|---------|------|
| `summary://generated` | `{directory, summary?} \| {directory, error?}` | `generate_summary` background task completes |

## Frontend Components

| Component | Path | Role |
|-----------|------|------|
| `Sidebar` | `src/components/Sidebar.tsx` | Directory list + search |
| `DirectoryCard` | `src/components/DirectoryCard.tsx` | Directory card (name, summary, Open/Generate buttons) |
| `SessionPanel` | `src/components/SessionPanel.tsx` | Virtual-scrolled session list + header command |
| `SessionCard` | `src/components/SessionCard.tsx` | Session row (Resume, copy command, Detail) |
| `SessionDetail` | `src/components/SessionDetail.tsx` | Detail slide-in drawer with full command |
| `ContextMenu` | `src/components/ContextMenu.tsx` | Right-click menu (Finder, Terminal, Pin, Hide, Tags) |
| `NewTagModal` | `src/components/NewTagModal.tsx` | Create new tag |
| `ConfirmDialog` | `src/components/ConfirmDialog.tsx` | Destructive confirmation (delete tag) |
| `CopyButton` | `src/components/CopyButton.tsx` | Reusable copy btn, icon changes to checkmark for 1.5s, variants: `icon` / `full` |

## Command Copy (3 locations)

| Location | Content | Button |
|----------|---------|--------|
| SessionPanel header | `cd "/path" && opencode` | copy icon |
| SessionCard | `cd "/path" && opencode --session <id>` | copy icon |
| SessionDetail | `cd "/path" && opencode --session <id>` | Copy button with text |

## Key UX Flows

- **Missing directory**: when user tries to open a deleted/moved directory, shows dialog → hide or relocate (relocate calls `rename_directory_meta` + `bootstrap_session`)
- **Bootstrap**: `bootstrap_session` opens a new Terminal, runs `opencode run "hi"` in the target directory, then self-deletes

## Style & Conventions

- Dark theme only, custom `surface` color palette (`bg-surface`, `bg-surface-card`, etc.)
- No ESLint/Prettier — only `tsc --noEmit` for type checking
- No router — state-driven navigation (`selectedDir`, `detailSession`)
- `@tanstack/react-virtual` `useVirtualizer` for virtualized session list
- Detail panel: CSS `@keyframes slide-in` from right
- No tests, no CI/CD
