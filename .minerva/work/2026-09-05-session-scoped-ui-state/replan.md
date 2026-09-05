# Replan — 2026-09-05

## Original plan

Fix #21 by releasing the session in `useSession.stop()`:
`setActiveSessionId(null)` in the `finally`, beside `setIsRecording(false)`. The proposal
justified it as safe on the grounds that "every consumer of `activeSessionId` is already gated on
`isRecording`, which is false by then".

## What changed

**That justification was wrong, and the existing test suite caught it.** One consumer is *not*
gated on `isRecording`:

```tsx
droppedSegments={viewingSessionId === activeSessionId ? droppedSegments : 0}
```

That gate is deliberate and load-bearing. `2026-09-05-stop-feedback` established that the
"transcription fell behind — N segments dropped" warning must **outlive** the recording: the
count is only readable before the click (`capture_drop_count` reads an engine `stop_session`
releases on entry), so the warning is shown after `isRecording` has gone false, and it is scoped
to the right pane by comparing against `activeSessionId`. Nulling the id makes that comparison
false, the prop collapses to `0`, and the banner vanishes — reintroducing the exact defect #24
had just fixed. `App.test.tsx`'s "keeps the dropped-segment warning when the stop clears the
backend count" failed immediately.

The deeper error: `activeSessionId` is doing two jobs. `RecordingList` reads it as **"which
recording is live right now"**; the drop warning reads it as **"which recording does this
post-stop state belong to"**. Clearing it serves the first and destroys the second.

## New plan

**Gate the display, not the state** — which is the principle the other two fixes in this unit
already follow.

- `activeSessionId` keeps meaning "the most recently active recording" and is **not** cleared on
  stop. Its declaration now says so, and says that anything meaning "recording right now" must
  conjoin `isRecording` rather than read it alone.
- `App` passes `activeId={isRecording ? activeSessionId : null}` to `RecordingList`. The dot means
  "recording now", so it is gated on exactly that.

Strictly better than the original: it fixes #21 at the point of display, leaves the
drop-warning attribution intact, and needs no change to `useSession.stop()` at all. No success
criterion changes — criterion 1 ("after a recording stops, no row shows the live dot") and
criterion 6 ("#24's behaviour unchanged") are both now satisfiable, which they were not together
under the original plan.
