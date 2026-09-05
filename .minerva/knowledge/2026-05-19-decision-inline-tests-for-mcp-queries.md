# Test MCP query functions inline (`#[cfg(test)] mod tests`), not via `tests/`

**Date**: 2026-05-18
**Type**: decision
**Summary**: MCP query tests live inline under `#[cfg(test)] mod tests`; `tests/` cannot reach the crate-internal `mcp_server` module without making it `pub`
**Context**: .minerva/work/2026-05-19-mcp-server

## Context

The existing DB-layer tests in this repo live as integration tests under `src-tauri/tests/` (`db_notes.rs`, `db_search.rs`, etc.) and share `tests/common/mod.rs` for in-memory database setup with all migrations applied. That pattern works for DB SQL because the schema is reachable from anywhere — the tests open a fresh `rusqlite::Connection` and exercise raw SQL.

The MCP server's query layer is different: `src-tauri/src/mcp_server/service/queries.rs` takes `&DbConn` (the crate-internal `state::DbConn(Mutex<Connection>)` wrapper), and the entire `mcp_server` module is declared as `mod mcp_server;` (not `pub mod`) in `lib.rs`. Integration tests under `tests/` see only the crate's public surface, so they cannot reach `queries::list_notes` etc. without either:

1. Making `mcp_server` (and its `service` and `queries` submodules) `pub`, expanding the public surface for tests only.
2. Putting unit tests inline in `queries.rs` via `#[cfg(test)] mod tests`.

Option 1 leaks implementation detail into the crate's public API just to support tests. Option 2 keeps tests next to code and matches the existing `#[cfg(test)]` pattern in `src/transcription/speaker_tracker.rs`. CLAUDE.md permits `#[allow(clippy::unwrap_used)]` only inside `#[cfg(test)] mod tests` blocks, so inline tests are the intended carve-out.

## Decision

Tests for `mcp_server::service::queries` live inline in `queries.rs` under `#[cfg(test)] mod tests`, with `#![allow(clippy::unwrap_used)]` at the top of the module. The test module re-implements the migration-loading helper (chain of `include_str!` calls) rather than depending on `tests/common/mod.rs` — which is unreachable from inline tests.

## Consequences

- When a new migration lands, **two** places need updating: `src/db/migrations.rs` (production) and the `include_str!` chain in `src-tauri/tests/common/mod.rs` AND in `mcp_server/service/queries.rs`'s test module. There's no shared helper.
- Other modules following this pattern (testing crate-internal functions that take internal types) should also go inline rather than expand the public API.
- If the duplication of migration-loading becomes a maintenance burden, the right refactor is a `#[cfg(test)] pub(crate) fn test_db()` helper somewhere reachable from both inline and integration tests — not making `mcp_server` `pub`.

## Related
- [[2026-05-19-decision-tool-handler-router-pattern]] — also decided by this repo's no-`#[allow]`-outside-tests policy
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — covers the same sync query layer that the tool handlers offload
