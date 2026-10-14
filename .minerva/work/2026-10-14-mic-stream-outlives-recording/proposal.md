# Proposal: mic-stream-outlives-recording

**Date**: 2026-10-14
**Status**: Draft

## Goal

Stop microphone capture streams from outliving the recording that opened them, by upgrading
`cpal` from 0.15.3 to 0.18.2, whose CoreAudio stream is stopped when dropped. In the same
change, make every per-segment `[whisper]` log line name the device the segment came from, so
the next speaker-capture dropout can be attributed from the logs.

## Why

After the 2026-10-14 recording (session `d5b66d8d`) was stopped at 15:48:11, the dev log reads
`[source] MacBook Pro Microphone stopped`. It then logs 2,474
`[whisper] queue full — dropped the oldest segment (device MacBook Pro Microphone)` warnings over
the next five hours. A `sample` of the running process shows a Core Audio IO thread still
running `audio::source::build_stream`'s data callback and the resampler, with no `capture-*` or
`whisper-*` threads left. The process sits at ~36% CPU while idle, and the mic stays open.

Cause: in cpal 0.15.3 (`host/coreaudio/macos/mod.rs:449-466`), `add_disconnect_listener`
stores a strong `stream.clone()` inside the stream's own `StreamInner._disconnect_listener`.
That is a reference cycle, and `Stream` has no `Drop` that stops the audio unit. So when
`source::supervise` drops the stream, the `AudioUnit` keeps running for the life of the
process. cpal 0.16.0's changelog lists "CoreAudio: Fix callback being called after dropping the
stream". 0.18.2 (2026-08-16) keeps a `Weak<Mutex<StreamInner>>` in its listeners and has
`impl Drop for Stream`. The iPhone stream does not leak only because cpal pauses it itself on
disconnect. Every working mic in every recording leaks until the app quits.

The second change is diagnostic. In the same recording, the speaker tap's transcript stopped
at ~15:03:11 while audio kept playing through the speakers, and the log shows only
`[whisper] no speech in 25600 samples` lines with no device name. So it cannot say which
source went silent. That dropout (the tap stays alive but stops seeing Meet's audio) is **not**
fixed here. Its cause is unconfirmed and needs a hardware repro with the user. It will be
filed as a follow-up.

## Approach

1. **Upgrade cpal to 0.18.2** in `src-tauri/Cargo.toml` (`cpal = "0.18"`). Migrate darric's two
   cpal call sites, `audio/device.rs` and `audio/source.rs`:
   - Replace the deprecated `device.name()` with `device.description()` → its `name()`. **Device
     identity stays the human-readable name**, because `capture.disabled_devices` (persisted in
     `settings`) and `transcript_lines.device_id` are keyed by it. Switching to cpal's new
     stable `id()` would silently orphan every persisted disabled device.
   - Replace `SampleRate` struct access (`.0`) with the `u32` alias.
   - Pass `StreamConfig` by value.
   - Adapt to the unified `cpal::Error` type where errors are stringified.
   - Keep the explicit `stream.play()`. 0.18 no longer auto-starts streams.
2. **Regression test (hardware, `#[ignore]`d)** in `audio/source.rs`. It runs `run_source` on
   the default input device in a thread, waits until at least one callback has arrived, sets
   `shutdown`, joins the thread, waits ~1 s, and asserts that no callback ran after the join.
   It is written and run **before** the upgrade, to show it fails on 0.15.3, and run again
   after to show it passes.
3. **Device-attributed whisper logs.** `Transcriber::transcribe` gains a `source: &str`
   parameter used in its per-segment debug lines (`too short`, `no speech`, `N of M samples are
speech`, the final line). The pool passes `job.device_name`, and test call sites pass a
   fixed label.

### Candidate approaches considered

- **A — Upgrade cpal to 0.18.2 (recommended).** Fixes the leak at its source, in the library
  that owns the bug. It also picks up 0.17–0.18's other CoreAudio fixes (a device-enumeration
  segfault, UB around null pointers). Cost: an API migration in two files.
- **B — Stay on 0.15.3 and work around it.** Wrap the stream in a guard whose `Drop` calls
  `pause()`, and route the data callback through a slot the supervisor empties at teardown.
  This stops the audio, but the `AudioUnit` plus listener still leak per stream, and darric
  keeps a dependency with a bug that is fixed upstream.
