# Proposal: model-download-progress

**Date**: 2026-09-05
**Status**: Draft

## Goal

Make the first-launch whisper model download visible in the UI. Today it is invisible on a
fresh install and the app reads as frozen: the Record button sits disabled at "Starting…"
for the several minutes a ~1.6 GB download takes, with nothing on screen explaining why.

## Why

The progress UI already exists — `RecorderPane.tsx` renders a labeled percentage and a
progress bar from a `downloadProgress` prop, fed by the `model_download_start` /
`model_download_progress` / `model_download_done` events `model.rs` emits. It is simply
mounted in the wrong place.

`RecorderPane` early-returns a "Select a recording, or press Record to start a new one."
placeholder whenever `session === null`, **before** reaching the progress block. On a fresh
install there are no recordings, so `session` is always null while the startup download runs
and the indicator can never render. The one moment the feature exists for is the one moment
it is unreachable.

The Record button then compounds it. `lib.rs` kicks off `ensure_model` at startup; pressing
Record before it finishes reaches `load_transcriber`, finds no cached transcriber, and calls
`ensure_model` again — which blocks on the full download. `Header.tsx` labels that "Starting…"
and disables the button, so the user gets a disabled control, a static empty pane, and no
indication that anything is happening. (That second `ensure_model` call is also a genuine
duplicate-download race against the startup one — both stream into the same `.tmp` path. This
proposal closes the only UI path that reaches it and files the underlying race separately.)

There is also no error event. `ensure_model` failures are logged and swallowed, so a failed
download would leave the new indicator pinned at its last percentage and the Record button
disabled forever. Surfacing the download honestly requires surfacing its failure too.

## Approach

**Lift the download indicator to app scope and make the Record button say what it is doing.**

1. **Emit `model_download_error`.** `model.rs::ensure_model` gains an emit on its failure
   paths, carrying the error string; both call sites (`lib.rs` startup, `sessions.rs`
   `load_transcriber`) keep their existing logging. Bind it in `lib/tauri.ts` alongside the
   three sibling events.

2. **Own the state in `useSession`.** `model_download_error` clears `downloadProgress` back to
   `null` and sets `error`, so a failed download releases the UI instead of stranding it.

3. **Render the indicator in `App.tsx`**, directly beneath `Header`, whenever
   `downloadProgress !== null` — outside `RecorderPane` and therefore independent of whether a
   recording is selected. Remove the block from `RecorderPane` and drop its now-unused
   `downloadProgress` prop.

4. **Make the button honest.** `Header` takes `downloadProgress`; while a download is in
   flight the label reads `Downloading 42%` rather than `Starting…`, and the button is disabled
   for the download as well as for `isStarting`. `RecorderPane`'s "Resume recording" button is
   gated the same way via `canResume`, closing the second path into `load_transcriber`.

Two alternatives were rejected:

- **Duplicate the progress block into `RecorderPane`'s `session === null` branch.** The
  smallest possible diff, and wrong: it leaves app-global state owned by a session-scoped
  component, so the indicator still vanishes the moment the user selects a recording
  mid-download, and the Record button still says "Starting…". It treats the symptom's location
  rather than the state's ownership.
- **A blocking modal overlay for the duration of the download.** Rejected because the download
  is a background startup task and browsing, renaming and deleting existing recordings are all
  legitimate during it. It also makes the failure mode strictly worse: a failed download traps
  the user behind an overlay rather than merely disabling one button.

## Success criteria

- With no recording selected (`session === null`) and a download in flight, the UI shows a
  labeled progress indicator with the current percentage. Verified by a test rendering `App`
  in that state.
- While a download is in flight the Record button label reads `Downloading <n>%`, not
  `Starting…`, and the button is disabled. Verified by a `Header` test.
- `RecorderPane` no longer accepts or renders `downloadProgress`; selecting or deselecting a
  recording does not change the indicator's visibility.
- `RecorderPane`'s "Resume recording" button is not offered while a download is in flight.
- A `model_download_error` event clears the progress indicator, re-enables the Record button,
  and surfaces the message in the existing error bar. Verified by a `useSession` test.
- `npm run lint`, `npm test`, `npm run build`, `cargo clippy` and `cargo test` all pass, with
  no lint suppressions added.

Added by the 2026-09-05 replan (see `replan.md`):

- The indicator is visible from the moment the window renders, not from the first progress
  event — verified by a test in which no event is emitted at all and the state is seeded from
  `model_download_state`.
- Two overlapping `ensure_model` calls produce exactly one download and one intact file.
- A failed download leaves no `.tmp` behind.
- The Stop button is never disabled while a recording is active.

Removed by the same replan: the deferral of the duplicate-download race to a tracker issue.
It is fixed at its source here.

## Open Questions

- None load-bearing. Exact copy ("Downloading speech model") is a judgment call made inline.
