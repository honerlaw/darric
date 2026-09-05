# Replan: model-download-progress

## 2026-09-05 — the event stream is not sufficient, and the UI guard does not close the race

### Original plan

Treat the download's invisibility as a mounting bug. The `model_download_start` /
`_progress` / `_done` events already existed and were already consumed by `useSession`; the
indicator was simply rendered inside `RecorderPane`, below its `session === null` early
return. Lift it to `App` scope, add a `model_download_error` event so the failure path has a
terminal counterpart, and make the Record and Resume buttons withhold themselves during a
download.

The duplicate-download race between `lib.rs`'s startup `ensure_model` and
`sessions.rs::load_transcriber`'s was explicitly deferred to a tracker issue, on the stated
grounds that this diff "closes the only UI path that reaches it".

### What changed

Code review found two connected facts that invalidate both halves of that reasoning.

1. **Events emitted during Tauri `setup()` are lost.** `emit` reaches only webviews that
   already hold a listener, and `useSession`'s `listen()` calls resolve hundreds of
   milliseconds after mount — long after `setup()` spawns `ensure_model`. So on the exact
   run this change exists for, a fresh install, `model_download_start` never arrives. The
   banner does not appear and the Record button is not disabled until the first
   `model_download_progress` tick, which at a 5% threshold is ~80 MB into a ~1.6 GB file.
   The "frozen app" window the unit set out to eliminate survives, minutes of it.

   The proposal's core assumption — that the existing event stream carried enough
   information and only its consumer was misplaced — is wrong. Event delivery is not
   guaranteed to a webview that has not mounted yet, so mount-time state has to be
   **queryable**, not merely broadcast.

2. **The UI guard therefore cannot be what closes the race.** Deferring the duplicate
   download was justified by the guard; the guard is not armed during precisely the window
   in which the second download is triggered. Filing an issue for a corruption that is
   permanent and has no in-app recovery — `ensure_model` accepts any existing file on an
   `exists()` check, with no size or checksum validation — while shipping a comment that
   claims the guard prevents it is not an honest record.

Two smaller findings ride along because they are the same failure surface: the single
Record/Stop button is disabled by the download gate even while it reads "Stop", so an active
recording cannot be stopped; and the new error branch leaves its partial `.tmp` behind.

### New plan

The approach is unchanged in shape — surface the download honestly at app scope — but it now
covers the download's _state_, not only its _events_, and it fixes the race at its source
rather than deferring it behind a guard that does not hold.

1. **Make download state queryable.** `model.rs` keeps the current percentage in a
   process-wide atomic (`-1` when no download is in flight), updated on every chunk. A new
   `model_download_state` command returns `Option<u32>`, and `useSession` calls it once on
   mount to seed `downloadProgress`. Events remain the live update path; the query closes
   the mount gap. Drop the emit threshold from 5% to 1% so the bar also moves visibly.

2. **Serialize `ensure_model`.** A process-wide `tokio::sync::Mutex` around the download,
   with an `exists()` re-check after acquiring it, so the second caller returns the file the
   first one finished instead of racing it into the same `.tmp`.

3. **Clean up on failure.** The error branch removes the partial `.tmp` before emitting
   `model_download_error`.

4. **Never disable Stop.** The download gate applies only when `!isRecording`.

5. Plus the small triaged fixes: clear a stale `error` on a new start, clamp the rendered
   percentage, correct the `canResume` comment and the banner's auto-start promise, and
   document the new first-launch behavior in `README.md`.

### Success criteria changes

Added:

- The download indicator is visible from the moment the window renders, not from the first
  progress tick — verified by a test that seeds `useSession` from `model_download_state`
  with no events emitted at all.
- Two overlapping `ensure_model` calls produce exactly one download and one intact file.
- A failed download leaves no `.tmp` behind.
- The Stop button is never disabled while a recording is active.

Removed:

- The proposal's deferral of the duplicate-download race to a tracker issue. It is fixed here.
