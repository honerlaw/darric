# Single-flight the whisper model load so a session started mid-download still transcribes

**Status**: Draft
**Date**: 2026-09-05

## Goal

A recording started while the whisper model is still downloading must end up with a working
transcriber. If it genuinely cannot get one, the session must fail loudly instead of recording
audio that is silently never transcribed.

## Why

Reported symptom: level bars animate for every input and output device, no transcript line ever
appears, and stopping processes nothing.

Diagnosed from runtime state on the reporter's machine:

- The session ran `2026-09-05T18:37:30Z → 18:40:17Z`, but `ggml-large-v3-turbo.bin` was not renamed
  into place until `18:39:35Z` — the download was still running when Record was pressed.
- That session has **zero** rows in `transcript_lines`.

The mechanism: `lib.rs`'s startup task calls `model::ensure_model` to download the model, and
`commands/sessions.rs::load_transcriber` calls `model::ensure_model` *again* when it finds no
cached transcriber. Neither knows about the other. Both stream into the same
`ggml-large-v3-turbo.tmp` (`tokio::fs::File::create` truncates it out from under the first writer),
and both then `tokio::fs::rename(tmp, bin)`. Exactly one rename can succeed; the loser gets
`ENOENT`.

Because the startup download begins earlier, it is reliably the winner, so the *session's* call is
reliably the loser. `ensure_model` returns `Err`, and `load_transcriber` converts that into a plain
`None`:

```rust
Err(e) => { log::error!("[session] model unavailable: {e}"); None }
```

`CaptureEngine::start` takes `Option<Arc<Transcriber>>`, so `None` builds no `TranscriptionPool` and
only logs a warning. Capture and metering still run — hence the moving bars — but nothing is ever
submitted for transcription, and `stop()`'s trailing-segment flush is itself behind
`if let Some(pool)`, so stop processes nothing either.

This is the failure shape already recorded in
`2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently`: a silent `None` in a feature-gating
`Option`, indistinguishable from "correctly configured off". That entry's own stated implication —
prefer a shape where the mistake is unexpressible — is what this unit applies.

## Approach

**Make model acquisition and transcriber construction single-flight, shared by every caller.**

`AppState.transcriber` becomes `Arc<tokio::sync::Mutex<Option<Arc<Transcriber>>>>`. A single helper
holds that lock across the *whole* acquire-and-load sequence (resolve path → `ensure_model` →
`Transcriber::new` → store), so a second caller arriving mid-download blocks on the lock and then
finds the already-loaded transcriber rather than starting a competing download. One download, one
`Transcriber::new`, one 1.6 GB resident copy.

An async mutex is used rather than `OnceCell` deliberately: a `OnceCell` that resolved to an error
would cache the failure, so a transient network failure would poison transcription for the rest of
the process lifetime. Holding a mutex and re-checking leaves the next caller free to retry.

Both callers go through that one helper:

- `lib.rs`'s startup pre-load, so the model is ready before the first session where possible.
- `sessions.rs::load_transcriber`, so a session started mid-download waits for the in-flight load
  instead of racing it.

**Fail loudly.** `load_transcriber` returns `Result<Arc<Transcriber>>` rather than `Option`, and
`begin_capture` propagates the error. `start_session` / `resume_session` then return `Err` to the
frontend, which already renders `error` in `App.tsx`. A session that cannot transcribe is worthless
— darric persists no audio, only transcript lines — so refusing beats recording nothing silently.

`CaptureEngine::start` keeps its `Option<Arc<Transcriber>>` parameter: that is the honest type for
the engine, and its tests construct it without a transcriber. The change is that the *session* path
can no longer pass `None` by accident.

**Roll back the session rows when capture fails to start.** `start_session` inserts the `sessions`
and `recording_segments` rows before `begin_capture`. Making `begin_capture` fail more often would
otherwise litter the sessions list with empty sessions, so a failed start now deletes the rows it
just inserted.

### Rejected alternatives

- **Have `load_transcriber` poll/wait for the startup task instead of loading anything.** Needs a
  timeout, and has no recovery path if the startup load already failed — a user who was offline at
  launch could never start a session without restarting the app.
- **Fix `ensure_model` internally only (unique temp filename, or a file lock).** Removes the
  `ENOENT`, but still downloads 1.6 GB twice and still builds two `WhisperContext`s. It treats the
  collision as a file-naming problem rather than the missing mutual exclusion it actually is.

## Success criteria

- [ ] Only one whisper model download and one `Transcriber::new` can be in flight per process, no
      matter how many callers ask concurrently.
- [ ] A caller arriving while a load is in flight receives the same `Arc<Transcriber>` the in-flight
      load produces, rather than starting its own.
- [ ] A failed load is not cached: the next caller retries.
- [ ] `start_session` / `resume_session` return an error when no transcriber can be obtained, rather
      than starting a capture that transcribes nothing.
- [ ] A `start_session` that fails leaves no `sessions` or `recording_segments` rows behind.
- [ ] `cargo clippy`, `cargo test`, `cargo fmt --check`, and the TypeScript checks all pass with no
      new `#[allow]` or `eslint-disable`.

## Open Questions

- None outstanding. The blocking-start UX — `start_session` awaits the whole download, so the Record
  button appears stuck for minutes — is real but belongs to the concurrent
  `2026-09-05-model-download-progress` unit, which is surfacing download progress in the UI.
