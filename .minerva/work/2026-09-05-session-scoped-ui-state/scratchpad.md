# Scratchpad: session-scoped-ui-state

## Quick decisions 2026-09-05

- [decided] intake: the seed names #21/#22/#23 explicitly, so this is adoption, not a match —
  `**Closes**: #21, #22, #23` written at creation (no escalation; the user chose the items)
- [decided] scope check: one unit, one PR, unphased — the user asked for one workstream, and the
  three share `useSession`/`App` wiring. Not decomposed: three separate units would re-pay
  propose/worktree/review/promote/ship five times over for ~40 lines of fix.
- [decided] approach #22: a `startingSessionId` in `useSession`, not the obvious
  `viewingSessionId === activeSessionId` gate — that gate reads false for the whole start (the
  id is assigned from `start_session`'s return) and would strip the "Starting…" a resume
  correctly shows today. Dominant on not-regressing-resume.
- [decided] approach #23: add `session_id` to the `transcript_chunk` payload rather than
  capturing the id when the linger listener attaches. The capture approach does not work: the
  `[sessionId]` effect is declared before `[isLive]`, so `sessionRef.current` is already the new
  session at attach time. Dominant on correctness, not just on style.
- [decided] whole-proposal soundness: three bounded fixes, one additive event field, no public
  interface removed — sound (the event gains a field; no consumer breaks)

## Notes

- Base is `1033c2f` (main after #24 squash-merged), so the stop-feedback code these fixes build
  on is present.

## Review triage 2026-09-05

Local-diff mode: main-model minerva audit + a fresh-context code-quality subagent. 12 findings.

- **1** [medium] sidebar dot pulses for the whole stopping window (`isRecording` alone) →
  **FIX** — gated `isRecording && !isStopping`, matching every other consumer. Replan entry 2.
- **2** [medium] `useTranscript`'s initial fetch has no stale-response guard — #23 via the query
  path → **FIX** — same attribution as the chunk filter. Replan entry 2.
- **3** [medium] `droppedSegments` never reset, so the previous recording's warning shows on the
  next one's pane → **FIX** — reset on `activeSessionId` change (keyed on the session, so a
  resume of the same recording correctly keeps it). Replan entry 2.
- **4** [low] overlapping resumes cleared each other's `startingSessionId` → **FIX** — only the
  call that set the id clears it. `isStarting` has the same flaw and is left alone (pre-existing).
- **5** [low] `setActiveSessionId` exported with no consumer → **FIX** — removed; it also handed a
  caller a way around the invariant the new declaration comment establishes. Verified dead first.
- **6** [medium] the `viewingSessionId === startingSessionId` half of the #22 gate was untested →
  **FIX** — resume A while viewing B.
- **7** [medium] `setStartingSessionId(null)` release untested → **FIX**.
- **8** [medium] the dot was under-constrained in two directions (count-based assertion with one
  session on screen) → **FIX** — two-session, row-scoped assertion.
- **9** [low] the chunk-filter comment described only one of the two paths → **FIX** — the comment
  now separates the during-recording switch (where a captured id is genuinely wrong) from the
  post-stop linger (where it would have been fine). The reviewer was right that the original
  justification conflated them; the conclusion is unchanged.
- **10** [low] the `startingSessionId !== null &&` guard is unobservable → **IGNORE** — kept as
  documentation of intent; `RecorderPane` early-returns when `viewingSessionId` is null.
- **11** [low] `isStarting` + `startingSessionId` could be one discriminated union → **IGNORE** —
  the split is justified (the header needs the app-scoped boolean for a fresh start, which has no
  id), and findings 4 and 7 close the drift hazard that motivated it.
- **12** [low] `mockPendingStop`'s doc comment sat above the wrong declaration → **FIX**.

All nine behavioural changes are mutation-tested — eight in vitest, the Rust payload field in
cargo. Two of the review's own suggested mutations (`isRecording ? …` and `viewingSessionId`
for the dot) are now caught explicitly.
