# `CaptureEngine` takes a transcriber, not an `Option<transcriber>`

**Date**: 2026-09-05
**Type**: decision
**Summary**: the optional transcriber let two separate bugs present as a normal recording that transcribed nothing, so the parameter is now required and callers that cannot obtain one must fail
**Context**: .minerva/work/2026-09-05-transcriber-single-flight

## Decision

`CaptureEngine::start` takes `&Arc<Transcriber>`. The engine's `pool` field is
`Arc<TranscriptionPool>`, not `Option<_>`. A caller that cannot obtain a transcriber returns an
error instead of starting a capture.

## Why

The `Option` was justified by a comment: "With no transcriber the audio is still captured and
metered — the UI stays honest about which devices are live while the model loads." That was never
true in the shipped app. `begin_capture` awaited the model **before** constructing the engine, so
`None` never meant "still loading" — it only ever meant "loading failed", and it rendered as a
recording indistinguishable from a working one.

Two distinct defects reached users through that single `None`:

- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — `Arc::try_unwrap(...).ok()`
  yielded `None` on every run;
- [[2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber]] — a losing `rename`'s `Err`
  was mapped to `None`.

Different causes, same escape hatch, same user-visible result. The second one is what made the
type the problem rather than either call site: an `Option` that is only ever `None` by mistake is
a defect channel, not a feature.

Removing it deleted five unreachable branches, including the two `if let Some(pool)` guards in
`stop()` — one of which is exactly the guard that made
[[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] possible, because a
reviewer verified the flush ordering _inside_ a block that never ran.

## Implications

- **A feature-gating `Option` whose `None` arm is unreachable is worse than useless** — it is
  where the next silent failure will hide. If no caller can legitimately pass `None`, the type
  should not permit it.
- Darric persists transcript lines and **no audio**, so a session without a transcriber produces
  nothing recoverable. Refusing to start is not a degradation; continuing was the degradation.
- The error now reaches the user: `start_session` / `resume_session` return `Err`, and `App.tsx`
  already renders it.
- If a "record now, transcribe later" mode is ever wanted, it needs buffered audio on disk and a
  deliberate design — not a `None` that silently discards every segment.

## Related

- [[2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber]] — the report that forced this
- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — the earlier bug through the same `Option`
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — the review failure the removed guard enabled
- [[2026-09-05-bug-the-session-start-guard-is-check-then-act]] — see also
- [[2026-09-05-reference-stop-session-releases-the-engine-before-teardown]] — see also
