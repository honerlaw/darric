# Proposal: knowledge-wiki-migration

**Date**: 2026-09-05
**Status**: Draft

## Goal

Migrate darric's legacy `.minerva/decisions/` corpus into the current minerva LLM-wiki
structure at `.minerva/knowledge/`, so the four existing architectural decisions become
visible to the wiki toolchain (`knowledge_lint.py`, `knowledge_fix.py`,
`synthesis_status.py`, `migration_status.py`) instead of reading as a false clean.

Scope: records only. No source code, no build config, no runtime behavior.

## Why

`minerva:migrate` reported the corpus as structurally invisible. Every wiki tool
enumerates entries through the `ENTRY_RE` glob (`^\d{3,}|\d{4}-\d{2}-\d{2}-[a-z]+-.+\.md$`)
rooted at `.minerva/knowledge/`. darric has no such directory — its records live in the
pre-wiki `.minerva/decisions/` layout with `NNN-<slug>` names.

Three consequences, all currently live:

1. **False clean.** `migration_status` returned `non_conforming_files: []` and
   `conforming_entry_count: 0` — not because the corpus is healthy, but because it globbed
   a directory that does not exist. The same blind spot makes `minerva:lint` unable to see
   the four entries at all.
2. **No cross-references.** The four decisions have zero `## Related` edges, so the corpus
   is a flat list rather than a navigable wiki. Three of them are genuinely related (the
   rmcp pin constrains the router pattern; the inline-test decision and the spawn_blocking
   decision both follow from the same `#[tool]` async-handler constraint).
3. **Broken pointers on the way.** `minerva:migrate-fix` already renamed the work unit
   `001-mcp-server/` → `2026-05-19-mcp-server/` and retargeted its four `**Context**`
   lines. It could not touch the entries themselves, and the relative markdown links from
   `proposal.md` into `../../decisions/` will break the moment the entries move.

`AGENTS.md` (with `CLAUDE.md` symlinked to it) still points agents at
`.minerva/decisions/` as authoritative and describes work units as `NNN-<slug>`. Both
statements go stale with this change, so they are part of it.

## Approach

Relocate and complete the corpus in one pass, on one branch.

### 1. Relocate with history

`git mv` each entry from `.minerva/decisions/` to `.minerva/knowledge/`, renaming to the
conforming `<YYYY-MM-DD>-<type>-<slug>.md` stem. All four land on **2026-05-19** — the
landing date `knowledge_rename.landing_date` derives from git (oldest commit touching the
path, following renames). Several entries sharing a date is normal: identity is the whole
stem, not the date.

| From | To |
| --- | --- |
| `001-rmcp-as-mcp-sdk.md` | `2026-05-19-decision-rmcp-as-mcp-sdk.md` |
| `002-tool-handler-router-pattern.md` | `2026-05-19-decision-tool-handler-router-pattern.md` |
| `003-inline-tests-for-mcp-queries.md` | `2026-05-19-decision-inline-tests-for-mcp-queries.md` |
| `004-spawn-blocking-for-rusqlite-tools.md` | `2026-05-19-decision-spawn-blocking-for-rusqlite-tools.md` |

The `decision-` type segment is inserted by hand. `knowledge_rename.py` cannot supply it:
it preserves everything after the id verbatim (`rest = p.name[len(id)+1:-3]`), so on names
that carry no type segment it would promote the first slug word into the type slot —
minting `2026-05-19-rmcp-…`, `2026-05-19-tool-…`, `2026-05-19-inline-…`,
`2026-05-19-spawn-…`. This is why the rename half of the migration is hand-done.

### 2. Complete each entry

Add the two fields the wiki reads and the block it navigates by, leaving every authored
body section (`## Context` / `## Decision` / `## Consequences`) byte-identical:

- `**Type**: decision` — read by `TYPE_RE`; without it lint falls back to the filename segment.
- `**Summary**: <one line>` — read by `SUMMARY_RE`; its presence is what lets `index.md`
  be rebuilt mechanically instead of needing an LLM to re-condense the finding.
- `## Related` — hand-authored `[[stem]] — <label>` edges. Not automated, and not
  automatable: `minerva:lint-fix` only repairs reciprocals of links that already exist,
  never the initial edges.

`**Date**: 2026-05-18` stays as authored in every body. The filename records when the
entry *landed*; the body records when it was *written*. The one-day gap is correct and
must not be "fixed".

### 3. Author `index.md`

A `# Knowledge index` with a `## Decisions` section and one `- [[stem]] — summary`
catalog line per entry, matching `CATALOG_LINE_RE` and the `SECTION_TO_TYPE` mapping.

### 4. Repair the pointers this move breaks

- `.minerva/work/2026-05-19-mcp-server/proposal.md` — three relative links
  (`../../decisions/001-…`, `003-…`, `004-…`) retargeted to `../../knowledge/<new-stem>.md`.
- `AGENTS.md` — the `.minerva/decisions/` bullet becomes `.minerva/knowledge/`, describing
  the wiki (index + entries + wikilinks) rather than a flat decision folder; the
  `NNN-<slug>` work-unit path becomes `<YYYY-MM-DD>-<slug>`.

### 5. Carry the already-applied `minerva:migrate-fix` output

The work-unit rename and its four `**Context**` rewrites were applied to the working tree
before this unit existed; they are committed here rather than left dangling on `main`.

### Rejected alternatives

- **Delegate the scaffold to `minerva:init`.** Init is not in this orchestrator's
  delegation table, it would have to run against `main`, and its output overlaps entirely
  with what this unit must author regardless. The one thing it uniquely installs — the
  `.minerva/worktrees/` `.gitignore` entry — is a single line, and `minerva:propose`'s
  own pre-flight names adding it manually on the default branch as the sanctioned
  alternative. Done as a separate bootstrap commit on `main` (`b8ae1aa`).
- **Leave `.minerva/decisions/` in place and start `.minerva/knowledge/` fresh.** Cheapest,
  and wrong: it makes the false clean permanent and splits the corpus across two layouts
  that no tool reads together.

## Success criteria

1. `.minerva/decisions/` no longer exists; all four entries live at
   `.minerva/knowledge/2026-05-19-decision-<slug>.md` with git history followed.
2. `migration_status` reports `conforming_entry_count: 4`, `non_conforming_files: []`,
   `entries_without_related: []`, `index_present: true`.
3. `knowledge_lint.py` exits with **zero errors** (pending-reconciliation warnings are
   normal — they are `minerva:cleanup`'s job on the default branch).
4. No `.md` file in the repo references `.minerva/decisions/` or a `../../decisions/`
   relative path.
5. Every authored body section and `**Date**` field is byte-identical to what it was
   before the move; only `**Type**`, `**Summary**` and `## Related` are added.
6. `AGENTS.md` points at `.minerva/knowledge/` and the dated work-unit path.

## Open Questions

None. `overview.md` is deliberately **not** written here: it is a shared aggregate
rewritten wholesale, which is why promote is add-only and `minerva:cleanup` writes it on
the default branch after merge.
