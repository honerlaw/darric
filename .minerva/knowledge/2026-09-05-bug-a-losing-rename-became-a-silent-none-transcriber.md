# The losing `ensure_model` rename returned `Err`, which became a silent `None` transcriber

**Date**: 2026-09-05
**Type**: bug
**Summary**: a session started mid-download raced the startup `ensure_model`; the loser's `rename` got `ENOENT`, and `load_transcriber` turned that `Err` into `None`, so capture and metering ran normally while nothing was ever transcribed
**Context**: .minerva/work/2026-09-05-transcriber-single-flight

## Context

Reported as: level bars animate for every input and output device, no transcript line ever
appears, and pressing stop processes nothing.

[[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] describes the same race one layer
down and predicts that "the user's only signal is that transcription silently produces nothing".
This is the confirmed field instance of that prediction — but by a **different mechanism than that
entry assumes**, which is why it is recorded separately.

## Finding

The evidence, from the reporter's machine:

| Fact                                         | Value                      |
| -------------------------------------------- | -------------------------- |
| session ran                                  | `18:37:30Z` → `18:40:17Z`  |
| `ggml-large-v3-turbo.bin` renamed into place | `18:39:35Z`                |
| rows in `transcript_lines` for that session  | `0`                        |
| size of the resulting `.bin`                 | `1624555275` — **correct** |

The model file was **not corrupt**. Both racers write the same bytes to the same offsets of the
same `.tmp`, so the surviving file was byte-correct. The damage was entirely in the control flow:
the winner renamed, the loser's `tokio::fs::rename` failed `ENOENT`, and `ensure_model` returned
`Err`. `load_transcriber` then did this:

```rust
Err(e) => { log::error!("[session] model unavailable: {e}"); None }
```

`CaptureEngine::start` accepted `Option<Arc<Transcriber>>`, so `None` built no
`TranscriptionPool` and logged a warning. Capture threads, taps and RMS metering all ran — hence
the moving bars — and every completed segment hit `if let Some(p) = pool` and was discarded.
`stop()`'s trailing-segment flush sat behind the _same_ guard, so stopping also did nothing.

The startup path surfaced its `ENOENT` as a visible "download failed" error. The session path
swallowed it. Same `Err`, two dispositions, and only the silent one was user-facing.

## Implications

- **Corrupt output is not the only way a download race hurts you.** The file can be perfect and
  the feature still dead, because the loser's _error_ is the payload. Do not conclude from a
  correct checksum that a download race did no damage.
- **Diagnosing this needs three timestamps, not a log.** Nothing was reproducible after the fact,
  but `stat` on the model file, `started_at`/`ended_at` on the session, and a `COUNT(*)` on
  `transcript_lines` pinned it exactly: the model landed two minutes _after_ the session began, so
  `ensure_model` provably ran a second time. Prefer that triangulation over waiting for a repro.
- Serialising the _download_ alone (a lock inside `ensure_model`) fixes the corruption but not
  this: the second caller still builds its own 1.6 GB `Transcriber`, and a genuine failure still
  becomes `None`. The mutual exclusion has to span acquire **and** load — see
  [[2026-09-05-decision-capture-engine-requires-a-transcriber]] for the type-level half.

## Related

- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — the same race at the download layer, and the `.tmp` corruption it causes
- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — the previous instance of a silent `None` disabling transcription in this exact position
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — why the `if let Some(pool)` guard survived an earlier review
- [[2026-09-05-decision-capture-engine-requires-a-transcriber]] — the change that makes this shape unexpressible
