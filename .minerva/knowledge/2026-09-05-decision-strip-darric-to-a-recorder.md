# Strip darric to a recorder: remove the AI, MCP, notes, tasks, tags and search subsystems

**Date**: 2026-09-05
**Type**: decision
**Summary**: darric is now only a recorder — the AI chat, MCP client, MCP server, notes, tasks, tags, search and board features were deleted outright, retiring the four MCP decisions with them
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

darric had grown to span meeting capture, a notes app, a kanban board, a tagged search index, a
two-provider AI chat agent, an MCP client and an MCP server — about 10,300 lines, roughly 1,000
of which were already unreachable.

The recording path was the least developed part of all of it. It captured the default microphone
only; `audio/system_tap.rs` had been the single line `// removed — mic-only capture` since the
initial commit, so the app could not hear the other side of any call it sat in.

## Decision

Everything not serving "start a recording, transcribe it, stop" was deleted, and the space spent
on real multi-device capture instead. Removed: `src-tauri/src/ai/` (Claude and Gemini providers,
the streaming agent harness, the stdio MCP client), `src-tauri/src/mcp_server/` (the rmcp
streamable-HTTP server and its 919-line query layer), six command modules, and the timeline,
notes, board, settings, search, chat and tag frontend. The Tauri command surface went from 30
commands to 9. Migration 009 drops the `notes`, `tasks`, `tags`, `session_tags`, `note_tags`,
`task_tags` and `chat_messages` tables and the `sessions.notes` column.

## Consequences

- The four MCP decisions are retired, not reversed. They were sound; the subsystem they governed
  no longer exists. Read them as history of a removed feature, not as live architecture.
- `2026-05-19-decision-inline-tests-for-mcp-queries` is the exception and stays live. Its
  predicted cost — the migration list duplicated between `db/migrations.rs` and
  `tests/common/mod.rs` — came due during this very strip, when migration 009 had to be added to
  both by hand. The MCP code that motivated the entry is gone; the duplication it warned about
  outlived it.
- Anything wanting darric's data over MCP must be rebuilt against the new schema. Nothing
  external depended on it yet.

## Related

- [[2026-05-19-decision-rmcp-as-mcp-sdk]] — retired with the MCP server this removes
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — retired with the MCP server this removes
- [[2026-05-19-decision-tool-handler-router-pattern]] — retired with the MCP server this removes
- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — survives the removal; its predicted cost came due during it
- [[2026-09-05-constraint-phases-must-use-the-canonical-list-form]] — see also
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — see also
