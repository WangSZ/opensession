# Codex Session Source Design

## Objective

Extend OpenSession from an OpenCode-only session manager into a unified project-directory session manager for OpenCode and Codex.

The directory remains the primary navigation entity. Sessions from both tools are merged under the same normalized directory, labeled by source, and filterable by source. Codex support is limited to user-facing interactive sessions that can be resumed. Both resume and fork are supported.

## Scope

### Included

- Discover OpenCode and Codex sessions independently.
- Merge sessions that belong to the same directory.
- Label every session as `opencode` or `codex`.
- Filter a directory's sessions by all, OpenCode, or Codex.
- Resume and fork sessions using the correct CLI.
- Preserve session notes, pinning, hiding, and Issue links across the multi-source migration.
- Start a new OpenCode or Codex session from directory-level actions.
- Degrade gracefully when either source is missing or incompatible.

### Excluded From The First Release

- Codex session archive or deletion.
- Full conversation browsing.
- Live or running-session monitoring.
- Codex Cloud task management.
- Codex `exec`, review, compact, and sub-agent threads.
- Parsing Codex rollout JSONL files for additional metrics.

## Architecture

Introduce a source-neutral repository above two isolated adapters:

```text
OpenCodeAdapter --+
                  +--> SessionRepository --> Tauri commands --> React UI
CodexAdapter -----+
```

Each adapter owns source-specific discovery, schema validation, database queries, and model mapping. `SessionRepository` owns directory normalization, cross-source aggregation, sorting, composite session lookup, and source status reporting.

```rust
trait SessionAdapter {
    fn status(&self) -> SourceStatus;
    fn list_directories(&self) -> Result<Vec<SourceDirectory>, String>;
    fn get_sessions(&self, directory: &str) -> Result<Vec<Session>, String>;
    fn get_session_detail(&self, id: &str) -> Result<SessionDetail, String>;
}
```

The existing OpenCode queries move behind `OpenCodeAdapter` without changing their behavior. `CodexAdapter` reads Codex's local SQLite database in read-only mode.

The adapter boundary is required because Codex's state database is an internal storage format. If the Codex app-server protocol becomes stable and preferable, it can replace the internals of `CodexAdapter` without changing Tauri commands or frontend models.

## Codex Discovery And Filtering

Resolve Codex storage in this order:

1. `$CODEX_HOME`, when set.
2. `~/.codex`.

Within the resolved directory, scan `state_*.sqlite` candidates in descending numeric version order. Select the first candidate whose `threads` table contains all required columns. Do not hard-code `state_5.sqlite`.

Required Codex fields include:

- `id`
- `cwd`
- `title`
- `preview`
- `source`
- `model_provider`
- `model`
- `tokens_used`
- `created_at_ms` or a supported timestamp fallback
- `updated_at_ms` or a supported timestamp fallback
- `archived`
- `has_user_event`

Only include threads that meet all of these rules:

- Not archived.
- Contain a user event.
- Belong to an interactive source supported by the Codex resume picker, such as CLI, VS Code, or app-server.
- Are not `exec`, review, compact, or sub-agent threads.

The filter should be expressed in one source-specific function and covered by fixture tests so schema and source changes are visible.

## Domain Model

Model the application that owns a session separately from the model API provider:

```ts
export type SessionSource = "opencode" | "codex";

export interface SessionRef {
  source: SessionSource;
  id: string;
}

export interface TokenUsage {
  input?: number | null;
  output?: number | null;
  total?: number | null;
}

export interface Session {
  source: SessionSource;
  id: string;
  directory: string;
  title: string;
  first_message: string;
  model_name?: string | null;
  model_provider?: string | null;
  token_usage?: TokenUsage | null;
  cost?: number | null;
  file_changes?: number | null;
  // Existing timestamps, note, pin, hidden, and hierarchy fields remain.
}
```

