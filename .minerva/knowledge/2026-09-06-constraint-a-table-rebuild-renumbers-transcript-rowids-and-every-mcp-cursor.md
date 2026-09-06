# A table rebuild renumbers transcript rowids, and every MCP cursor with them

**Date**: 2026-09-06
**Type**: constraint
**Summary**: `transcript_lines` has a `TEXT` primary key, so its rowid — the `get_transcript` cursor — is reassigned by `VACUUM` or a create-copy-drop-rename migration like 010; cursors are valid only for one app process
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

`get_transcript` pages by rowid rather than `recorded_at`. The rowid is assigned under the
insert mutex, is monotonic, and never ties, so "everything after cursor N" is exact even when
two devices land lines in the same millisecond. `recorded_at` cannot do that: `persist_and_emit`
stamps it before taking the lock, so two lines can be timestamped in one order and inserted in
the other, and a timestamp cursor would skip one.

## Finding

`transcript_lines` has `id TEXT PRIMARY KEY`, so its rowid is SQLite's implicit one, not an
`INTEGER PRIMARY KEY` alias. An implicit rowid is reassigned whenever the table is rebuilt:
by `VACUUM`, which nothing in darric runs, and by a create-copy-drop-rename migration — which
migration 010 already is, because SQLite cannot alter a `CHECK` constraint any other way, and
which any future change to `direction`'s `CHECK` would be again.

Both can only run inside `db::open` at startup, on the one launch that applies a new
migration, before the server exists; and a restart drops every MCP session anyway. So a cursor
is valid for the life of the app process and the tool description says not to persist one.

## Implications

- Do not add a `VACUUM` anywhere the app runs while serving, and do not rebuild
  `transcript_lines` outside a migration.
- If a cursor ever needs to survive a restart, add an explicit `INTEGER` sequence column
  first and page on that; rowid cannot carry it.
- Rowid order is transcription-completion order, not speech order, across devices. Each line
  carries `recorded_at` for callers that need to re-sort.

## Related

- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the tool whose cursor this bounds
- [[2026-09-06-bug-an-empty-page-with-no-cursor-restarts-the-poll-loop]] — the other way a cursor goes wrong
- [[2026-09-06-decision-recorded-at-is-the-capture-time]] — see also
