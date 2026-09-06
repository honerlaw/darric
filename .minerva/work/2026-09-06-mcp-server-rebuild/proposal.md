# Proposal: mcp-server-rebuild

**Date**: 2026-09-06
**Status**: Shipped (2026-09-06)

## Goal

Rebuild a read-only MCP server inside the running darric app so Claude Code and any other
streamable-HTTP MCP client can list recordings, read transcripts including one still in
progress, search across all recordings, and see live recorder state. Always on, fixed loopback
port 27842, endpoint `/mcp`, with a small status chip in the header.

## Why

PR #1 shipped an MCP server in May against the old notes-and-tasks schema
(`.minerva/work/2026-05-19-mcp-server/proposal.md`), and PR #7 deleted it when darric was
stripped to a recorder. The recorder now produces the one thing worth analyzing:
device-attributed transcripts. Exposing them over MCP lets the Claude subscription the user
already pays for do the analysis, with no in-app AI and no token spend by darric.

In-process rather than a standalone file reader because one process is preferred and live state
such as which session is recording is wanted. Every transcript line is committed to SQLite the
moment whisper emits it (`audio/mod.rs::persist_and_emit`) and the database runs in WAL mode, so
a meeting in progress is queryable line by line with no extra plumbing.

## Approach

What shipped. See [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] for the
decision record.

### Dependencies

rmcp 3.2 (`server`, `macros`, `transport-streamable-http-server`, `schemars`), schemars 1.0,
axum 0.8, tokio-util 0.7. rmcp 3.2's `StreamableHttpServerConfig::default()` validates the
`Host` header against `localhost`, `127.0.0.1` and `::1`; that default is kept and
`disable_allowed_hosts` is never called, which is the DNS-rebinding floor
[[2026-05-19-decision-rmcp-as-mcp-sdk]] required. The four May decisions carry over unchanged:
`spawn_blocking` around every rusqlite call
([[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]]), the explicit
`#[tool_handler(router = self.tool_router)]` form
([[2026-05-19-decision-tool-handler-router-pattern]]), and inline tests
([[2026-05-19-decision-inline-tests-for-mcp-queries]]).

For the protocol test, rmcp's streamable-HTTP client is implemented for reqwest 0.13 while the
model downloader is on 0.12, so the test takes a renamed `reqwest13` dev-dependency
([[2026-09-06-reference-rmcp-3-streamable-http-client-needs-reqwest-0-13]]).

### Shared query layer

`src-tauri/src/db/sessions.rs` owns `Session` and `TranscriptLine` and the SQL over them:
`list_sessions(conn, limit, offset)`, `get_session`, `transcript_lines` (capture order, for
display), `transcript_page` (rowid order, for the cursor), and `search`. `commands/sessions.rs`
keeps its Tauri commands but calls these. `TranscriptLine` carries `seq`, the SQLite rowid; the
frontend type mirrors it as `seq: number | null`, null on a line the transcript hook appends live
from a `transcript_chunk`, which has no rowid until the transcript reloads.

Inline tests build their schema with `db::test_db()`, one helper over the production migration
list, replacing the per-file `include_str!` chains
([[2026-09-06-decision-one-test-db-helper-built-from-the-production-migration-list]]).

### Own read-only connection

`db::open` is split into `db::path()`, `db::open_at()` and `db::open_read_only()`. After the app
connection has migrated in `setup`, the server opens the same file with
`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` behind its own mutex and never runs migrations.
The recorder's inserts never wait on an agent query, and read-only is enforced by SQLite
([[2026-09-06-decision-mcp-server-reads-through-its-own-read-only-connection]]). Every query
still goes through `spawn_blocking`.

### Cursor

`get_transcript` pages by rowid: assigned under the insert mutex, monotonic, never tied, and
race-free where `recorded_at` is not. An empty page echoes the caller's cursor rather than
returning `None`, so a poll loop can always pass `next_cursor` straight back
([[2026-09-06-bug-an-empty-page-with-no-cursor-restarts-the-poll-loop]]).

`transcript_lines` has a `TEXT` primary key, so its rowid is implicit and is reassigned by
`VACUUM` or by a create-copy-drop-rename migration such as 010. Both can only run inside
`db::open` at startup, before the server exists, and a restart drops every MCP session, so a
cursor is valid for the life of the app process and the tool description says not to persist
one ([[2026-09-06-constraint-a-table-rebuild-renumbers-transcript-rowids-and-every-mcp-cursor]]).
Rowid order is transcription-completion order, not speech order, across devices; each line
carries `recorded_at` and the description says so.

### Server module

- `src-tauri/src/mcp_server/mod.rs` — `bind(port)` binds a std listener synchronously so
  `setup` learns the outcome before it returns; `serve(listener, db, live)` returns an
  `McpServerHandle` (whose `Drop` cancels) plus the serve future, which adopts the listener into
  Tokio and mounts rmcp's streamable HTTP service under axum at `/mcp`. The caller spawns the
  future on its own runtime ([[2026-09-06-constraint-tauri-setup-runs-outside-the-tokio-runtime]]).
