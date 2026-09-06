# A `cargo test` binary holds whatever audio-capture permission its terminal already has

**Date**: 2026-09-06
**Type**: reference
**Summary**: TCC grants audio capture to the responsible process, so once the dev app tapped a device from a terminal, test binaries launched from that terminal tap it too
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

[[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]] recorded that a
bare test binary "can never hold this permission" because it has no bundle identifier to grant
against, and that output capture therefore cannot be runtime-verified by any test in the repo.

## Finding

`audio::hardware::taps_transcribe_only_the_device_that_played`, an ignored test in the plain
`cargo test` binary, created a process tap on MacBook Pro Speakers, started it, received audio
while `afplay` played a fixture, and transcribed the fixture verbatim. No prompt appeared.

macOS TCC attributes a permission request to the _responsible_ process — for a command-line
program, the terminal application that launched it. `npm run tauri:dev` runs `target/debug/darric`
under that same terminal, and the first time it started a tap the terminal was prompted and
granted. Every process the terminal launches afterwards, test binaries included, inherits that
grant. The earlier entry was written before any grant existed, when the prompt had nowhere to
attach.

## Implications

- Hardware-level tests of taps and microphones can run locally, on a machine where the dev app
  has already been granted capture through the same terminal. They still cannot run in CI and
  stay `#[ignore]`d.
- A test that fails with `AudioDeviceStart` OSStatus 268451843 on a fresh machine is not broken:
  launch the dev app from that terminal once and grant the prompt.
- The grant follows the terminal, so a test run from a different terminal app, an IDE, or a
  Claude Code session launched elsewhere may not hold it.

## Related

- [[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]] — contradicts
