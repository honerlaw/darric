# Events emitted from Tauri `setup()` reach no webview, so startup state must be queryable

**Date**: 2026-09-05
**Type**: constraint
**Summary**: `emit` only reaches webviews already holding a listener, so anything emitted during `setup()` is lost and needs a command the frontend can poll on mount
**Context**: .minerva/work/2026-09-05-model-download-progress (see git history if the worktree has been cleaned up)

## Context

`lib.rs`'s `setup()` spawns `model::ensure_model` so the Whisper model is downloading and loaded
before the user asks for a recording. `ensure_model` emits `model_download_start` immediately,
then `model_download_progress` as bytes arrive, and `useSession` listens for all of them.

A fix that moved the progress indicator to app scope looked complete and was not.

## Finding

**Tauri's `emit` delivers only to webviews that already hold a listener for that event.** It does
not queue, replay, or buffer for a webview that has not yet mounted. `setup()` runs before the
frontend exists, and `useSession`'s `listen()` calls resolve hundreds of milliseconds after mount
— so `model_download_start` is emitted into an empty room, every single time, on the one run
that matters.

The observable consequence was sharp. With progress emitted every 5%, the frontend learned
nothing until the download crossed 5% of ~1.6 GB — roughly 80 MB, minutes on a slow link. For
that entire window the banner was absent and the Record button was enabled, which is precisely
the "frozen app" symptom the work set out to remove.

The fix is a **queryable counterpart**: `model.rs` keeps the live percentage in a process-wide
atomic, a `model_download_state` command returns it, and the frontend seeds itself from that
query on mount and takes events from there. Events remain the live update path; the query
handles the mount gap.

## Implications

- **An event-only contract is incomplete for anything that can start before the frontend
  mounts.** Any background task kicked off in `setup()` needs a command exposing its current
  state, not just events announcing its transitions. The event stream describes _changes_; a
  client that missed the changes needs a way to read the _value_.
- The symptom is a race, so it is invisible in any test that emits the event after render, and
  invisible in dev when the model is already cached. The test that pins it emits nothing at all
  and asserts the UI is correct purely from the seeded query.
- Emit granularity sets the width of the blind window when a query is absent. Dropping the step
  from 5% to 1% shrank it, but shrinking a window is mitigation — the query is the fix.

## Related

- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — the other, unrelated reason the same download was invisible
- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — the race this delivery gap left unguarded
- [[2026-09-06-constraint-tauri-setup-runs-outside-the-tokio-runtime]] — see also
- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — see also