- **C — Upgrade to 0.16.0 only**, the first release with the fix. Less churn than A, since
  `name()` is not yet deprecated there. But it is a 16-month-old release that still needs the
  0.17/0.18 migration later, and lacks the later CoreAudio fixes.

## Success criteria

- `src-tauri/Cargo.lock` resolves `cpal` to `0.18.2`, and
  `grep -rn "\.name()" src-tauri/src/audio/device.rs src-tauri/src/audio/source.rs` finds no
  call to the deprecated `DeviceTrait::name()`. A code comment at the device-identity site says
  the id is the description name, and why it is not `id()`.
- **The leak test is a hard gate.** The new ignored test `a_stopped_source_delivers_nothing_more`
  in `audio/source.rs` works like this:
  - It builds a `CaptureDevice` from `cpal::default_host().default_input_device()`, named with
    `name()` on the 0.15.3 run and with `description().name()` after the upgrade. That edit
    between the two runs is expected.
  - The callback increments a shared `Arc<AtomicUsize>` that the test owns, so the count
    outlives `run_source`.
  - It runs `run_source` on a thread and waits up to 5 s for at least one callback. On timeout
    it **panics** with "no callbacks — is audio capture permitted for this terminal?", so a
    permission failure can never read as a pass or as a leak.
  - It sets `shutdown` and joins, sleeps 200 ms for any in-flight callback, snapshots an atomic
    callback counter, sleeps 1 s more, and asserts the counter did not move.
  - It prints the before/after counts.
- Run with
  `cargo test --manifest-path src-tauri/Cargo.toml --lib a_stopped_source -- --ignored --nocapture`:
  - On a tree with only the test added (cpal 0.15.3), it must show callbacks before shutdown
    **and** a counter that keeps rising after the join.
  - On the upgraded tree, it must pass.
  - Both outputs, with counts, are recorded in the scratchpad and quoted in the PR body.
  - If the test cannot get callbacks from this environment (no permission), the implementer
    **stops and asks the user**. Claiming the gate passed is not allowed.
  - If it still fails on 0.18.2, the unit replans rather than merging the upgrade.
- **Device names are byte-identical.** The existing ignored test
  `audio::device::tests::enumeration_yields_unique_stable_ids` is extended to print every
  `list_input_devices` name. It is run on 0.15.3 and on 0.18.2, and the two outputs are recorded
  in the scratchpad. They must be exactly equal strings, for every device present in both runs.
  This matches the source-level evidence that 0.15.3's `name()` and 0.18.2's
  `description().name()` (via coreaudio-rs 0.14.2 `get_device_name`) both read
  `kAudioDevicePropertyDeviceNameCFString`, Output scope, Master element.
- **Whisper lines name their device.** `Transcriber::transcribe` takes `source: &str`, and the
  pool passes `job.device_name`, the same field the existing `queue full` line uses. Each of
  its four debug lines (`too short`, `no speech`, `N of M samples are speech`, and the final
  `{line:?}`) includes `{source}`. This is checked by reading the four `log::debug!` calls in
  `transcription/mod.rs`, and by a `RUST_LOG=darric_lib=debug` run of an existing ignored
  transcription test whose output shows the label.
- `npm run check` passes, `cargo test --manifest-path src-tauri/Cargo.toml` passes,
  `npm run lint:rust` (which is `--all-targets`, so it covers the ignored tests) reports no warnings that are absent on `main`, and no lint suppression is
  added (`CLAUDE.md`).

## Open Questions

- **Accepted gap:** this unit never verifies the fix in the real app. An end-to-end check
  (record, stop, then confirm with `sample` that no source callback runs and that no
  `queue full` lines follow the stop) needs the user to relaunch their dev build from merged
  `main`. It is not a merge gate; the hardware test is the only proof of the mechanism, and CI
  cannot run it.
- `source.rs` assumes the default input format is F32, because its callback is `&[f32]`.
  0.18.2's CoreAudio `default_config` accepts only F32 or I16, so an I16-only device is a
  pre-existing gap and not a regression. It is out of scope.
- Observed while reading `audio/device.rs`: `list_input_devices` filters on
  `excluded.contains(&name)`, but the registry holds aggregate **UIDs** (`darric-tap-<uuid>`),
  not names. That filter may never match. It is adjacent and out of scope, and goes in the
  speaker-capture follow-up issue.
