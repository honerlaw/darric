# A slow command that updates state only in `finally` reads as a dead click

**Date**: 2026-09-05
**Type**: pattern
**Summary**: `stop()` awaited a multi-second Tauri command and mutated every piece of UI state in its `finally`, so for the whole call the app rendered exactly as it had while recording — and the button stayed live, so a second press hit a non-idempotent command
**Context**: .minerva/work/2026-09-05-stop-feedback

## Context

Pressing Stop in darric produced no visible change for several seconds. `CaptureEngine::stop()`
joins the capture threads, flushes each device's segmenter and waits for the whisper pool to
drain, and inference serialises on one GPU
([[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]]), so with several
devices that is a real wait.

The hook was written the obvious way:

```ts
try {
  await stopSession();
} catch (e) {
  setError(String(e));
} finally {
  setIsRecording(false);
  clearInterval(timerRef.current); // ← the clock kept ticking until here
  await refresh();
}
```

Nothing is wrong with any single line. The defect is that the _only_ state transition happens
after the await.

## Finding

Between the click and the resolution there was no state change at all, so **everything keyed
off the old flag went on asserting the old truth**: the button still read "Stop", the header
still pulsed `recording · MM:SS`, and the elapsed counter kept **counting upward** — the app
claiming to record audio that had already stopped.

Two consequences that a "just show a spinner" framing misses:

1. **The still-live control is a second bug.** `stop_session` takes the engine out of app state
   on entry, so a second press returned `NoSession` straight into the error bar. Pressing again
   because the first press appeared to do nothing is the single most likely thing a user does
   here, and it was rewarded with an error.
2. **The fix is a third state, not an earlier flip.** Flipping `isRecording` false at the click
   would have been simpler and wrong: `canResume` is `!isRecording && …`, so it would have put a
   live "Resume recording" button in front of the user while the first engine's workers were
   still draining. The honest model is `recording → stopping → stopped`, with the old flag
   staying true and _"actively capturing"_ re-expressed as `isRecording && !isStopping`.

## Implications

- **An `await` with no state change before it is a window in which the UI is lying.** Ask what
  the user sees for the duration of every awaited call, not just what is true once it returns.
- **Adding a phase means auditing every consumer of the flag it splits**, not just the two
  places you meant to change. Here that turned up a poll that erased a warning
  ([[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]]) and a caret that
  kept blinking.
- **A control the user just pressed should not become `disabled`.** Native `disabled` moves
  focus to the body mid-interaction and announces nothing, which relocates the original
  confusion to assistive tech. `aria-disabled` keeps focus, and the re-entrancy guard belongs in
  the hook that owns the invariant rather than in one button's markup.
- Prefer freezing a running counter at the moment of the click over letting it run: the frozen
  number is also the more truthful one.

## Related

- [[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]] — why the rest of the UI flattens on its own during that window, and what else the same release breaks
- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — the other way this app rendered a state nobody could see
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — the same "does this actually run?" question asked of a guard rather than a render