All session operations use `SessionRef`, not a bare session ID. Persisted map keys use `<source>:<id>`, for example `opencode:ses_123` or `codex:019f...`.

`DirectoryGroup` adds source counts:

```ts
source_counts: {
  opencode: number;
  codex: number;
}
```

`session_count` remains the combined total. `total_cost` means known cost only because Codex does not expose a comparable local cost value. The UI must not imply that it contains Codex cost.

## Directory Identity And Aggregation

Normalize absolute directory paths before aggregation. Existing directories may be canonicalized when the filesystem entry exists; missing paths retain a cleaned absolute representation so relocation metadata still works.

For each normalized path:

- Merge OpenCode and Codex source records.
- Sum session counts.
- Preserve the most recent activity timestamp.
- Sum known OpenCode cost only.
- Compute `source_counts` independently.
- Apply the existing directory tags, pin, hidden, Git, worktree, summary, and missing-directory metadata once.

Session ordering remains newest-first by updated timestamp, with the existing pinning behavior applied in the frontend.

The two adapters load independently. Failure of one adapter must not discard results from the other.

## Persistence Migration

### Session Metadata

Current `session_meta.json` entries are keyed by bare OpenCode session ID. On load:

- Treat keys without a recognized source prefix as OpenCode.
- Convert them to `opencode:<id>` in memory.
- Persist the migrated format on the next successful write or through an explicit one-time migration.

All new hide, pin, and note commands receive both `sessionSource` and `sessionId`.

### Issue Links

Add `session_source` to `IssueLink` and `LinkedSession`. Existing links without the field default to `opencode` through Serde defaults.

Index `issue_links` by the composite storage key, not the bare session ID. All link, unlink, and lookup commands receive a session source. Directory-level Issue links remain keyed only by normalized directory because directories are shared across sources.

Migration must be idempotent and preserve every existing note, pin, hidden flag, and Issue association.

## Tauri Commands

Source-neutral commands retain their current names where practical, but every session-specific command adds `sessionSource`:

- `get_session_detail(sessionSource, sessionId)`
- `open_in_terminal(directory, sessionSource?, sessionId?)`
- `fork_session(directory, sessionSource, sessionId)`
- `toggle_session_hidden(sessionSource, sessionId)`
- `toggle_session_pin(sessionSource, sessionId)`
- `set_session_note(sessionSource, sessionId, note)`
- `link_session_to_issue(sessionSource, sessionId, issueId, directory)`
- `unlink_session_from_issue(sessionSource, sessionId)`
- `get_session_issue(sessionSource, sessionId)`

Frontend `invoke()` parameter casing must continue to match the Rust command arguments exactly.

Command construction is centralized by source:

| Source | Start | Resume | Fork |
|---|---|---|---|
| OpenCode | `opencode` | `opencode --session <id>` | `opencode --session <id> --fork` |
| Codex | `codex` | `codex resume <id>` | `codex fork <id>` |

The platform-specific terminal launcher should receive a structured command description rather than duplicating source conditionals in macOS, Windows, and Linux branches. Session IDs are validated and arguments are shell-escaped before generating a terminal script.

Worktree creation and missing-directory relocation accept a selected source for the new session. The most recently selected source may be remembered locally as the default, but the user can always choose another available source.

## User Interface

### Session Filtering

Add a segmented control to the session toolbar:

```text
[ All 11 ] [ OpenCode 8 ] [ Codex 3 ]
```

The default is All. The selected filter remains active while navigating between directories during the application session. A source with zero sessions is disabled.

Source filtering is applied before text search, hidden-session filtering, pin sorting, and virtualization.

### Session Cards

- Display a stable, text-based OpenCode or Codex source badge beside the title.
- Build Resume, Fork, context menu, and copied command actions from `session.source`.
- Display only metrics that exist for that source.
- For Codex, show total tokens when available; do not display zero cost, zero file changes, or fabricated input/output token values.
- Use the composite session identity for React keys, highlighting, Issue maps, and context-menu state.

