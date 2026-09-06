# Proposal: mcp-server-rebuild

**Date**: 2026-09-06
**Status**: Draft

## Goal

Rebuild a read-only MCP server inside the running darric app so Claude Code and any other
streamable-HTTP MCP client can list recordings, read transcripts including one still in
progress, search across all recordings, and see live recorder state. Always on, fixed loopback
port 27842, endpoint `/mcp`, with a small status chip in the header.

## Why

PR #1 shipped an MCP server in May against the old notes-and-tasks schema
(`.minerva/work/2026-05-19-mcp-server/proposal.md`), and PR #7 deleted it when darric was stripped to a recorder. The
recorder now produces the one thing worth analyzing: device-attributed transcripts. Exposing
them over MCP lets the Claude subscription the user already pays for do the analysis, with no
in-app AI and no token spend by darric.

In-process rather than a standalone file reader because one process is preferred and live state
such as which session is recording is wanted. Every transcript line is committed to SQLite the
moment whisper emits it (`audio/mod.rs::persist_and_emit`) and the database runs in WAL mode, so
a meeting in progress is queryable line by line with no extra plumbing.

## Approach

### Dependencies

rmcp 3.2 with features `server`, `macros`, `transport-streamable-http-server`, `schemars`;
schemars 1.0; axum 0.8; tokio-util 0.7. rmcp 3.2 validates the `Host` header by default against
`localhost`, `127.0.0.1` and `::1`, which is the DNS-rebinding floor
[[2026-05-19-decision-rmcp-as-mcp-sdk]] required, so the default is kept and
`disable_allowed_hosts` is never called.

The four standing MCP knowledge decisions carry over unchanged: rmcp as the SDK,
`spawn_blocking` around every rusqlite call
([[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]]), the explicit
`#[tool_handler(router = self.tool_router)]` form
([[2026-05-19-decision-tool-handler-router-pattern]]), and inline `#[cfg(test)]` tests
([[2026-05-19-decision-inline-tests-for-mcp-queries]]). rmcp 3.2 still exposes every name that
form depends on: `StreamableHttpService`, `StreamableHttpServerConfig::with_cancellation_token`,
`LocalSessionManager`, and the `Parameters` wrapper.

### Shared query layer

New `src-tauri/src/db/sessions.rs` owns the `Session` and `TranscriptLine` structs and four
functions over a `&rusqlite::Connection`:

- `list_sessions(conn, limit, offset)` — newest first, with `recorded_minutes` and
  `in_progress` (`ended_at IS NULL`).
- `get_session(conn, id)` — one session or `None`.
- `transcript_page(conn, session_id, after: Option<i64>, limit)` — lines in rowid order,
  returning `next_cursor` and `has_more`.
- `search(conn, query, session_id: Option<&str>, limit)` — matching lines with their session's
  id, topic and `started_at`, newest first.

`commands/sessions.rs` keeps its Tauri commands but calls these instead of holding SQL inline.
`TranscriptLine` gains `seq: i64`, the SQLite rowid, mirrored as `seq: number` in
`src/types/index.ts`. It is the stable per-line key the UI currently lacks, and it is what the
MCP cursor and search hits carry.

### Own read-only connection

`db::open` splits into `db::path() -> PathBuf` and `db::open()`. After the app connection has
run migrations in `setup`, the server opens the same file with `SQLITE_OPEN_READ_ONLY` behind its
own `Mutex`, and never runs migrations. The recorder's inserts never wait on an agent query, and
read-only is enforced by SQLite rather than by convention. `spawn_blocking` still wraps every
query: the connection is the server's own, but rusqlite is still blocking I/O on a Tokio
runtime that also drives capture.

### Cursor

Transcript paging is by rowid, not `recorded_at`. The rowid is assigned under the insert mutex,
is monotonic, and never ties, so "everything after cursor N" is exact even when two devices land
lines in the same millisecond, and it is race-free where `recorded_at` is not: a worker computes
`Utc::now()` before taking the lock, so two lines can be timestamped in one order and inserted in
the other.

`transcript_lines` has a `TEXT PRIMARY KEY`, so its rowid is implicit and would be renumbered by
`VACUUM`. Nothing in darric runs `VACUUM` today. Adding one would invalidate every cursor an
agent holds; if that ever becomes necessary the cursor must move to an explicit
`INTEGER` column first.

### Server module

- `src-tauri/src/mcp_server/mod.rs` — binds `127.0.0.1:27842`, mounts rmcp's streamable HTTP
  service under axum at `/mcp`, and returns an `McpServerHandle` whose `Drop` cancels the serve
  task via a `CancellationToken`.
