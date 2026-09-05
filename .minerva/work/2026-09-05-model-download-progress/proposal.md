# Proposal: model-download-progress

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)

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
duplicate-download race against the startup one — both stream into the same `.tmp` path. The
original plan deferred it; review showed the UI guard could not close it, so it is fixed at its
source here. See `replan.md` and
[[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]].)

There is also no error event. `ensure_model` failures are logged and swallowed, so a failed
download would leave the new indicator pinned at its last percentage and the Record button
disabled forever. Surfacing the download honestly requires surfacing its failure too.

## Approach

**The download is surfaced at app scope, and its state is queryable rather than only
broadcast.** The second half was not in the original plan; see `replan.md`.

1. **`ModelDownloadBanner`** (new) renders from `App`, directly under the header, whenever a
   download is in flight — outside `RecorderPane` and so independent of whether a recording is
   selected. It carries a real `progressbar` role and clamps the percentage it is given. The
   old block inside `RecorderPane`, which sat below that component's `session === null` early
   return and could therefore never render on a fresh install, is gone along with its prop.

2. **Download state is queryable.** `model.rs` keeps the live percentage in a process-wide
   atomic, updated on every chunk, and a `model_download_state` command returns it.
   `useSession` seeds `downloadProgress` from that query on mount, preferring any value a live
   event already supplied. This is what makes the indicator correct on a first launch: the
   startup download begins in Tauri's `setup()`, and events emitted there reach no webview at
   all. The emit step also dropped from 5% to 1%, so the bar moves rather than appearing to
   stall for ~80 MB at a time.

3. **`ensure_model` is serialised.** A process-wide `tokio::sync::Mutex`, with an `exists()`
   re-check after acquiring it, so the startup pre-load and a `start_session` that overlaps it
   produce one download rather than two interleaved writes into a shared `.tmp`. The original
   plan deferred this race to a tracker issue on the strength of the UI guard below; that
   reasoning did not survive review.

4. **Failures are terminal and visible.** A new `model_download_error` event carries the
   message; `ensure_model` removes the partial `.tmp` before emitting it. `useSession` clears
   the progress and puts the reason in the existing error bar, and clears that stale message
   again when a later start or resume begins.

5. **The buttons say what they are doing.** `Header`'s label reads `Downloading <n>%` while a
   download runs, taking precedence over `Starting…`, and the gate applies only to _starting_ a
   recording — the same button serves as Stop, and disabling it mid-recording would strand the
   user. `RecorderPane`'s Resume is withheld through `canResume` for the same reason.

Two alternatives were rejected at design time:

- **Duplicate the progress block into `RecorderPane`'s `session === null` branch.** The smallest
  possible diff, and wrong: it leaves app-global state owned by a session-scoped component, so
  the indicator still vanishes the moment a recording is selected mid-download, and the Record
  button still says "Starting…".
- **A blocking modal overlay for the duration of the download.** The download is a background
  startup task and browsing, renaming and deleting existing recordings are legitimate during it.
  It also makes the failure mode strictly worse: a failed download would trap the user behind an
  overlay rather than merely disabling one button.

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
