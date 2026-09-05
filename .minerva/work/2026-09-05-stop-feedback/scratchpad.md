# Scratchpad: stop-feedback

## Balanced decisions 2026-09-05

- [reviewed — folded] scope check: one unit, one PR, unphased (Skeptic accepted the scope call
  but flagged a missing App-level integration test — the two prior repo bugs have exactly the
  "wiring verified in isolation, never exercised composed" shape; folded as success criterion 7,
  along with pinning the recording→stopping→stopped state machine and an early-return guard
  inside `stop()`)
- [reviewed — folded] approach: frontend-only `isStopping` state (Skeptic accepted; folded its
  one load-bearing point — `isStopping` into `RecorderPane` must carry the same
  `viewingSessionId === activeSessionId` gate `isRecording` does, or a user viewing a past
  recording mid-stop sees "finishing" on the wrong pane). Rejected: backend `stop_progress`
  events (needs a pending count `TranscriptionPool` does not have, for a non-linear display);
  optimistic stop (makes `canResume` true mid-flush).
- [decided] whole-proposal soundness: frontend-only, four files plus tests, no new IPC contract,
  no public interface, no knowledge conflict — sound (solo gate)

## Notes

- Both Skeptics independently confirmed `stop_session` does `engine.take()` synchronously before
  `spawn_blocking(engine.stop())`, and never holds `session_transition`. Consequences: (a) device
  meters flatten to idle/0.0 on the first poll after the click, so leaving `useDevices` alone is
  right; (b) the "engine absent, teardown still running" window is guarded only by the frontend —
  a standing fact for promote, not a fix for this unit.
- [reviewed — clean] completion verification: Verifier accepted all 8 criteria, independently
  reproducing the mutation tests on both `App.tsx` wirings plus `cargo clippy`/`cargo fmt` to
  confirm Rust is untouched. Its one low-severity note — `stoppingRef.current = true` set outside
  the `try`, so a synchronous throw before it could wedge the flag — was not a criterion gap but
  was cheap, so it was folded: everything after the guard now runs inside the `try`, leaving the
  `finally` as the single reset path.
