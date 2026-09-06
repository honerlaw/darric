# Two concurrent model downloads interleaved into one `.tmp` and cached the result forever

**Date**: 2026-09-05
**Type**: bug
**Summary**: startup pre-load and `load_transcriber` both called `ensure_model`, writing one `.tmp` at independent offsets; the mixed result was renamed in and accepted by a bare `exists()` check on every later launch
**Context**: .minerva/work/2026-09-05-model-download-progress (see git history if the worktree has been cleaned up)

## Context

Two independent call sites reach `model::ensure_model`:

- `lib.rs`'s `setup()` spawns it so the model is ready before the first recording;
- `commands::sessions::load_transcriber` calls it when a session starts and no transcriber is
  cached yet.

On a first launch these overlap by construction. The startup download takes minutes; a user who
presses Record during it reaches the second call site, which finds `state.transcriber` still
`None` and calls `ensure_model` again.

## Finding

`ensure_model` had no serialisation. Both calls saw `path.exists() == false`, both ran
`File::create` on the same `ggml-large-v3-turbo.tmp` — the second truncating the first — and
both then wrote at their own independent offsets. Whichever finished first renamed the tmp into
place; the loser kept writing into that same inode and its own `rename` failed `ENOENT`.

The resulting `.bin` is a mix of two byte streams. Nothing detects that, because `ensure_model`'s
cache check is `path.exists()` with no size or checksum validation, so **every subsequent launch
accepts the corrupt file as a valid cached model**. The install is permanently broken with no
in-app recovery path, and the user's only signal is that transcription silently produces nothing.

A second, milder symptom: the loser's `ENOENT` surfaced to the user as
"Speech model download failed: No such file or directory (os error 2)" for a download that had
in fact succeeded.

The fix is a process-wide `tokio::sync::Mutex` in `model.rs` around the download, with a second
`path.exists()` check **after** acquiring it, so a caller that waited returns the file the winner
finished rather than starting its own.

## Implications

- **A UI guard was proposed as the fix first, and was not sufficient.** The plan was to disable
  Record and Resume while a download is in flight, closing the only user-reachable trigger. That
  reasoning failed on
  [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]]: the frontend does not learn
  a startup download is running until it queries for it, so the guard is not armed during exactly
  the window in which the second call happens. A guard on the caller cannot protect an invariant
  that belongs to the callee.
- **`exists()` is not a cache validity check.** Any download-to-cache path that renames a
  partial file into its final name inherits this: once a bad file is in place it is
  indistinguishable from a good one. Validating size or checksum before accepting the cache would
  make the corruption recoverable; today it is not.
- The failure path now also removes the partial `.tmp`, which previously accumulated up to
  ~1.6 GB of dead bytes that nothing ever reclaimed.

## Related

- [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]] — why the UI guard originally proposed as the fix could not be armed in time
- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — another failure in this codebase that produced a plausible result instead of an error
- [[2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber]] — see also
- [[2026-09-05-reference-model-rs-download-paths-have-no-tests]] — see also
- [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]] — see also
