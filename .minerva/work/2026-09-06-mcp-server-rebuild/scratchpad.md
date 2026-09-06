# Scratchpad: mcp-server-rebuild

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06

- [decided] scope check: single unit, one PR, no phases — decided under human gates in `minerva:explore` → `minerva:propose` earlier this session (user chose in-app server over standalone binary; always-on fixed port; status tool included)
- [decided] approach: A, shared query layer + server's own read-only SQLite connection — decided under human gates in `minerva:propose` (rejected B: self-contained SQL duplicating the transcript SELECT; rejected C: FTS5 from day one, no measured need)
- [reviewed — folded] whole-proposal soundness: Skeptic accept with 6 concerns — folded 1 (rowid renumbering trigger is a rebuild migration like 010, not just VACUUM; cursors now scoped to the app process), 2 (protocol test must insert through a concurrent writer after the first page and assert the cursor sees it), 3 (rowid order is transcription-completion order; description points at recorded_at), 4 (LiveStatus returns engine snapshot only; status fills topic/started_at from the read-only connection); proceeded past 5 (seq on the UI type is disclosed scope) and 6 (no port retry is the chosen design)
