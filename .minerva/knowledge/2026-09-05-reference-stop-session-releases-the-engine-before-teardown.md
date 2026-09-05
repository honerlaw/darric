# `stop_session` releases the engine before it tears it down

**Date**: 2026-09-05
**Type**: reference
**Summary**: `stop_session` `take()`s the engine out of `AppState` synchronously on entry and only then spawns the blocking teardown, so for the seconds that follow every engine-derived command reports "no recording" while capture threads, segmenters and whisper workers are still running
**Context**: .minerva/work/2026-09-05-stop-feedback

## The fact

`commands/sessions.rs::stop_session` begins:

```rust
let engine = state.engine.lock()…​.take();
let Some(engine) = engine else { return Err(AppError::NoSession) };
…
tokio::task::spawn_blocking(move || engine.stop()).await…
```

The `take()` is synchronous and happens **first**; `engine.stop()` — thread joins, segmenter
flush, whisper queue drain — runs afterwards on a blocking task and takes seconds. So there is a
multi-second window in which `state.engine` is `None` while the recording is, in every physical
sense, still finishing.

`stop_session` also does **not** hold the `session_transition` lock that `start_session` and
`resume_session` take.

## What follows from it

Four consequences, all observed:

1. **`list_capture_devices` reports every device `state: "idle"`, `level: 0.0`** from the first
   poll after the click (`commands/devices.rs` maps a `None` engine to exactly that). The level
   meters therefore flatten on their own — no frontend work is needed to stop them animating,
   and none should be added.
2. **`capture_drop_count` returns `0`** for the same reason. This is destructive rather than
   merely stale: a poll landing in the window overwrites a real "transcription fell behind — N
   segments dropped" warning with zero, and since the poll is gated on the recording being live
   it never runs again. The count is only readable _before_ the click, so anything displaying it
   must stop polling at the click rather than when the command returns.
3. **The true final drop count is unreachable.** Segments can still be dropped during the flush,
   and by then nothing can read the counter. What the user sees is the count as of the click.
4. **Nothing in the backend prevents a `start_session` / `resume_session` during the window.**
   Both check `engine.is_some()` under `session_transition`, and by then the engine is already
   `None` — so both would succeed and install a second engine while the first one's whisper
   workers are still draining. It is unreachable today only because the frontend keeps
   `isRecording` true across the whole stop, which keeps `canResume` false and keeps the one
   button routed to Stop. That is a UI invariant standing in for a backend one.

## Implications

- Treat "the engine is gone" as **"a stop has begun"**, not as "no recording is running". Any
  new command that derives state from `state.engine` inherits all of the above.
- A backend guard for point 4 — holding `session_transition` across the teardown, or a
  `Stopping` state in `AppState` — would let the frontend stop carrying that invariant. Nothing
  reaches it today, so this is recorded rather than fixed.

## Related

- [[2026-09-05-pattern-state-changed-only-in-finally-reads-as-a-dead-click]] — the UI defect that made this timing matter, and the phase that now stands in for the backend guard
- [[2026-09-05-bug-the-session-start-guard-is-check-then-act]] — the same check-then-act shape on the start side, which `session_transition` was introduced to close
- [[2026-09-05-decision-capture-engine-requires-a-transcriber]] — the engine's construction contract
- [[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]] — see also
