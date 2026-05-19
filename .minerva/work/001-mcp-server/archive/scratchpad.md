# Scratchpad: mcp-server

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `/promote`: significant items get promoted to
> `decisions/`, `proposal.md` gets updated to match reality, and the raw
> scratchpad is archived.

## Decisions worth capturing

- **rmcp pinned at `1.7`.** Latest stable, includes the Host-header validation fix (CVE on DNS rebinding in streamable HTTP server transport landed in 1.4). Features: `server, macros, transport-streamable-http-server, schemars`. Axum 0.8 confirmed via rmcp's dev-deps.
- **`#[tool_handler(router = self.tool_router)]` not bare `#[tool_handler]`.** Without `router = self.tool_router`, the macro calls `Self::tool_router()` (static fn) per request and the stored field is unused → `dead_code` warning. Adding the explicit `router = …` arg both binds the field and avoids the warning without an `#[allow]` (which CLAUDE.md forbids outside test modules). The rmcp counter example masks this with `#![allow(dead_code)]` at the file level — we cannot.
- **`Implementation` and `ServerInfo` (alias for `InitializeResult`) are `#[non_exhaustive]`.** Cannot use struct-literal `{…}` form. Use `Implementation::new(name, version)` and `ServerInfo::new(caps).with_server_info(…).with_instructions(…)`.
- **Inline `#[cfg(test)] mod tests` inside `queries.rs` instead of integration tests.** The `mcp_server` module is `mod` (not `pub mod`) in `lib.rs`, so `tests/` cannot reach `queries::*`. Making it `pub mod` would expand the crate's public surface for tests only. Inline tests with duplicated migration `include_str!` chain is the smaller hit.
- **All tool handlers wrap query calls in `tokio::task::spawn_blocking`.** rusqlite is sync and holds a `std::sync::Mutex<Connection>`. Spawning blocking keeps the runtime responsive and satisfies the implicit "awaits exist" requirement (otherwise clippy's `unused_async` could fire on a pedantic build, since tools are required to be `async fn`).
- **No `tools/list`-vs-router caching concern.** rmcp's `ToolRouter` is built once via `Self::tool_router()` in `DarricService::new`, the service factory creates a new service-with-cloned-Arc per session. The clone is cheap (Arcs).

## Things future-me might want to revisit

- **Settings changes for `mcp_server.*` require app restart.** Out-of-scope per proposal. If/when we want hot-reload, the pattern is in `commands/settings.rs` — it already rebuilds `AgentHarness` on `ai.*` keys. Would need to lock `state.mcp_server`, drop the old handle (Drop cancels), then re-spawn.
- **`commands/settings.rs::save_setting` does NOT trigger an MCP server rebuild.** Left for v2. Today, port/enabled changes silently persist until next launch.
- **Timeline tool is a fresh query, not a reuse of `commands/timeline.rs`.** There's no `commands/timeline.rs` today — the frontend's `TimelineScreen.tsx` composes from `list_sessions + list_notes + list_tasks + chat_history`. The MCP `timeline` tool queries each table directly with date filters and merges in Rust. Slight contract drift risk if the UI's timeline semantics ever diverge.
- **FTS5 not yet adopted.** `search` uses `lower(...) LIKE '%query%'`, which works but is O(n) per table. Migration to FTS5 would touch a new migration + `search` + per-entity `search_*` patterns. Migration 009 would be the home.
- **No auth.** Trust-loopback per proposal. If we ever need it, the snippet generator in `SettingsModal.tsx::buildClaudeDesktopSnippet` is where token would be inserted.

## Tried-and-dropped

- **`mod queries;` as a sibling file next to `service.rs`.** Initially had `mcp_server/service.rs` with `mod queries;` — Rust looks for `mcp_server/service/queries.rs` not a sibling. Converted `service.rs` → `service/mod.rs` so the queries submodule sits at `service/queries.rs`.
- **`stmt.query_map(...)?.collect::<...>()?` as if-arm tail.** Triggered E0597 (`stmt` doesn't live long enough). Workaround: bind to `let rows = ...` first, then return `rows` as the trailing expression. Pattern repeats across all read tools in `queries.rs`.
- **Hand-rolling JSON-RPC over HTTP without rmcp.** Considered to avoid the dep weight. Dropped because (a) the streamable HTTP transport has session-management subtleties and (b) the recent CVE on Host header validation means rolling our own would re-litigate a class of bug rmcp already addresses.
