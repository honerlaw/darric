# Replan — 2026-09-05

## Original plan

Fix #21 by releasing the session in `useSession.stop()`:
`setActiveSessionId(null)` in the `finally`, beside `setIsRecording(false)`. The proposal
justified it as safe on the grounds that "every consumer of `activeSessionId` is already gated on
`isRecording`, which is false by then".

## What changed

**That justification was wrong, and the existing test suite caught it.** One consumer is _not_
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

---

# Replan — 2026-09-05 (second entry, review-driven)

## Original plan

Three fixes for #21, #22 and #23, each scoped to the one code path named in its issue.

## What changed

Review found that two of the three were **half-fixes** — the same defect reachable through a
second path the issue had not named:

- **#23 fixed the event path and left the query path.** `useTranscript`'s initial
  `getSessionTranscript(sessionId).then(setLines)` has no stale-response guard, so a slow fetch
  for the previously-selected session overwrites the pane that replaced it. Verified: select
  "Retro" while its fetch is pending, click "Standup", release the fetch — Standup's pane shows
  Retro's line. Identical user-visible outcome to #23, reached by query rather than by event.
- **#21's `activeSessionId` attribution had a hole.** `droppedSegments` is attributed by
  `activeSessionId` and never reset, so starting a _second_ recording immediately shows the
  first one's "transcription fell behind" warning on the new pane until the next poll.

Also: the #21 fix as written used `isRecording`, which the previous unit deliberately keeps true
across the whole stop — so the sidebar dot kept pulsing "recording now" for the entire stopping
window, the one place every other consumer already checks `isStopping`.

## New plan

Scope grows by two paths and one predicate, all inside the unit's stated goal — _state that
belongs to one session must be attributed to it at the point of display_:

- `useTranscript`'s initial fetch compares the resolved `sessionId` against `sessionRef.current`
  before `setLines`, the same attribution the chunk filter applies.
- `App` resets `droppedSegments` when `activeSessionId` changes. Keyed on the session rather than
  on the start, so **resuming the same recording keeps its warning** — those segments really were
  dropped from that transcript.
- The sidebar dot is gated `isRecording && !isStopping`, matching `RecorderPane.statusLine`,
  `emptyTranscriptMessage` and the header's own dot.

## Success criteria — added

9. A slow transcript fetch for a deselected session does not land in the pane that replaced it.
10. A dropped-segment warning does not carry into a _different_ recording, and does survive a
    resume of the same one.
11. The sidebar dot is dark for the stopping window, not just after it.
