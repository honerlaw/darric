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

## Work notes 2026-09-05

- Root cause confirmed by reading, not guessing: `RecorderPane` early-returns its "Select a
  recording" placeholder when `session === null`, **above** the `downloadProgress` block. On a
  fresh install there are no recordings, so the indicator was structurally unreachable in the
  only situation it exists for. The feature was written, wired end-to-end, and mounted where it
  could never run.
- Two paths call `ensure_model`: `lib.rs`'s startup pre-load and `sessions.rs::load_transcriber`.
  Pressing Record during the startup download reaches the second one, which blocks on a full
  ~1.6 GB download while the button sits disabled at "Starting…". That is the reported "freeze".
- Both paths stream into the same `ggml-large-v3-turbo.tmp`, so the two downloads interleave
  writes into one file and then both rename it into place. Out of scope for this UI change;
  filed for promote's deferral bar. This diff closes the only two UI triggers (Record and Resume
  are both withheld while a download is in flight).
- Misfiled `Header.tsx` into `src/components/` on the first write; it lives in
  `src/components/layout/`. Caught by the Header test asserting against the untouched original.
- `jsdom` has no `window.matchMedia`, and `App`'s colour-scheme effect calls it on mount, so no
  test could render `App` at all before this. Stubbed once in `src/test/setup.ts`.
- Success criterion 5 says "verified by a `useSession` test". It is verified through `App`
  instead, which mounts the real `useSession` and additionally asserts the button re-enables —
  strictly more than the criterion asks, so no replan.
- The resume guard has no visual counterpart in the pane, which is exactly the shape
  `2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup` describes. Covered by a test
  plus a positive-control test, so a future change that breaks selection cannot leave the guard
  assertion passing vacuously.
