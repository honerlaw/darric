# Proposal: session-scoped-ui-state

**Date**: 2026-09-05
**Status**: Draft
**Closes**: #21, #22, #23

## Goal

Close the three deferred defects left by `2026-09-05-stop-feedback`. All three are the same
mistake in three places: **a piece of UI state that belongs to one session is displayed
without checking which session it belongs to** — or, in #21's case, never released at all.

## Why

`2026-09-05-stop-feedback` made the stopping window honest and, in reviewing it, surfaced three
pre-existing defects it deliberately did not widen scope to fix. Each was filed with a concrete
failure scenario.

**#21 — the sidebar's recording dot never clears.** `RecordingList` pulses a red dot on the row
whose `id === activeId`, and `useSession` never resets `activeSessionId`. `stop()` clears
`isRecording` and `isStopping` but leaves the id set, so a fully-stopped recording keeps a live
"recording now" dot indefinitely. This is the last element in the tree that still reads live
after a stop — the same complaint that produced the previous unit, just after the window rather
than during it.

**#22 — `isStarting` reaches `RecorderPane` ungated.** `App.tsx` passes four active-session
props to the pane. Three now carry a `viewingSessionId === activeSessionId` gate; `isStarting`
does not, so a merely-*selected* recording renders "Starting…" while a different one starts.

**#23 — flush lines can land on the wrong transcript.** `useTranscript` keeps a
`transcript_chunk` listener alive for 20 s after a stop so whisper's asynchronous flush lines
still arrive. `appendChunk` reads `sessionRef.current` at call time, and the payload carries no
session identity, so a session switch inside that window appends the stopping session's lines to
whichever session is now selected.

## Approach

**Give each piece of state an explicit owner, and check it at the point of display.** Three
independent fixes, one shared principle.

1. **`stop()` releases the session (#21).** `setActiveSessionId(null)` joins the `finally` beside
   `setIsRecording(false)`. Safe because every consumer of `activeSessionId` is already gated on
   `isRecording`, which is false by then: the pane's `isRecording`/`isStopping` props, the
   pin-to-active effect, and `useTranscript`'s `isLive` all read false either way. `RecordingList`
   is the only consumer that reads it *un*gated, which is precisely the bug.

2. **A `startingSessionId` names what is starting (#22).** The obvious fix — reusing
   `viewingSessionId === activeSessionId` — is wrong, and the issue says so: during a start
   `activeSessionId` has not been assigned yet (it is set from `start_session`'s return), so that
   gate reads false for the whole operation and would *remove* the "Starting…" that a **resume**
   correctly shows today. Instead `useSession` exposes `startingSessionId`: the id for a resume,
   `null` for a fresh recording. `App` passes `isStarting={startingSessionId !== null &&
   viewingSessionId === startingSessionId}`. A fresh start shows "Starting…" in the header only,
   which is correct — there is no session for the pane to describe yet.

3. **The `transcript_chunk` event carries its `session_id` (#23).** `persist_and_emit` already
   has the id in scope; it is one field. `useTranscript` then drops any chunk whose `session_id`
   is not the session being displayed.

   The frontend-only alternative — capturing the session id when the linger listener attaches —
   **does not work**, and this is worth recording because it looks like it does. The `[sessionId]`
   effect is declared before the `[isLive]` effect, so on the render where the user clicks away
   both run in that order: `sessionRef.current` is *already* the newly-selected session by the
   time the linger listener attaches, and the captured value is the new session, not the stopping
   one. A fix that depends on effect declaration order is exactly the shape of two defects already
   recorded here ([[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]],
   [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]]). Making the
   event self-identifying removes the ordering question rather than reasoning about it.

## Success criteria

1. After a recording stops, no row in `RecordingList` shows the live dot.
2. Pressing Record while a past recording is selected does not make that recording's pane read
   "Starting…".
3. Resuming a recording still shows "Starting…" on that recording's pane — the regression the
   naive gate would have introduced.
4. `transcript_chunk` carries `session_id`, and `useTranscript` appends only chunks matching the
   displayed session.
5. Selecting a different recording during the post-stop linger leaves the stopping session's
   flush lines out of the newly-selected pane, and they are still present when that session is
   reselected (they were persisted).
6. The stopping-state behaviour shipped in #24 is unchanged — its tests still pass untouched.
7. Every new behaviour is mutation-tested: reverting each fix individually fails the suite.
8. `npm run check` passes (typecheck, typecheck:node, lint, format, clippy, rustfmt, tests).

## Open Questions

None. All three diagnoses were written against the code when the issues were filed.
