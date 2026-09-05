# Replan log: knowledge-wiki-migration

## 2026-09-05 — success criterion 4 was written too broadly to be satisfiable

### Original plan

Success criterion 4 read:

> No `.md` file in the repo references `.minerva/decisions/` or a `../../decisions/`
> relative path.

### What changed

Nothing about the work or the approach. The criterion as worded cannot be satisfied by
any correct execution of this unit, because the unit's own record has to name the layout
it migrated *from*. At completion verification the sweep returned 10 hits, all of them in
`.minerva/work/2026-09-05-knowledge-wiki-migration/proposal.md` and `scratchpad.md` —
the Goal, the Why, the rename table, the rejected alternatives, and criterion 4 quoting
itself.

A second, separate case surfaced alongside it: `.minerva/work/2026-05-19-mcp-server/archive/scratchpad.md`
carries a bare `decisions/` in its boilerplate header ("significant items get promoted to
`decisions/`"). That file is an **archived** record of the process as it stood in May.
Rewriting it would falsify the archive, and `minerva:migrate-fix` names exactly this case
as correct to leave — "prose in an entry recounting an old number". It is left as written.

Neither case is a live pointer. Every pointer a reader would actually follow — the three
relative links in `2026-05-19-mcp-server/proposal.md` and the `AGENTS.md` Routing bullets
— was retargeted.

### New plan

Restate criterion 4 to say what it was always meant to test: no *live* pointer survives.
Historical and self-referential prose is explicitly out of its scope, and the archived
scratchpad is named so a later reader does not "fix" it.

No change to the approach, the diff, or any other criterion.
