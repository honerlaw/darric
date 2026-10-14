# cpal 0.18 reports xruns through the stream error callback, and they are not fatal

**Date**: 2026-10-14
**Type**: constraint
**Theme**: audio-capture
**Summary**: CoreAudio overloads arrive as ErrorKind::Xrun while the stream keeps running; don't rebuild on them
**Context**: .minerva/work/2026-10-14-mic-stream-outlives-recording (see git history if the worktree has been cleaned up)

## Context

`source::build_stream` treated every call of cpal's error callback as "the stream is gone": it set
`stream_failed`, and `supervise` dropped the stream and rebuilt it with backoff. That was correct
on cpal 0.15.3, whose CoreAudio backend reported only device loss there.

## Finding

cpal 0.18.2 registers a `kAudioDeviceProcessorOverload` listener on every CoreAudio stream and
reports each overload as `ErrorKind::Xrun`, without stopping anything. Under the old handling, an
overload (likely while whisper runs on the GPU) tore down a healthy microphone and dropped audio
during the rebuild. Overloads recurring within `STABLE_AFTER` (5 s) of each rebuild would also
keep `failing_since` set until `GIVE_UP_AFTER` (60 s) marked the mic failed for the rest of the
recording. `source::is_fatal` now classifies errors: `Xrun` is logged at warn and the stream
keeps running, and every other kind (`DeviceNotAvailable`, `StreamInvalidated`, …) still
rebuilds. A unit test pins the classification.

## Implications

- `cpal::ErrorKind` is `#[non_exhaustive]` and grows between releases. Any new non-fatal kind
  must be added to `is_fatal`'s exclusion, because the default is fatal.
- 0.18.2 also reports a nominal sample-rate change as `StreamInvalidated`. Rebuilding on it is
  correct, because `build_stream` re-reads the rate.

## Related

- [[2026-10-14-bug-a-dropped-cpal-0-15-input-stream-kept-capturing]] — the upgrade that introduced this
