# `model.rs`'s download paths cannot be exercised from a test

**Date**: 2026-09-05
**Type**: reference
**Summary**: `MODEL_URL` is a hard-coded `const`, so no Rust test can reach the status, mid-stream, rename, cleanup or serialisation paths; `model.rs` is the one module of seven with no test block
**Context**: .minerva/work/2026-09-05-model-download-progress (see git history if the worktree has been cleaned up)

## Context

`src-tauri/src/model.rs` owns the whole model-acquisition path: the cache check, the HTTP
download, progress reporting, the `.tmp` write-and-rename, failure cleanup, and the mutex that
serialises concurrent callers.

## Finding

`MODEL_URL` is a module-level `const` pointing at Hugging Face, and `ensure_model` takes no
injection point for it. A test therefore cannot reach any of the download's behavior without
network access to that specific URL and a ~1.6 GB transfer. Six of the crate's seven modules
carry a `#[cfg(test)] mod tests` block; `model.rs` carries none.

The paths with no coverage include the ones most recently changed and least obvious: a non-2xx
response, a mid-stream chunk error, a failed rename, the removal of the partial `.tmp` on
failure, and the `DOWNLOAD_LOCK` re-check that prevents
[[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] from recurring.

The frontend half of the same behavior is well covered — `App.test.tsx` and
`Header.test.tsx` pin the indicator, the seeding, and the button states, and those tests were
mutation-checked. The gap is entirely on the Rust side.

## Implications

- The correctness of the serialising lock and the failure cleanup currently rests on reading the
  code, not on a check that would fail if someone removed them.
- Making these paths testable means taking the URL as a parameter or a config value rather than a
  `const` — a small change, but one nothing currently forces.

## Related

- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — the fix living in this untested module
