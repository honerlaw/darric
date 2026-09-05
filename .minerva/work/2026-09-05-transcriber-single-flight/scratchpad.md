# Scratchpad — 2026-09-05-transcriber-single-flight

## Quick decisions 2026-09-05

- [decided] pre-flight: peer session darric-92 replied MINERVA-BUSY on `2026-09-05-model-download-progress` (download progress UI, touching `model.rs` + frontend) — resolved **adjacent**, not a collision: different subsystem, and this unit deliberately does not touch `model.rs`.
- [decided] open-issue match: only open issue is #13 (verify output capture on real hardware) — unrelated, no adoption.
- [decided] scope: one work unit, one PR — ~4 Rust files, no phases; comfortably inside the quick-run bar.
- [decided] approach: single-flight behind `tokio::sync::Mutex` in `AppState`, shared by the startup pre-load and the session path. Dominant over (a) polling for the startup task (needs a timeout, no recovery if startup failed) and (b) fixing `ensure_model`'s temp-file naming only (still downloads 1.6 GB twice and builds two contexts).
- [decided] `tokio::sync::Mutex` over `OnceCell` — a `OnceCell` would cache a failed load and poison transcription for the process lifetime.
- [decided] deliberately leave `model.rs` untouched to avoid a merge conflict with the peer's in-flight diff; the cross-process temp-file residual is recorded as a follow-up rather than fixed here.
- [decided] whole-proposal soundness: no public interface change — `start_session`/`resume_session` already return `Result` and `App.tsx` already renders `error`.

## Notes

## Review triage 2026-09-05

Fresh-context code review returned 5 definite defects + 4 suggestions. Dispositions:

- **FIX** resume_session left a live engine behind a returned Err — rewrote as DB-first with `restore_ended_session`.
- **FIX** check-then-act on `engine.is_some()`, widened to a model download by this unit — added `AppState.session_transition`. Promoted as a bug entry.
- **FIX** rollback omitted `transcript_lines`, so the `sessions` delete failed under `foreign_keys=ON` — added, wrapped in a transaction, mutation-verified (fails with `FOREIGN KEY constraint failed` without it).
- **FIX** `CaptureEngine`'s `Option<Arc<Transcriber>>` was dead and the proposal's justification for it was factually false. Made required; corrected the proposal. Promoted as a decision entry.
- **FIX** test gaps — added a queued-behind-a-failing-load case, moved the concurrency test to a multi_thread runtime, added four inline tests for the two rollback helpers.
- **FIX** the pre-lock log line claimed to distinguish waiting from cached and did not — now probes `try_lock` and reports `Origin`.
- **FIX** a stale `whisper_model_path` would have bricked recording permanently now that both paths consult it — falls back to the downloaded model.
- **TODO** cross-process `.tmp` racing survives (both locks are process-local); the real fix is validating the cached model rather than trusting `exists()`.
- **TODO** `CaptureEngine::start`'s error path never calls `exclusions.unregister` (pre-existing).
- **TODO** `await_holding_lock = "allow"` in Cargo.toml contradicts the CLAUDE.md linting policy (pre-existing).

## Cross-session note

`darric-92` shipped PR #14 (`2026-09-05-model-download-progress`) mid-run, adding a `DOWNLOAD_LOCK`
inside `ensure_model` for the same root cause. Resolved as adjacent, not a collision: that lock stops
the file corruption, this unit stops the duplicate `Transcriber::new` and the `Err`-to-`None` swallow
that produced the reported symptom. Rebased onto it cleanly; the two locks nest outer-to-inner.

---

_Archived at promote 2026-09-05. Promoted: `2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber`, `2026-09-05-decision-capture-engine-requires-a-transcriber`, `2026-09-05-bug-the-session-start-guard-is-check-then-act`._
