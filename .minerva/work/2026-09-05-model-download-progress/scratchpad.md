# Scratchpad: model-download-progress

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Quick decisions 2026-09-05

- [decided] pre-flight: `2026-09-05-strip-to-recorder` is Draft-but-shipped (all 3 phase PRs merged) — adjacent theme, not a collision; no live `darric-*` peer sessions to query
- [decided] open-issue match: only open issue is #13 (verify output capture on real hardware) — no match, no adoption
- [decided] scope check: single unit, single PR — ~6 small files, no phasing (a phased quick run would be a scope-fit escape signal)
- [decided] approach: lift the indicator to `App` scope + honest Record button label (dominant). Rejected duplicating the block into `RecorderPane`'s null branch (leaves app-global state session-scoped; indicator still vanishes on selection) and a blocking modal (takes away legitimate concurrent use; worsens the failure mode)
- [decided] whole-proposal soundness: the one new interface is the `model_download_error` Tauri event, which follows three existing siblings — confident, no escalation
- [decided] backend duplicate-download race (startup `ensure_model` vs `load_transcriber`'s, both streaming to the same `.tmp`) is out of scope for the UI fix; it has a writable failure scenario so it goes to promote's deferral bar as an issue. This diff closes the only UI path that triggers it.
