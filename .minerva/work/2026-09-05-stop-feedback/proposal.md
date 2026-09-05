# Proposal: stop-feedback

**Date**: 2026-09-05
**Status**: Draft

## Goal

Pressing Stop must visibly change the app's state at the moment of the click, instead of
leaving the UI presenting as a live recording for the several seconds the backend spends
finishing. A second click during that window must be impossible rather than producing an
error.

## Why

`useSession.stop()` awaits `stopSession()` and mutates React state only in its `finally`.
`stop_session` spends that time inside `CaptureEngine::stop()` — joining the capture threads,
flushing each device's segmenter, then waiting for the whisper worker pool to drain the queue.
Inference serialises on one Metal GPU ([[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]]),
so with several devices that is a few seconds.

For that whole window **no React state changes at all**, and everything keyed off
`isRecording` keeps asserting a recording is in progress:

- `Header` renders "Stop" on an enabled button with a pulsing accent dot.
- `Header` renders `recording · MM:SS` beside a pulsing red dot, and the elapsed counter keeps
  **counting upward** — the app claims to still be capturing audio that stopped on the click.
- `RecorderPane` renders `MM:SS elapsed` and, with an empty transcript,
  "Listening — transcript will appear here as you speak…".

The one thing that does change is invisible from the UI's point of view: `stop_session` takes
the engine out of `AppState` synchronously, before spawning the blocking teardown, so
`list_capture_devices` reports every device `idle` at `level: 0.0` from the first poll after
the click and the level meters flatten. Flat meters under a running clock and a "Listening…"
message read as a freeze, not as progress.

The still-enabled button is the second defect. A second click re-invokes `stop_session`, whose
engine is already `take()`n, so it returns `AppError::NoSession` — and `stop()`'s `catch`
writes that into the error bar. Pressing Stop twice because the first press appeared to do
nothing is the most likely thing a user does here, and it is rewarded with a spurious error.

## Approach

**A "stopping" state in `useSession`, threaded to both consumers.** No backend change and no
new IPC contract: everything needed is already known on the frontend at the moment of the
click.

1. **`useSession` gains `isStopping`.** `stop()` sets it true on entry and clears it in the
   `finally` beside `setIsRecording(false)`, so it clears on the failure path too. The
   elapsed-timer interval is cleared **at the top** of `stop()` rather than in the `finally`,
   which freezes the clock on the click — capture really has ended by then, so a frozen number
   is the honest one.

2. **`stop()` guards itself.** An early return when a stop is already in flight, so the
   "never invoke `stop_session` twice concurrently" invariant lives in the hook rather than
   only in one button's `disabled` attribute. `stop_session` is not idempotent, and today the
   button is the only thing standing between a user and its error path.

3. **The state machine is explicit**: `recording → stopping → stopped`. `isRecording` stays
   true for the whole stopping window (it is what keeps `canResume` false), so **"actively
   capturing" is `isRecording && !isStopping`** everywhere it is asked. Both components
   compose the two booleans that way rather than each inventing its own reading.

4. **`Header`** labels the button "Stopping…" and disables it — the one case where disabling
   this button is correct, since the action it offers has already been taken. The
   `recording · MM:SS` line becomes a `finishing · MM:SS` line with the pulse removed.

5. **`RecorderPane`** reports the recording as finishing rather than listening, in both the
   elapsed line and the empty-transcript message. Its `isStopping` prop carries the **same**
   `viewingSessionId === activeSessionId` gate `isRecording` already carries in `App.tsx`, so
   a user who clicks to a past recording mid-stop does not see "finishing" on the wrong pane.

Rejected:

- **Backend `stop_progress` events** giving a determinate "finishing 3 of 5 segments".
  `TranscriptionPool` tracks only `dropped` and has no pending count, so this needs new Rust
  state, an `AppHandle` plumbed into `stop()`, and a new event contract — a strictly larger
  surface for a display that would not even be linear in time, since segment durations vary.
  It layers cleanly on top of this if the indeterminate state proves not to be enough.

- **Optimistic stop** — flip `isRecording` false immediately and let the command run
  unawaited. It makes `canResume` (`!isRecording && downloadProgress === null`) true while the
  first engine's whisper workers are still draining, putting a live "Resume recording" button
  in front of the user mid-flush. `stop_session` never holds the `session_transition` lock
  that `start_session`/`resume_session` use, so that window is only unreachable because the UI
  currently refuses to enter it. This approach would make it reachable.

## Success criteria

1. Clicking Stop changes the button to a disabled "Stopping…" **before** `stop_session`
   resolves.
2. A second click during the stopping window cannot invoke `stop_session`, and no
   `NoSession` error reaches the error bar.
3. The elapsed counter freezes on the click rather than continuing to climb until the
   backend returns.
4. The header no longer presents the session as actively recording during the window.
5. `RecorderPane` reports the recording as finishing rather than "Listening…", and only on
   the pane for the session actually being stopped.
6. `isStopping` clears on both the success and the failure path of `stop_session`.
7. An `App.test.tsx` test drives the **composed** tree — click Stop against a `stop_session`
   mock that stays pending, assert the stopping UI, resolve it, assert the stopped UI. Prop-level
   tests in isolation cannot catch a wiring mistake in `App.tsx`, which is the only consumer of
   both components, and that exact shape is the documented cause of two prior bugs here
   ([[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]],
   [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]]).
8. `npm run check` passes (typecheck, typecheck:node, lint, format, clippy, rustfmt, tests).

## Open Questions

None blocking. One standing fact surfaced during design and deferred to promote:
`stop_session` takes the engine and runs teardown without holding `session_transition`, so
the "engine absent but still tearing down" window is guarded only by the frontend. Nothing
reaches it today; it is worth recording rather than fixing here.
