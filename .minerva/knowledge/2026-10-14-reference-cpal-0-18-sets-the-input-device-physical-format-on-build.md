# cpal 0.18 sets the input device's physical format to F32 on every stream build

**Date**: 2026-10-14
**Type**: reference
**Theme**: audio-capture
**Summary**: Building a CoreAudio input stream calls set_physical_format(rate, channels, F32), a system-wide change
**Context**: .minerva/work/2026-10-14-mic-stream-outlives-recording (see git history if the worktree has been cleaned up)

## Context

Review of the cpal 0.15.3 → 0.18.2 upgrade compared what a stream build does to the device.

## Finding

Before building a CoreAudio input stream, cpal 0.18.2 calls
`set_physical_format(device, rate, channels, F32)` (`host/coreaudio/macos/device.rs`), and falls
back to setting only the sample rate when no matching F32 physical format exists. cpal 0.15.3 set
only the nominal rate. darric requests the device's current rate and channel count with an
`&[f32]` callback, so on a device that offers an F32 physical format, each build or rebuild can
change the hardware's bit depth for every app using that microphone, the meeting app included.
No problem has been observed from it.

## Implications

- If another app's audio on a shared mic glitches as darric starts or rebuilds a source, this is
  the first thing to check.

## Related

- [[2026-10-14-bug-a-dropped-cpal-0-15-input-stream-kept-capturing]] — the upgrade that introduced it