### Session Detail

- Display session source separately from model provider.
- Generate the source-specific command.
- Render metric blocks conditionally.
- Keep the current first-message and timeline behavior where data is available.

### Directory Actions

Replace the OpenCode-only `Open` behavior with a compact source menu:

- Open with OpenCode.
- Open with Codex.
- Open plain terminal.

The session panel header displays the directory path and source-specific launch buttons instead of a hard-coded OpenCode command. Directory context menus expose both source launch actions when available.

New-directory and worktree flows ask which available source should be started. The UI may preselect the most recently used source.

## Summaries

Directory summaries remain keyed by normalized directory and combine recent titles from both adapters.

Summary execution is independent of session storage:

1. Prefer OpenCode when it is available, preserving current behavior.
2. Fall back to `codex exec` with a read-only sandbox when OpenCode is unavailable.
3. Disable generation and report a source-level error when neither CLI is available.

The summarizer must not create or modify files in the target repository.

## Source Status And Error Handling

Replace the single OpenCode database-not-found state with per-source status:

```rust
enum SourceStatus {
    Available,
    NotInstalled,
    NoSessions,
    UnsupportedSchema,
    ReadError,
}
```

The UI shows concise source availability information. Detailed paths and database errors go to application logs. One failed source never blocks the other source or shared metadata features.

Schema mismatch must be explicit. Do not silently return an empty Codex list when the database exists but required columns are unsupported.

## Testing

Add Rust unit and fixture tests for:

- Merging OpenCode and Codex sessions under the same directory.
- Keeping different directories separate after normalization.
- Filtering archived, empty, `exec`, review, compact, and sub-agent Codex threads.
- Mixed-session sorting and source counts.
- Source-specific start, resume, fork, and copy command construction.
- Safe path and session ID handling.
- Migration of legacy `session_meta.json` keys.
- Migration of legacy Issue links.
- Isolation when OpenCode and Codex use the same session ID.
- Returning `UnsupportedSchema` for an incompatible Codex fixture.
- Returning OpenCode data when Codex loading fails, and vice versa.
- Serializing unavailable metrics as `null` rather than zero.

Frontend verification covers:

- All/OpenCode/Codex filter counts and disabled states.
- Source badges and conditional metrics.
- Source-aware Resume, Fork, copy, and detail actions.
- Composite keys in Issue views and highlighted-session navigation.
- Per-source empty and error states.

Required verification commands:

```bash
npm run build
cargo test
cargo check
```

Manual verification uses real OpenCode and Codex data to confirm directory aggregation, source filtering, resume, fork, new-session launch, metadata migration, and single-source degradation.

## Implementation Sequence

1. Add source-neutral domain types, composite session identity, and persistence migrations.
2. Move existing OpenCode queries behind `OpenCodeAdapter` without behavior changes.
3. Add Codex path resolution, schema validation, query mapping, and fixture tests.
4. Add `SessionRepository` directory aggregation and per-source status.
5. Generalize terminal command construction and add Codex start, resume, and fork.
6. Update Tauri commands and frontend command wrappers to pass session source.
7. Add source filtering, badges, conditional metrics, and provider launch menus.
8. Update Issue views, summary title aggregation, empty states, and migration handling.
9. Run automated verification and manual mixed-source smoke tests.

## Acceptance Criteria

- A directory used by both tools appears exactly once.
- Its session list contains labeled OpenCode and Codex sessions.
- All, OpenCode, and Codex filters show correct counts and results.
- Only resumable interactive Codex sessions are shown.
- Resume and fork launch the correct CLI for each session.
- Existing OpenCode notes, pinning, hiding, tags, summaries, and Issue links remain intact.
- Bare session ID collisions across sources do not cross-link metadata.
- OpenSession remains usable when only OpenCode or only Codex is installed.
- A Codex schema mismatch is reported without suppressing OpenCode data.
- No database is opened writable and no target repository is modified during discovery or summary generation.
