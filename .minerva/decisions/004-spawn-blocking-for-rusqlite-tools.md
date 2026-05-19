# MCP tool handlers wrap rusqlite calls in `tokio::task::spawn_blocking`

**Date**: 2026-05-18
**Context**: .minerva/work/001-mcp-server

## Context

The MCP tool handler functions in `src-tauri/src/mcp_server/service/mod.rs` are `async fn` (required by rmcp's `#[tool]` macro). The underlying query layer in `queries.rs` is synchronous — it locks `DbConn(Mutex<rusqlite::Connection>)` and runs `rusqlite` calls, which are blocking I/O.

Existing Tauri commands in this repo (`commands/notes.rs`, `commands/tasks.rs`, etc.) call rusqlite directly from inside async functions without any blocking-pool offload. That's tolerated for typical UI calls because they're tiny and infrequent, but the MCP server can be hammered by an external client (a long agentic loop), and a slow query holding the runtime would degrade everything else (transcription streaming, audio capture, UI responsiveness).

Separately: clippy's `pedantic` includes `unused_async`, which fires on an `async fn` with no awaits. The query functions are sync; calling them directly from the tool handler would produce `async fn` bodies that contain zero awaits.

## Decision

Every MCP tool handler dispatches its query through `tokio::task::spawn_blocking`:

```rust
let db = self.db.clone();
let value = tokio::task::spawn_blocking(move || queries::list_notes(&db, &args))
    .await
    .map_err(internal)?  // JoinError
    .map_err(internal)?; // query error
json_result(&value)
```

## Consequences

- Tool handlers cannot be "simplified" by calling the query function directly. Don't strip the spawn_blocking. It both prevents runtime stalls and gives the `async fn` a real await so `unused_async` doesn't fire.
- The pattern of double `map_err(internal)?` (one for `JoinError` from spawn_blocking, one for the query's own `rusqlite::Error`) is intentional, not a copy-paste artifact.
- This decision applies to MCP tool handlers specifically. Existing Tauri command handlers (`commands/*.rs`) do NOT follow this pattern — they're called from the UI's request path, where the cost-benefit tradeoff is different. Don't retrofit blindly.
- If we ever add write-tools, they should follow the same pattern (writes block too) and additionally take a lock on any in-app state that mirrors the DB.
