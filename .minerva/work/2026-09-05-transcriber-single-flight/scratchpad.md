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
