# One piece of state answering two questions cannot be cleared for either reader

**Date**: 2026-09-05
**Type**: pattern
**Summary**: `activeSessionId` answered both "which recording is live right now" and "which recording does this post-stop state belong to"; clearing it to fix the first silently destroyed the second, and the fix was to gate the display rather than the state
**Context**: .minerva/work/2026-09-05-session-scoped-ui-state

## Context

`RecordingList` pulses a red "recording now" dot on the row whose `id === activeSessionId`, and
`useSession` never cleared that id — so a fully stopped recording kept a live dot forever. The
obvious fix is one line: `setActiveSessionId(null)` in `stop()`'s `finally`.

The proposal justified it as safe on the grounds that "every consumer of `activeSessionId` is
already gated on `isRecording`, which is false by then". That sentence was written without
enumerating the consumers, and it was false.

## Finding

One consumer is not gated on `isRecording`, and deliberately so:

```tsx
droppedSegments={viewingSessionId === activeSessionId ? droppedSegments : 0}
```

The "transcription fell behind — N segments dropped" warning **must** outlive its recording: the
count is only readable before the stop is pressed
([[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]]), so it is displayed
after `isRecording` has gone false and is scoped to the right pane by comparing against
`activeSessionId`. Nulling the id makes that comparison false and the warning vanishes —
reintroducing the exact defect the previous unit had just fixed.

The id was answering **two different questions**:

| Reader           | Question                                     | Wants the id    |
| ---------------- | -------------------------------------------- | --------------- |
| `RecordingList`  | is this recording live _right now_?          | cleared on stop |
| the drop warning | which recording does this state _belong to_? | kept after stop |

Clearing it serves the first reader by breaking the second. There is no value of the variable
that satisfies both, because the disagreement is not about the value — it is about what the
variable means.

**The fix is to gate the display, not the state.** `activeSessionId` keeps one meaning — _the
most recently active recording_ — and the reader that means "right now" says so at the point of
display: `activeId={isRecording && !isStopping ? activeSessionId : null}`.

## Implications

- **Before clearing or repurposing shared state, enumerate its readers and write down the
  question each one is asking.** If two questions differ, no assignment satisfies both; add the
  qualifier at the reading site instead. The enumeration is the work — the previous unit's own
  entry already said to audit every consumer
  ([[2026-09-05-pattern-state-changed-only-in-finally-reads-as-a-dead-click]]), and this unit
  asserted the audit's conclusion in its proposal without performing it.
- **A test suite is what makes this cheap.** The wrong fix was caught in under a minute by an
  existing test from the previous unit, before any of it was reviewed. Its failure named the
  contradiction precisely.
- **State that outlives the thing it describes needs its lifetime written down.** `activeSessionId`
  now carries a comment saying it deliberately survives the recording and that anything meaning
  "right now" must conjoin `isRecording`. Two later bugs in the same unit — a warning carried into
  the _next_ recording, and a dot lit through the stopping window — were both this same
  under-specified lifetime.

## Related

- [[2026-09-05-pattern-state-changed-only-in-finally-reads-as-a-dead-click]] — the previous unit, whose "audit every consumer of the flag you split" implication this is a second instance of
- [[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]] — why the drop warning has to outlive its recording at all
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — the same class of cross-entity guard, lost rather than overloaded
