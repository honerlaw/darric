# The MCP server reads through its own read-only SQLite connection

**Date**: 2026-09-06
**Type**: decision
**Summary**: MCP tools query a second `SQLITE_OPEN_READ_ONLY` connection, never the app's write connection, so an agent's query cannot delay a transcript insert and cannot write by drift
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

The May 2026 server shared `AppState.db` — one `Mutex<Connection>` — with the recorder. Every
transcript line is inserted under that mutex the moment whisper emits it, and an external agent
in a loop can query far harder than the UI ever does. A long `search` scan on the shared
connection would have made the recorder's insert wait for it, and "read-only" was a convention
the tools happened to follow.

## Finding

`db::open` is split into `db::path()` / `db::open_at()` / `db::open_read_only()`. After the
app connection has migrated in `setup`, the server opens the same file again with
`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` behind its own mutex and never runs
migrations. WAL mode lets that reader observe every commit the writer makes: the protocol test
inserts a line through the writer while the server is up and reads it back through the
cursor. `spawn_blocking` still wraps every query — the connection is separate, but rusqlite is
still blocking I/O on the runtime that drives capture.

## Implications

- Read-only is enforced by SQLite, not by the tool authors remembering.
- The reader must open after the writer has migrated: a reader must never rebuild tables
  underneath a writer, and the writer creates the `-wal`/`-shm` companions the reader needs.
- An agent query can still cost CPU and disk on the same machine as whisper; what it cannot do
  is hold the recorder's mutex. "Cannot block the recorder's writes" is the honest claim.
- `db::open_at` is what lets a test stand up a real on-disk database that a second connection
  observes — the arrangement the app runs under, rather than an in-memory stand-in.

## Related

- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the server this connection serves
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — still required; a separate connection does not make rusqlite async
- [[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]] — why `status` reports not-recording while the reader still sees the session in progress
