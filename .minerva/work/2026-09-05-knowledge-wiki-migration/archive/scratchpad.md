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
- [decided] replan accepted: criterion 4 was unsatisfiable as worded (the unit's own record must name its source layout). Amended to test for *live* pointers only; archived boilerplate in 2026-05-19-mcp-server/archive/scratchpad.md left as written per migrate-fix's "prose recounting an old number" rule. No change to approach or diff.
- [decided] AGENTS.md Routing section written from the template-of-record verbatim, including the `.minerva/reference/` bullet even though that directory does not exist in darric. Omitting it would make the section read as permanently stale to `minerva:init`'s staleness check, which is disjunctive over the template's `.minerva/...` bullets. Don't delete the bullet.

## Review triage 2026-09-05
- F1 [FIX] empty `.minerva/decisions/` left on disk after the git mv — rmdir'd. (Not tracked; git removes it on checkout after merge anyway.)
- F2 [SUGGEST->logged] `.minerva/reference/` bullet points at a non-existent dir — deliberate, rationale recorded above.
- F3 [IGNORE] no `overview.md` — by design; `minerva:cleanup` writes it on the default branch post-merge. Already stated in the proposal's Open Questions.
- F4 [TODO] `.github/workflows/check.yml` runs TypeScript / Frontend Build / Rust / Frontend Tests / Rust Tests — no knowledge-lint gate. Now that a real corpus exists, nothing mechanically defends it. File as an issue at promote.
- F5 [IGNORE] verified all four `## Related` labels against both endpoints' bodies — each claim is supported by the linked entry's own text. Edges are reciprocal (rmcp<->router, rmcp<->spawn, router<->test, test<->spawn); every entry has exactly 2.
- F6 [IGNORE] index.md catalog is alphabetical by stem under a single `## Decisions` section, matching `SECTION_TO_TYPE` and `CATALOG_LINE_RE`. Lint confirms.
