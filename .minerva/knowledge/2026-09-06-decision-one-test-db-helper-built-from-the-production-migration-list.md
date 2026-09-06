# One `db::test_db()` helper, built from the production migration list

**Date**: 2026-09-06
**Type**: decision
**Summary**: inline Rust tests get their schema from `db::test_db()`, which runs `migrations::migrations()`, instead of each module re-listing every migration with `include_str!`
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

The May 2026 inline-tests decision recorded its cost: with tests inline rather than under
`tests/`, the migration-loading helper was duplicated, and a new migration had to be added to
every copy. By September `commands/sessions.rs` carried a ten-entry `include_str!` chain of
its own, and the MCP rebuild was about to add a third copy in `db/sessions.rs` and a fourth in
`mcp_server`. That decision named the fix — "a `#[cfg(test)] pub(crate) fn test_db()` helper
reachable from both" — and this unit did it.

## Finding

`src-tauri/src/db/mod.rs` has

```rust
#[cfg(test)]
pub fn test_db() -> Connection
```

which opens an in-memory connection, enables foreign keys, and runs the same
`migrations::migrations()` production uses. The inline test modules in `commands/sessions.rs`,
`db/sessions.rs` and `mcp_server` all call it; the `commands/sessions.rs` chain is gone. A
new migration is now added in exactly one place and every inline test sees it.

`pub` rather than `pub(crate)` because `db` is a private module and clippy's
`redundant_pub_crate` (nursery) says so.

## Implications

- Do not reintroduce an `include_str!` chain in a new test module; call `crate::db::test_db()`.
- The integration tests under `src-tauri/tests/` still carry their own chain, because they
  cannot see a `#[cfg(test)]` item in the crate. That duplication remains, now between two
  places instead of four.
- `db::open_at(&path)` is the on-disk counterpart for a test that needs a second connection to
  observe the first.

## Related

- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — builds on
- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the unit that added the third caller and made the helper worth writing
