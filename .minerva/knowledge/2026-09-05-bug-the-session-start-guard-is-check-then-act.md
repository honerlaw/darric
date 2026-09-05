# The `engine.is_some()` start guard is check-then-act, and an overwritten `CaptureEngine` never stops

**Date**: 2026-09-05
**Type**: bug
**Summary**: `start_session` / `resume_session` check `engine.is_some()` at the top but install the engine only after the model load, so two commands could both pass and the second's engine silently replaced the first — whose threads and whisper workers then ran until the app quit
**Context**: .minerva/work/2026-09-05-transcriber-single-flight

## Context

Both commands opened with:

```rust
if state.engine.lock()….is_some() { return Err(AppError::SessionActive); }
```

and installed the engine as the last act of `begin_capture`. Everything between is the window.

That window used to be short. Making the transcriber load blocking (see
[[2026-09-05-decision-capture-engine-requires-a-transcriber]]) stretched it to the length of a
1.6 GB model download — minutes on a first launch.

## Finding

Two commands can both pass the guard and both build a `CaptureEngine`; the second's
`*state.engine.lock() = Some(engine)` drops the first.

**`CaptureEngine` has no `Drop` impl.** Dropping one therefore never sets its `shutdown` flag, and
`source::run_source` loops on `while !shutdown.load(…)`. The abandoned engine's capture threads
keep running, keep holding their `Arc<TranscriptionPool>` clones so the whisper workers stay alive
too, and keep writing `transcript_lines` rows for the orphaned session — with the microphone open,
no UI indication, and no reachable handle to stop any of it. Only quitting the app recovers.

Reachable through the UI: the Header's Record button is disabled while `isStarting`, but
RecorderPane's "Resume recording" button is gated only on `!isRecording` and has no `disabled`
attribute — so a double-click, or Record followed by Resume during a first-launch download, is
enough.

The fix is an `AppState.session_transition: tokio::sync::Mutex<()>` held across the whole command,
so the guard and the install are inside one critical section and the second caller finds
`engine.is_some()` and is correctly refused.

## Implications

- **A guard is only as good as the window between checking and acting**, and that window is a
  property of the code _after_ the check. Adding an `await` to a command silently widens every
  check-then-act above it — this one went from microseconds to minutes without its guard changing
  a character.
- **A struct that owns threads needs `Drop`, or overwriting it leaks them silently.** `OutputTap`
  has `Drop` and tears itself down correctly; `CaptureEngine` relies on an explicit `stop(self)`
  that a dropped value never receives. Anything holding a `JoinHandle` and a shutdown flag should
  assume it will one day be dropped rather than stopped.
- Fixing the frontend button alone would not have been enough: the race is reachable from any two
  concurrent invocations, and a UI guard cannot protect an invariant owned by the backend.

## Related

- [[2026-09-05-decision-capture-engine-requires-a-transcriber]] — the change that widened this window
- [[2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber]] — the unit that surfaced it
- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — the earlier thread-leak-on-teardown in the same engine
- [[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]] — see also
- [[2026-09-05-bug-a-functional-updater-reads-a-ref-after-the-caller-has-moved-on]] — see also
