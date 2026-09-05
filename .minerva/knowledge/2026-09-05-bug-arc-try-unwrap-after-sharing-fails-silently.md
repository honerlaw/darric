# `Arc::try_unwrap` to reclaim a shared value fails silently and always

**Date**: 2026-09-05
**Type**: bug
**Summary**: reclaiming ownership with `Arc::try_unwrap(x).ok()` after clones have been handed out fails deterministically, and `.ok()` turns that failure into a plausible `None` that disables the feature with no error
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

`CaptureEngine::start` builds a transcription pool, wraps it in an `Arc`, hands a clone to each
device's capture thread, and then wanted the pool back to store on the struct:

```rust
let pool = Arc::new(pool);
for dev in devices {
    let pool_for_source = Arc::clone(&pool);   // one per device
    // … moved into the capture thread …
}
let pool = Arc::try_unwrap(pool).ok().flatten();   // strong_count == 1 + N
```

## Finding

With N devices the strong count is `1 + N`, and the engine refuses to start a recording with zero
devices — so `try_unwrap` failed on **every** run. It is not a race: there is no interleaving in
which it succeeds.

`.ok()` then converted `Err(the_pool)` into `None`, which is a perfectly ordinary value for an
`Option<TranscriptionPool>` field. Nothing logged, nothing failed, and the real pool stayed alive
inside the thread closures doing its job. Everything the engine wanted to do _through_ its own
handle silently became a no-op:

- the stop path's flush is `if let Some(pool)`, so every device's trailing partial segment was
  discarded on every stop;
- the dropped-segment counter always read 0, disabling the warning built specifically to stop a
  transcript being silently incomplete;
- `shutdown()` was never called, so worker threads stayed parked in `Condvar::wait` and their
  `JoinHandle`s were dropped unjoined — a thread leak per session.

The fix was to stop trying to reclaim sole ownership: `shutdown` takes `&self` and joins through a
drained `Mutex<Vec<JoinHandle>>`, and the engine holds `Option<Arc<TranscriptionPool>>` like every
other holder. The type now makes the mistake unexpressible.

## Implications

- `Arc::try_unwrap` is only appropriate where sole ownership is _structurally_ guaranteed. If a
  clone was handed to anything with a longer or unknown lifetime, it is guaranteed to fail.
- `.ok()` on a `Result` whose `Err` carries the value is where the evidence is thrown away. If
  `try_unwrap` must be used, match on the `Err` and at minimum log it.
- Prefer redesigning the API to work through a shared reference over reclaiming ownership. An
  operation that needs `self` by value forces exactly this problem on every caller.
- A silent `None` in a feature-gating `Option` is a bad failure shape generally: it is
  indistinguishable from "correctly configured off".

## Related

- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — how this survived a completion check that looked directly at the affected function
- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — see also
