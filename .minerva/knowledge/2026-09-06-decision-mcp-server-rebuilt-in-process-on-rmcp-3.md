# The MCP server is back in-process, on rmcp 3.2, always on at 127.0.0.1:27842

**Date**: 2026-09-06
**Type**: decision
**Summary**: darric again hosts a read-only MCP server in-process — rmcp 3.2, streamable HTTP on 127.0.0.1:27842/mcp, always on, four tools — reviving the four May 2026 MCP decisions the strip retired
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

PR #1 shipped an MCP server in May 2026 against the notes-and-tasks schema; PR #7 deleted
it with the rest of the product surface when darric was stripped to a recorder. The recorder
now produces device-attributed transcripts, and the user wants Claude Code — the subscription
they already pay for — to analyse them, including a meeting still in progress. A standalone
stdio binary reading the SQLite file would have worked for that (every line is committed as
whisper emits it), but the user chose one process and live recorder state.

## Finding

The server lives in `src-tauri/src/mcp_server/` and starts in `setup`, always, on the fixed
loopback port 27842 at `/mcp`. There is no settings toggle; a header chip shows the port and
copies `claude mcp add --transport http darric http://127.0.0.1:27842/mcp`, or reads
`port busy` / `off` with the reason as its title. Four tools: `status` (engine snapshot plus
the live session), `list_sessions`, `get_transcript` (rowid cursor), `search` (content and
topic, ASCII case-insensitive, returning `{ sessions, lines }`).

rmcp moved from 1.7 to 3.2 across the gap. Everything the May decisions depended on survived
— `StreamableHttpService`, `LocalSessionManager`, `with_cancellation_token`, `Parameters`,
`#[tool_handler(router = self.tool_router)]` — and `StreamableHttpServerConfig::default()`
now validates the `Host` header against `localhost`/`127.0.0.1`/`::1` by default, so the
DNS-rebinding floor the 1.4 pin existed for is met without configuration; `disable_allowed_hosts`
must never be called. A `Host: evil.example` probe returns 403.

## Implications

- The four May decisions are live again, not retired: `spawn_blocking` around every rusqlite
  call, the explicit `router =` handler form, inline tests, and rmcp as the SDK.
- Loopback means any local account on the machine can read the endpoint — a broader reach
  than the user-owned database file. Accepted; the README states it.
- A busy port is non-fatal: the app records normally without the endpoint. There is no retry
  short of relaunching.
- The bind is synchronous in `setup`, so the outcome is known before the webview exists and
  the frontend reads it with one command on mount.

## Related

- [[2026-05-19-decision-rmcp-as-mcp-sdk]] — builds on
- [[2026-09-05-decision-strip-darric-to-a-recorder]] — that strip retired the MCP server this revives
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — still governs every tool handler
- [[2026-05-19-decision-tool-handler-router-pattern]] — still the only form the lint policy allows
- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — the tests are inline again
- [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]] — why the chip polls a command instead of listening for an event
