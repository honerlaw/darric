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
