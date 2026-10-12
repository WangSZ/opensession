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
                                   ├── db.rs           — read-only OpenCode SQLite
                                   ├── codex_db.rs     — read-only Codex SQLite
                                   ├── reasonix_db.rs  — read-only Reasonix JSONL metadata
                                   ├── session_repository.rs — cross-source aggregation
                                   ├── session_sources.rs    — source identity + CLI commands
                                   ├── config.rs       — opencode DB resolution, app dirs, summaries migration
                                   ├── commands/sessions.rs  — directory/session queries
                                   ├── commands/summary.rs   — agent CLI summarization
                                   ├── commands/terminal.rs  — cross-platform terminal launcher + bootstrap
                                   └── commands/meta.rs      — tags, pin, hidden, rename, file manager
```

Entrypoint: `main.rs` → `opensession_lib::run()` (lib crate `opensession_lib`, crate-type `["lib", "cdylib", "staticlib"]`).

Frontend uses `@tauri-apps/api/core` `invoke()`. Parameter names must match `#[tauri::command]` function arguments exactly.

All commands registered in `lib.rs` via `generate_handler![]`.

## Database

- OpenCode DB is resolved via `opencode db path`, with XDG and platform fallbacks
- Codex discovers the newest supported `state_*.sqlite` under `$CODEX_HOME` or `~/.codex`
- Reasonix scans `$REASONIX_HOME` or `~/.reasonix` for `sessions/*.jsonl.meta`, resolving each workspace from the companion `.jsonl` transcript
- SQLite sources are opened read-only (`SQLITE_OPEN_READ_ONLY`); Reasonix files are also only read
- `session` table `model` column is JSON `{"id":"...","providerID":"..."}`, parsed via `serde_json`
- `part` / `message` tables contain conversation messages and tool calls

## Persistent State (Outside DB)

| File | Location | Format |
|------|----------|--------|
| Directory metadata | `$XDG_CONFIG_HOME/opensession/directory_meta.json` | `HashMap<String, DirectoryMeta>` (tags, pinned, hidden) |
| Summary cache | `$XDG_DATA_HOME/opensession/summaries.json` | `HashMap<String, Summary>` — migrated from old `{opencode_data_dir}/opensession/summaries.json` on first run |
| Search scopes | `$XDG_CONFIG_HOME/opensession/search_scopes.json` | `Vec<SearchScopeDef>` — custom multi-directory search collections |
| File search log | `~/Library/Logs/opensession/file-search.log` (macOS) | per-search trace: jobs, exit codes, emits |

## Registered Tauri Commands

Merged directory groups are cached in-process for 1s (`session_repository::list_directory_groups_with_statuses`); meta/git-info overlays are applied fresh per call. Mutations that change source data (worktree add/remove) call `invalidate_directory_cache`. Git info (`commands/git_info.rs`) keeps its own in-memory cache (lazy-loaded from `git_info.json`, write-through), refreshes stale worktrees concurrently with a 5s per-repo cooldown, and emits `git_info://updated` only when a value actually changed.

