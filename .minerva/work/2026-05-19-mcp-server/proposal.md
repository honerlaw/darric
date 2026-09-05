# Proposal: mcp-server

**Date**: 2026-05-18
**Status**: Shipped

## Goal

Expose darric's local data (notes, meetings + transcripts, tasks, tags, timeline) as an MCP server hosted by the running Tauri app, over HTTP on a fixed loopback port, so external local AI tooling (Claude Desktop, Claude CLI, other MCP-aware clients) can read darric without darric consuming any of its own bundled AI budget.

Scope of v1: read-only, tools-only API, trust-loopback, requires darric.app to be running.

## Why

darric currently consumes AI: users provide a Claude or Gemini API key in Settings, and the in-app harness pays for tokens. That works for self-hosted use but creates friction:

1. **Distribution friction.** Anyone trying darric needs their own Anthropic/Gemini key. There is no clean way to piggyback on a Pro/Max sub from a third-party app (Anthropic's Feb 2026 ToS clarification explicitly bans it, including via the Agent SDK) and no public OAuth to lean on.
2. **Cost asymmetry for power users.** A user with an existing Claude Pro/Max sub or Claude Code seat already has Claude available somewhere. Asking them to also pay API usage to query their own notes is wasteful.
3. **Inversion is free leverage.** darric already has the data in SQLite. Exposing it as MCP means any MCP-aware client the user already pays for can query darric, with no incremental token spend by darric.
4. **Strategic positioning.** "darric is a personal-data MCP server" is a more durable identity than "darric is yet another AI chat wrapper." The agent-first vision aligns with being a data source that agents read, not just a UI that talks to agents.

## Approach

### Architecture

A new module `src-tauri/src/mcp_server/` lives alongside the existing `src-tauri/src/ai/mcp/` (which is the client side). The Tauri app reads `mcp_server.enabled` and `mcp_server.port` settings at startup and, if enabled, spawns a tokio task that binds `127.0.0.1:<port>` and serves until the app exits. The `McpServerHandle` stored in `AppState.mcp_server` cancels the serve task on drop.

### Stack

- `axum 0.8` for the HTTP listener.
- `rmcp 1.7` (the official `modelcontextprotocol/rust-sdk`) for protocol framing, with features `server, macros, transport-streamable-http-server, schemars`. See [knowledge/2026-05-19-decision-rmcp-as-mcp-sdk](../../knowledge/2026-05-19-decision-rmcp-as-mcp-sdk.md) for the version pin and CVE rationale.
- The existing `state::DbConn(Mutex<rusqlite::Connection>)` is shared with the rest of the app. WAL mode is already on, so concurrent reads are safe.

### Transport

Streamable HTTP at `POST /mcp`. Single endpoint, SSE for streaming, session managed by rmcp's `LocalSessionManager`.

### Tool surface (v1, shipped)

| Tool                                    | Purpose                                                                                                |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `list_notes(limit?, offset?)`           | Note metadata + tag list, paginated, newest-updated first                                              |
| `get_note(id)`                          | Full note body + tags                                                                                  |
| `search(query, limit?)`                 | Substring search (case-insensitive `LIKE`) across notes, meeting topics + transcripts, and task titles |
| `list_meetings(limit?, since?, until?)` | Meeting metadata + recorded minutes + tags                                                             |
| `get_meeting(id)`                       | Meeting + full transcript; transcripts beyond 64 KB are truncated with a `truncated_at_bytes` field    |
| `list_tasks(status?, tag?)`             | Tasks ordered by `(col, position)`, optionally filtered by column or tag name                          |
| `list_tags()`                           | All tags, alphabetical                                                                                 |
| `by_tag(tag, types?)`                   | Notes/meetings/tasks with the given tag; `types` filters which kinds are returned                      |
| `timeline(from?, to?, types?, limit?)`  | Chronological combined view; merges per-table queries in Rust                                          |

All responses are JSON-serialized via `serde_json::Value`. All tool handlers dispatch their query through `tokio::task::spawn_blocking` — see [knowledge/2026-05-19-decision-spawn-blocking-for-rusqlite-tools](../../knowledge/2026-05-19-decision-spawn-blocking-for-rusqlite-tools.md).

### Lifecycle and settings

Settings keys:

- `mcp_server.enabled` (string `"true"`/`"false"`, default `true`)
- `mcp_server.port` (string-encoded `u16`, default `27842`)

Settings UI: a new section in `SettingsModal.tsx` with toggle, port input, status indicator, and a "Copy snippet" button that writes a Claude Desktop config fragment to the clipboard.

**Known v1 limitation:** changes to `mcp_server.*` settings require an app restart. `commands/settings.rs::save_setting` rebuilds the AI harness on `ai.*` keys but does not restart the MCP server. The hook point is documented in `archive/scratchpad.md` for future work.

If the port is taken at startup, the server logs the error and the app continues running without the MCP endpoint. The status command surfaces `listening: false` to the UI.

### Tests

13 unit tests under `#[cfg(test)] mod tests` inside `mcp_server/service/queries.rs`, covering each tool's happy path plus filter behavior and transcript truncation. Inline tests instead of integration tests — see [knowledge/2026-05-19-decision-inline-tests-for-mcp-queries](../../knowledge/2026-05-19-decision-inline-tests-for-mcp-queries.md).

### Out of scope for v1 (deferred)

- Writes (no create/update/delete from MCP).
- Resources (tools-only — Claude Desktop and the spec accept this).
- Auth (trust loopback; any process running as the user can read the SQLite file directly anyway).
- FTS5 search. `search` uses `lower(...) LIKE '%query%'`; works fine on personal-scale data. Migration to FTS5 would land as migration 009 and rewrite the search query.
- Live in-memory state. Everything reads from SQLite, so in-flight transcription buffers aren't visible until flushed — matches existing app behavior.
- Hot-reload of `mcp_server.*` settings.