- `src-tauri/src/mcp_server/service.rs` — `DarricService` with the read-only connection, an
  `Arc<dyn LiveStatus>`, and the tool router.
- `LiveStatus::snapshot` returns the engine's session id, per-device statuses and dropped
  count. Production implements it on `AppLiveStatus(AppHandle)` in `commands/mcp_server.rs`;
  the `status` tool fills in the live session's topic and `started_at` from its own connection.
  Tests pass a stub.

### Tools

| Tool             | Arguments                                                | Returns                                                                                                                                    |
| ---------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `status`         | none                                                     | `recording`; live session `id`, `topic`, `started_at` or `null`; per-device `id`, `name`, `direction`, `state`; `dropped_segments`         |
| `list_sessions`  | `limit?` (default 50, max 500), `offset?`                | `sessions[]` newest first: `id`, `topic`, `started_at`, `ended_at`, `recorded_minutes`, `in_progress`                                      |
| `get_transcript` | `session_id`, `after?`, `limit?` (default 500, max 2000) | `session`; `lines[]` of `seq`, `device_id`, `device_name`, `direction`, `content`, `recorded_at` in rowid order; `next_cursor`; `has_more` |
| `search`         | `query`, `limit?` (default 50, max 200), `session_id?`   | `sessions[]` whose topic matched and `lines[]` whose content matched (newest first, each with `seq`, `session_id`, `topic`, `started_at`)  |

- `search` is a `LIKE` over line content and session topic, case-insensitive for ASCII only:
  SQLite's `lower()` is ASCII-only, so the query is folded with `to_ascii_lowercase` to match.
  `%`, `_` and `\` are escaped. Device names are never matched. A topic match is a session, not
  every line of it, which is why the result is `{ sessions, lines }` rather than one `hits[]`.
- An unknown `session_id` and an empty `query` return `invalid_params`; a database failure
  returns `internal_error`. Limits are clamped to `[1, max]`.
- The `status` description notes that during the flush after Stop it reports
  `recording: false` while `list_sessions` still shows the session `in_progress`
  ([[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]]).

### Lifecycle

`lib.rs::start_mcp_server` runs after `app.manage(state)` and stores the outcome in
`AppState.mcp_server: Mutex<McpServerState>` — `Listening(handle)`, `PortBusy(reason)` when the
bind failed, or `Failed(reason)` for anything else. The app runs normally without the endpoint
in either failure. `commands/mcp_server.rs::mcp_server_status` returns
`{ listening, port, url, port_busy, error }`; the frontend reads it once on mount, because
events emitted during `setup` reach no webview
([[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]]).

### Chip

`src/components/McpChip.tsx`, rendered in `Header` left of the recording indicator through a
`mcpStatus` prop fed by `useMcpServer`. Hidden until status resolves. Listening: a button reading
`MCP · :27842` that copies `claude mcp add --transport http darric http://127.0.0.1:27842/mcp`
via `navigator.clipboard.writeText` and reads "Copied" for two seconds. Port busy:
`MCP · port busy`; any other failure: `MCP · off`; both carry the reason as `title`.

### Accepted tradeoffs

- Loopback means any local account on this machine can reach the endpoint, unauthenticated —
  a broader reach than the user-owned database file. Stated in the README.
- No retry on a busy port short of relaunching.
- Search is ASCII case-insensitive; full Unicode folding would need ICU, which the bundled
  SQLite lacks.

### Tests

- `db/sessions.rs`: paging in insertion order with an exact cursor, cursor echo on an empty
  page, session scoping, display order, LIKE escaping, ASCII folding on both sides,
  case-insensitivity and ordering, topic-as-session, session filter, device names never matched,
  list paging, `in_progress`.
- `mcp_server/mod.rs`: `bind` refuses a held port; a protocol round trip with rmcp's client
  against a temp-file database — tool list is exactly the four, `status` names the live session,
  `list_sessions` and `get_transcript` return seeded rows, then a fourth line inserted through
  the writer while the server is up comes back alone through the cursor, `search` finds a line,
  an unknown session is a protocol error.
- `mcp_server/service.rs`: `status` with no engine.
- `McpChip.test.tsx`: hidden, listening + copy, copied timeout, port busy, off, command text.
  `Header.test.tsx`: the chip renders through the header once status is known.
  `App.test.tsx` mocks `mcp_server_status`.

### Verification

Against the debug binary and the real database: the raw streamable-HTTP handshake and
`tools/list`; `status` and `list_sessions` answering from real recordings; `Host: evil.example`
rejected with 403; `claude mcp add --transport http …` then `claude mcp list` showing
`✔ Connected` (entry removed afterwards); a second instance launched while the first held
27842 logging `not started, port busy` and staying up.

### Docs

README section "Query darric from Claude": the `claude mcp add` line, the four tools, loopback
and read-only, local-account reach, `port busy` versus `off`, ASCII search, transcription order.

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
