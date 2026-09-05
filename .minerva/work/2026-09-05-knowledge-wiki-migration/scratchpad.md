# Scratchpad: knowledge-wiki-migration

## Quick decisions 2026-09-05
- [decided] pre-flight: no in-flight collision — 2026-05-19-mcp-server is `in_flight: False` (promoted/Shipped via work_status predicate), only `main` local+remote, zero open PRs, 0 peer sessions after live+interactive+`darric-` prefix filter (37 -> 0). No messages sent.
- [decided] open-issue match: `gh` authed, issues enabled on honerlaw/darric, zero open issues — no match, no ask.
- [decided] scope check: one work unit, one PR, no phases. Records-only change (4 entries + index + 2 pointer files); no source, build config or runtime behavior touched.
- [decided] approach: relocate-and-complete in one pass. Rejected (a) delegating scaffold to `minerva:init` — not in this orchestrator's delegation table, must run on `main`, and its output overlaps entirely with what the unit authors anyway; its one unique artifact (the `.minerva/worktrees/` gitignore line) is the manual alternative `minerva:propose`'s pre-flight explicitly sanctions. Rejected (b) leaving `.minerva/decisions/` and starting fresh — makes the false clean permanent and splits the corpus across two layouts.
- [decided] gitignore bootstrap: `.minerva/worktrees/` was absent from `.gitignore` on `main`, which is a hard pre-flight abort for `git worktree add`. Added + committed on `main` as `b8ae1aa`, scoped to that one line; everything else goes through this PR.
- [decided] type segment inserted by hand, not by `knowledge_rename.py`. The script preserves everything after the id verbatim, so on `NNN-<slug>` names with no type segment it promotes the first slug word into the type slot (`2026-05-19-rmcp-…`). Verified by reading `plan()` at knowledge_rename.py:262.
- [decided] entry bodies left byte-identical; only `**Type**`, `**Summary**` and `## Related` added. `**Date**: 2026-05-18` stays as authored — filename records landing, body records authoring.
- [decided] `overview.md` NOT written in this unit — shared aggregate, written on the default branch by `minerva:cleanup` after merge (phases.md Phase 4, "No synthesis phase here").

## Notes
- Landing dates derived via `knowledge_rename.landing_date`: all four entries -> 2026-05-19. Distinct slugs, so no collision.
- `CLAUDE.md` is a symlink to `AGENTS.md`; editing AGENTS.md covers both.
