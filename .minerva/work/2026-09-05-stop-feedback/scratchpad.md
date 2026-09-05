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

## Review triage 2026-09-05

Local-diff mode (no PR yet): minerva audit by the main model + a fresh-context code-quality
subagent. Findings 1-9 are the code review's, numbering continued from the audit's single item.

- **Audit 1** [low] `stop_session` takes the engine and runs teardown without holding
  `session_transition` → **SUGGEST** (standing fact, no writable failure scenario — reference entry)
- **1** [medium] the `stoppingRef` reset was undefended; deleting it left the suite green →
  **FIX** (added a start→stop→start→stop case asserting the call count per round)
- **2** [medium] the live dot kept `pulse-dot` untested → **FIX** (assert zero `.pulse-dot`
  nodes while stopping, with a positive control)
- **3** [medium] caret suppression unreachable in tests — every existing case renders an empty
  transcript → **FIX** (a case with a real line; needed a `scrollIntoView` stub in `setup.ts`,
  which is why no test had ever rendered one)
- **4** [medium] the `viewingSessionId === activeSessionId` gate was untested → **FIX**
  (App test selects a past recording mid-stop)
- **5** [medium] newly disabling Stop drops keyboard focus and announces nothing → **FIX**
  (native `disabled` kept only for the cannot-start cases; the stop uses `aria-disabled`, which
  keeps focus, plus an `sr-only` `role="status"` carrying the phase but not the ticking clock).
  This is a regression this diff introduced, not a pre-existing one.
- **6** [medium] the drop-count poll erased the "transcription fell behind" warning during the
  window and never ran again → **FIX** (poll gated on `!isStopping`; the count now outlives the
  recording, so the banner prop also picks up the viewing-is-active gate)
- **7** [low] the sidebar's pulsing dot keys off `activeSessionId`, which is never reset, so it
  pulses forever after a recording stops → **SUGGEST** (pre-existing, writable failure scenario;
  outside this unit's stated window — the seed is about the seconds *during* the stop)
- **8** [low] `isStarting` reaches `RecorderPane` without the viewing-is-active gate, so a
  different selected recording reads "Starting…" → **SUGGEST** (pre-existing; the
  `droppedSegments` half was fixed under 6 because 6 made it load-bearing)
- **9** [low] `useTranscript`'s post-stop linger appends the stopping session's flush chunks to
  whichever session is selected → **SUGGEST** (pre-existing, display-only, writable scenario)
- **10** standing facts confirming the design (ref-vs-state justified and cannot skew, ref
  cannot wedge, start/resume unreachable during the window, meters flatten on their own) →
  **IGNORE**

Every fix was mutation-tested: all seven behavioural changes now fail the suite when reverted.
Finding 6's fix initially had no test — the same gap class the review had just flagged — and one
was added before this was recorded.
