# Single-flight the whisper model load so a session started mid-download still transcribes

**Status**: Shipped (2026-09-05)
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
`commands/sessions.rs::load_transcriber` calls `model::ensure_model` _again_ when it finds no
cached transcriber. Neither knows about the other. Both stream into the same
`ggml-large-v3-turbo.tmp` (`tokio::fs::File::create` truncates it out from under the first writer),
and both then `tokio::fs::rename(tmp, bin)`. Exactly one rename can succeed; the loser gets
`ENOENT`.

Because the startup download begins earlier, it is reliably the winner, so the _session's_ call is
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

_Rewritten at promote to match what shipped._

**Model acquisition and transcriber construction are single-flight, shared by every caller.**
`AppState.transcriber` is a `TranscriberSlot` (`Arc<tokio::sync::Mutex<Option<Arc<Transcriber>>>>`).
A generic `get_or_init` in the new `transcription/loader.rs` holds that lock across the _whole_
acquire-and-load sequence — `ensure_model`, then `Transcriber::new` — so a second caller arriving
mid-download blocks and then observes the finished transcriber. `ensure_model` now has exactly one
call site in the crate, inside that lock.

An async mutex rather than a `OnceCell`, deliberately: a `OnceCell` resolved to an error caches the
failure, so one transient network failure at launch would disable transcription for the process
lifetime. `get_or_init` returns an `Origin` (`Cached` / `Loaded`) so the caller can log which case
it hit — the single most useful line for diagnosing this area.

`load_transcriber` was **deleted**, not converted: both callers (`lib.rs`'s startup pre-load and
`sessions.rs::begin_capture`) now go through `loader::get_or_load`.

**A missing transcriber is a hard failure.** `get_or_load` returns `Result`, `begin_capture`
propagates it, and `start_session` / `resume_session` return `Err` to the frontend, which already
renders `error` in `App.tsx`.

**`CaptureEngine` requires a transcriber.** `start` takes `&Arc<Transcriber>` and the `pool` field
is `Arc<TranscriptionPool>`. The original proposal kept the `Option` and justified it by claiming
"its tests construct it without a transcriber" — that claim was false; no test anywhere constructs
a `CaptureEngine`, and `None` had become unconstructible. Removing it deleted five unreachable
branches including both `if let Some(pool)` guards in `stop()`. Recorded as
`2026-09-05-decision-capture-engine-requires-a-transcriber`.

**`start_session` / `resume_session` are serialised.** Making the load blocking stretched the
existing check-then-act on `engine.is_some()` to the length of a model download, and an overwritten
`CaptureEngine` never stops (it has no `Drop`). An `AppState.session_transition` mutex is held
across each command. Recorded as `2026-09-05-bug-the-session-start-guard-is-check-then-act`.

**Both failure paths roll back completely.** `erase_session` removes `transcript_lines`,
`recording_segments` and the session in one transaction — `transcript_lines` included because its
foreign key to `sessions` has no cascade under `PRAGMA foreign_keys=ON`, so omitting it made the
rollback itself fail. `resume_session` reopens the session _before_ capturing and calls
`restore_ended_session` on failure; the reverse order left a live engine installed behind a
returned `Err`.

**A stale `whisper_model_path` setting falls back** to the downloaded model rather than failing
forever, since both paths now consult that setting and nothing in the UI can clear it.

`model.rs` was deliberately left untouched to stay clear of the concurrent
`2026-09-05-model-download-progress` unit, which added its own `DOWNLOAD_LOCK` inside
`ensure_model`. The two locks nest outer-to-inner with no cycle.

### Rejected alternatives

- **Have `load_transcriber` poll/wait for the startup task instead of loading anything.** Needs a
  timeout, and has no recovery path if the startup load already failed — a user who was offline at
  launch could never start a session without restarting the app.
- **Fix `ensure_model` internally only (unique temp filename, or a file lock).** Removes the
  `ENOENT`, but still downloads 1.6 GB twice and still builds two `WhisperContext`s, and still
  leaves the `Err`-to-`None` swallow in place. It treats the collision as a file-naming problem
  rather than the missing mutual exclusion it actually is.

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
  `2026-09-05-model-download-progress` unit, which shipped download progress in the UI as PR #14.

## Deferred work

- #16 — validate the cached whisper model instead of trusting `exists()`; both download locks are process-local so two app instances still race (priority: medium)
- #17 — `CaptureEngine::start`'s error path leaks tap UIDs in the `ExclusionRegistry`, permanently hiding those devices (priority: medium)
- #18 — `await_holding_lock = "allow"` in `Cargo.toml` contradicts the CLAUDE.md linting policy (priority: high)