- `src-tauri/src/mcp_server/service.rs` — `DarricService` holding the read-only connection, an
  `Arc<dyn LiveStatus>`, and the `ToolRouter`.
- `LiveStatus` is a trait with one method returning the recorder snapshot. Production implements
  it on a struct holding the Tauri `AppHandle`, reading `AppState.engine` under a short lock.
  Tests pass a stub, because `AppHandle` cannot be constructed in one.

### Tools

| Tool | Arguments | Returns |
| --- | --- | --- |
| `status` | none | `recording: bool`; live session `id`, `topic`, `started_at` or `null`; per-device `id`, `name`, `direction`, `state`; `dropped_segments` |
| `list_sessions` | `limit?` (default 50, max 500), `offset?` (default 0) | newest first: `id`, `topic`, `started_at`, `ended_at`, `recorded_minutes`, `in_progress` |
| `get_transcript` | `session_id`, `after?` (cursor), `limit?` (default 500, max 2000) | the session header; `lines[]` of `seq`, `device_id`, `device_name`, `direction`, `content`, `recorded_at` in rowid order; `next_cursor`; `has_more` |
| `search` | `query`, `limit?` (default 50, max 200), `session_id?` | `hits[]` of `seq`, `session_id`, `topic`, `started_at`, `device_name`, `direction`, `content`, `recorded_at`, newest first |

- `search` is a case-insensitive `LIKE` over line content and session topic only, never device
  names. `%` and `_` in the query are escaped so a literal underscore does not become a wildcard.
- An unknown `session_id` and an empty `query` return `invalid_params`; a database failure
  returns `internal_error`.
- All responses are JSON via `serde_json::Value`, as PR #1 did.
- The `status` tool's description states that during the flush after Stop it reports
  `recording: false` while `list_sessions` still shows the session `in_progress`, because
  `stop_session` takes the engine out on entry and writes `ended_at` only after whisper drains
  ([[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]]).

### Lifecycle

In `lib.rs::run`'s `setup`, after `app.manage(state)`, spawn the server and store the outcome in
a new `AppState.mcp_server: Mutex<McpServerState>` field: `Listening(McpServerHandle)` or
`Failed(String)`. A busy port logs the error and the app runs normally without the endpoint. The
handle lives as long as `AppState`, so the server stops when the app exits.

A new `commands/mcp_server.rs::mcp_server_status` command returns `{ listening, port, url,
error }`. The frontend calls it once on mount rather than listening for an event, because events
emitted during `setup` reach no webview
([[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]]).

### Chip

`src/components/McpChip.tsx`, rendered in `Header` left of the recording indicator. Hidden until
status resolves. When listening it reads `MCP · :27842`; clicking writes

```
claude mcp add --transport http darric http://127.0.0.1:27842/mcp
```

to the clipboard through `navigator.clipboard.writeText`, which PR #1 shipped from the same
webview, and the chip reads "Copied" for two seconds. When bind failed it reads
`MCP · port busy` with the error as its `title`. A new `src/lib/tauri.ts` wrapper and a
`useMcpServer` hook carry the status.

### Tests

- Inline tests in `db/sessions.rs` for paging, cursor exactness across interleaved devices,
  search escaping, the `session_id` filter, and the `in_progress` flag. The test helper builds
  its in-memory database from `db::migrations::migrations()` (made `pub(crate)`) rather than a
  second hand-maintained `include_str!` list, so a new migration cannot leave the helper behind.
- One protocol round-trip test in `mcp_server` using rmcp's client under `[dev-dependencies]`
  (features `client`, `transport-streamable-http-client-reqwest`): spawn on port 0 against a
  temp-file database seeded with one session and three lines, plus a stub `LiveStatus`;
  initialize; assert the tool list is exactly `status`, `list_sessions`, `get_transcript`,
  `search`; call `list_sessions` and `get_transcript` and check the seeded rows come back.

### Docs

A README section "Query darric from Claude" with the `claude mcp add` line, the four tools in
one sentence each, and a note that the server is loopback-only and read-only.

## Success criteria

- With darric running, `claude mcp add --transport http darric http://127.0.0.1:27842/mcp`
  connects, and the client's tool list shows exactly `status`, `list_sessions`,
  `get_transcript`, `search`.
- During a live recording, `get_transcript` called twice with the returned `next_cursor` yields
  on the second call only lines transcribed between the calls, and `status` names that session.
- `search` for a phrase spoken in an older recording returns that session and the line.
- With another process holding port 27842, darric starts and records normally and the chip
  reads `MCP · port busy`.
- Rust tests cover the query layer and one protocol round trip; `npm run check` passes with no
  new lint suppressions.

## Open Questions

None.