| invoke name | Parameters | Returns |
|-------------|-----------|---------|
| `list_directories` | `includeMissing?: bool` | `DirectoryListResponse` — merged groups (missing dirs filtered server-side unless `includeMissing`), `total_count` pre-filter, per-source statuses |
| `get_sessions` | `directory: String` | `Vec<Session>` |
| `get_session_detail` | `sessionSource: SessionSource`, `sessionId: String` | `SessionDetail` |
| `search_directories` | `query: String` | `Vec<DirectoryGroup>` |
| `generate_summary` | `directory: String` | `()` — fire-and-forget, result via event |
| `get_cached_summary` | `directory: String` | `Option<Summary>` |
| `open_in_terminal` | `directory: String`, `sessionSource: SessionSource`, `sessionId?: String` | `()` |
| `bootstrap_session` | `directory: String`, `sessionSource: SessionSource` | `()` |
| `set_directory_tags` | `path: String`, `tags: Vec<String>` | `()` |
| `delete_tag_global` | `tag: String` | `()` |
| `toggle_directory_pin` | `path: String` | `bool` (new state) |
| `toggle_directory_hidden` | `path: String` | `bool` (new state) |
| `rename_directory_meta` | `old: String`, `new: String` | `()` — preserves tags/pin, resets hidden |
| `open_in_file_manager` | `directory: String` | `()` — Finder/Explorer/xdg-open |
| `reveal_file` | `path: String` | `()` — Finder reveal (`open -R`)/Explorer select |
| `start_search` | `searchId: String`, `query: String`, `mode: name\|content`, `scope: {kind: home\|global\|custom, id?}`, `limit?: usize` | `()` — mdfind-based, results via event |
| `cancel_search` | `searchId: String` | `()` |
| `list_search_scopes` | — | `Vec<SearchScopeDef>` |
| `add_search_scope` | `name: String`, `directories: Vec<String>` | `SearchScopeDef` |
| `delete_search_scope` | `id: String` | `()` |

## Platform Details

- macOS: writes a temp shell script → `osascript` → Terminal.app, then the script self-deletes
- Windows: `cmd /c start wt cmd /k "cd /d ... && <agent> ..."`
- Linux: `x-terminal-emulator -e bash -c "cd ... && <agent> ..."`
- The selected `opencode`, `codex`, or `reasonix` CLI must be on `PATH`
- Clipboard: `navigator.clipboard.writeText()` with `textarea` fallback — no Tauri plugin needed
- File search uses `/usr/bin/mdfind` (Spotlight). Query variants are expanded for Kangxi radicals (U+2F00–U+2FDF) since Spotlight name matching does not fold them. In name mode, queries containing `*` are translated to a conjunction of anchored Spotlight predicates (first literal → `seg*`, last → `*seg`, middles → `*seg*`, joined with `&&`) because mid-pattern `*` in a single `kMDItemFSName == "sop*.pdf"cd` / `LIKE` silently fails depending on tokenization. Predicates are only an approximation, so results are additionally glob-filtered locally against the file name (`*` only, case-insensitive; `?` and `[..]` are literal). On macOS the search probes Downloads/Documents/Desktop first: ad-hoc rebuilds change the cdhash and orphan TCC grants, so Spotlight silently filters protected folders until the probe-triggered prompt is granted

## Tauri Events

| Event | Payload | When |
|-------|---------|------|
| `summary://generated` | `{directory, summary?} \| {directory, error?}` | `generate_summary` background task completes |
| `search://results` | `{searchId, paths: string[], done, truncated}` | `start_search` streams mdfind results in batches |

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
| `SearchOverlay` | `src/components/SearchOverlay.tsx` | Cmd+Shift+F Spotlight-style file search (name/full-text, scopes, Kangxi-radical-aware highlight) |

## Command Copy (3 locations)

| Location | Content | Button |
|----------|---------|--------|
| SessionPanel header | `cd "/path" && <agent>` | copy icon |
| SessionCard | Source-specific resume command | copy icon |
| SessionDetail | Source-specific resume command | Copy button with text |

## Key UX Flows

- **Missing directory**: when user tries to open a deleted/moved directory, shows dialog → hide or relocate (relocate calls `rename_directory_meta` + `bootstrap_session`)
- **Bootstrap**: `bootstrap_session` opens the selected agent in a new Terminal in the target directory

## Style & Conventions

- Dark theme only, custom `surface` color palette (`bg-surface`, `bg-surface-card`, etc.)
- No ESLint/Prettier — only `tsc --noEmit` for type checking
- No router — state-driven navigation (`selectedDir`, `detailSession`)
- `@tanstack/react-virtual` `useVirtualizer` for virtualized session list
- Detail panel: CSS `@keyframes slide-in` from right
- No tests, no CI/CD
