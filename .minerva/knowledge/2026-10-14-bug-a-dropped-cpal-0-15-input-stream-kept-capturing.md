# A dropped cpal 0.15 input stream kept capturing until the app quit

**Date**: 2026-10-14
**Type**: bug
**Theme**: audio-capture
**Summary**: cpal 0.15.3's CoreAudio stream referenced itself, so drop never stopped it; fixed by cpal 0.18.2
**Context**: .minerva/work/2026-10-14-mic-stream-outlives-recording (see git history if the worktree has been cleaned up)

## Context

After a recording was stopped, the dev log read `[source] MacBook Pro Microphone stopped`. It then
logged 2,474 `[whisper] queue full — dropped the oldest segment (device MacBook Pro Microphone)`
warnings over the next five hours. A `sample` of the process showed a Core Audio IO thread still
running `source::build_stream`'s data callback, with no `capture-*` or `whisper-*` threads left.
The app sat at ~36% CPU while idle, and the microphone stayed open.

## Finding

In cpal 0.15.3, `add_disconnect_listener` (`host/coreaudio/macos/mod.rs`) stored a strong
`stream.clone()` inside the stream's own `StreamInner`. `Stream` had no `Drop` that stopped the
audio unit, so dropping darric's handle released nothing: every working microphone of every
recording kept delivering for the life of the process. A disconnected device escaped only because
cpal paused it itself on disconnect.

cpal 0.16.0 fixed this ("CoreAudio: Fix callback being called after dropping the stream"). 0.18.2,
which darric now uses, holds a `Weak` reference and stops the unit in `Drop`. The ignored test
`audio::source::tests::a_stopped_source_delivers_nothing_more` reproduced the leak on 0.15.3:
callbacks went from 39 to 133 in the second after the source was joined. It passes on 0.18.2,
where the count stays flat.

## Implications

- Dropping a cpal `Stream` is now what stops capture. Code that wants a stream to stop must
  drop it (or `pause()` it), not just stop reading from it.
- Only hardware can catch this kind of leak. The regression test is `#[ignore]`d and runs
  only locally, from a terminal that holds the audio-capture grant.

## Related

- [[2026-10-14-constraint-cpal-0-18-reports-xruns-through-the-stream-error-callback]] — what the upgrade changed about stream errors
- [[2026-10-14-decision-input-device-identity-stays-the-device-name]] — how the upgrade kept device ids stable
- [[2026-09-06-reference-a-test-binary-holds-the-audio-permission-of-its-terminal]] — why the hardware test can run locally
