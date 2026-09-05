# A UI rewrite reproduces the markup and silently drops the state guards

**Date**: 2026-09-05
**Type**: pattern
**Summary**: rewriting a screen from its rendered shape carries the visible markup across but loses per-selection state resets and cross-entity guards — the parts with no visual counterpart
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Collapsing darric to one screen replaced the 284-line `MeetingScreen` with a smaller
`RecorderPane`. The rewrite worked from what the old screen displayed. Everything visible
survived — the editable title, the transcript list, the elapsed timer, the resume button.

Two things did not, and a code review caught both.

## Finding

`MeetingScreen` held two guards that had no rendered form:

1. **A per-session state reset.** It cleared its own draft state in a `useEffect` keyed on
   `sessionId`. `RecorderPane` kept `editingTitle` / `titleDraft` in local state with no such
   reset, and the component is never remounted because selecting a different recording only
   swaps a prop. So: start editing recording A's title, click recording B, commit — and **B is
   renamed with the text typed for A**, silently, with no error. The rename handler closes over
   the currently-selected id, which by then is B's.

2. **A cross-entity guard.** It received `canResume={!isRecording}` — global recording state,
   not "is _this_ one recording". The rewrite passed only the local flag, so "Resume recording"
   was offered while a different recording was already running. The backend correctly rejected
   it with `SessionActive`, into an `error` field the new `App.tsx` never rendered — so the
   button did nothing and said nothing.

Both bugs are invisible in a screenshot and invisible in a single-entity test. Both need two
entities and a switch between them to appear at all.

## Implications

- When rewriting a component, diff its **state management** against the original separately from
  its markup: every `useEffect`, every reset, every prop that carries global rather than local
  state. Those are the parts a shape-driven rewrite cannot see.
- Any component keyed on a selected entity needs an explicit answer to "what happens to
  in-progress local state when the selection changes?" — reset it, or key the component by the
  entity id so React remounts it.
- An error field that is set but never rendered is not error handling. If a hook exposes
  `error`, something must display it.

## Related

- [[2026-09-05-decision-strip-darric-to-a-recorder]] — the rewrite that produced both regressions
