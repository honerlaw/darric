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

## Review triage 2026-09-05

Mode: local-diff (fresh-context subagent) + minerva audit. 14 findings.

- [FIX] #1 med src/components/layout/Header.tsx:60 — the download gate also disables Stop, so an active recording cannot be stopped while the startup download is still streaming
- [FIX] #2 med src-tauri/src/lib.rs:29 + src/hooks/useSession.ts:62 — `model_download_start` is emitted during Tauri `setup()`, before the webview has any listener, so the banner does not appear until the first 5% tick (~80 MB in). The window this change exists to fix is still unguarded for the first minutes.
- [FIX] #3 med src-tauri/src/model.rs:65,95 — two concurrent `ensure_model` calls share one `.tmp` path; the interleaved result is renamed into place and cached forever with only an `exists()` check. Was deferred in the proposal on the strength of the UI guard, which #2 shows is not armed in time.
- [FIX] #4 med src-tauri/src/model.rs:37-47 — the error branch leaves the partial `.tmp` on disk, and the losing rename surfaces as a user-visible failure for a download that succeeded
- [FIX] #5 med README.md:36-38 — the first-launch section describes the download but not the behavior this diff added (minerva audit: documentation for behavior this diff touched is never deferred)
- [FIX] #6 low src/hooks/useSession.ts:77 — `error` is never cleared, so a failed download's message outlives a successful retry
- [FIX] #7 low src/components/RecorderPane.tsx:23,163 — `canResume` doc comment no longer describes the contract; `!isRecording && canResume` is now redundant
- [FIX] #8 low src/components/ModelDownloadBanner.tsx:23 — "recording starts once this finishes" promises an auto-start that does not happen
- [FIX] #9 low src/components/ModelDownloadBanner.tsx:31,37 — `progress` reaches `aria-valuenow` and a CSS width unclamped
- [IGNORE] #10 low src/components/RecorderPane.tsx:163 — Resume is unmounted rather than disabled during a download. No failure scenario. Mounting it disabled would change the visibility contract for the pre-existing "another recording is active" case, which is out of this unit's scope; the banner already explains the state.
- [IGNORE] #11 low proposal.md success criterion 5 — "verified by a `useSession` test"; verified through `App.test.tsx`, which mounts the real hook and asserts more. No failure scenario.
- [SUGGEST] #12 low src/test/setup.ts:8-18 — the `matchMedia` stub is installed in `beforeAll` and never restored, and pins every test to `matches: false` with a no-op change listener, so `App`'s dark branch and its listener add/remove pair are unexercised
- [SUGGEST] #13 low src-tauri/src/model.rs — `MODEL_URL` is a hard-coded `const`, so the download failure paths cannot be exercised from a test without injecting the URL
- [SUGGEST] #14 low Tauri `emit` from `setup()` reaches only webviews that already hold a listener, so any startup-time event needs a queryable counterpart — the general form of #2

#2 and #3 together are a load-bearing divergence: the proposal assumed the event stream was
sufficient and the UI guard closed the duplicate-download trigger. Both assumptions are wrong.
Replanning rather than patching around them.

## Review fixes 2026-09-05

- Review fix: `src-tauri/src/model.rs` — `DOWNLOAD_PCT` atomic + `download_progress()`, a
  `DOWNLOAD_LOCK` serialising `ensure_model` with an `exists()` re-check, `.tmp` removal on the
  failure path, a single `tmp_path()` derivation, percentage clamped at the source, and a 1%
  emit step in place of 5%.
- Review fix: `src-tauri/src/commands/model.rs` (new) + `lib.rs` — `model_download_state`
  command so the frontend can read a download that started before it mounted.
- Review fix: `src/hooks/useSession.ts` — seed `downloadProgress` from the query on mount
  (`current ?? pct`, so a live event already received wins); clear `error` when a start or
  resume begins.
- Review fix: `src/components/layout/Header.tsx` — `cannotStart` gates only starting, never
  stopping.
- Review fix: `src/components/ModelDownloadBanner.tsx` — clamp the percentage; drop the
  "recording starts once this finishes" auto-start promise.
- Review fix: `src/components/RecorderPane.tsx` — `canResume` doc comment corrected; the
  redundant `!isRecording &&` dropped.
- Review fix: `README.md` — first-launch section now describes what the app shows during and
  after a download.
- Wrote `#[allow(clippy::unnecessary_wraps)]` into the new command module on the first draft, on
  a function that returns `Option` and could not have triggered it. Caught immediately against
  `AGENTS.md`. Worth noting the reflex: the attribute went in pre-emptively, before any lint had
  fired, which is the case the policy is least likely to catch by review.
- Mutation-checked the two new load-bearing tests: removing the seed assignment fails both new
  `App` tests, and restoring `disabled={isStarting || isDownloading}` fails the new Stop test.

## Review triage applied 2026-09-05

- [FIXED] #1 #2 #3 #4 #5 #6 #7 #8 #9
- [IGNORED] #10 #11
- [SUGGESTED] #12 #13 #14 — see `## Review finding 2026-09-05`

## Review finding 2026-09-05

- `src/test/setup.ts` installs a `window.matchMedia` stub in `beforeAll` and never restores it,
  so it is process-wide for every test file, and it reports `matches: false` with a no-op change
  listener. `App`'s dark-mode branch and its `addEventListener`/`removeEventListener` pair are
  therefore never exercised — a change that dropped the removal (a real listener leak) or
  inverted the `classList.toggle` argument would fail no test.
- `MODEL_URL` in `src-tauri/src/model.rs` is a hard-coded `const`, so the download failure paths
  (non-2xx status, mid-stream error, rename failure, and now the `.tmp` cleanup and the
  serialising lock) cannot be exercised from a Rust test without injecting the URL. `model.rs` is
  the one module of seven with no `#[cfg(test)] mod tests`.
- Tauri's `emit` reaches only webviews that already hold a listener, so any event emitted from
  `setup()` is lost to a frontend that has not mounted. An event-only contract is therefore
  incomplete for anything that starts at app startup; it needs a queryable counterpart. This is
  the general form of the bug this unit hit.
