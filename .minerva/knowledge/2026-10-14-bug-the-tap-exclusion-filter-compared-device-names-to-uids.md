# The tap-exclusion filter compared device names to UIDs, so a recording listed its own tap as a mic

**Date**: 2026-10-14
**Type**: bug
**Theme**: audio-capture
**Summary**: ExclusionRegistry holds aggregate UIDs but enumeration compared names; now compares cpal's device id()
**Context**: .minerva/work/2026-10-14-mic-stream-outlives-recording (see git history if the worktree has been cleaned up)

## Context

Each output tap creates a private aggregate device and registers its UID (`darric-tap-<uuid>`) in
`ExclusionRegistry`, so that `list_input_devices` does not offer the app's own tap as a
microphone. The aggregate is private, but it is still visible to this process's own enumeration.

## Finding

`list_input_devices` checked `excluded.contains(&name)`. The aggregate's name is
`Darric Tap — <device>`, never its UID, so the filter never matched. While a tap existed, the
input list contained `"Darric Tap — MacBook Pro Speakers"`. The ignored test
`audio::device::tests::a_registered_tap_is_not_listed_as_a_microphone` reproduced this. cpal
0.15 exposed no device UID. 0.18's `device.id()` returns `kAudioDevicePropertyDeviceUID`, and the
filter now compares that, so the test passes.

## Implications

- The registry and the filter must agree on what they compare: UIDs. A device's name is darric's
  _identity_ for inputs, but its UID is what excludes it.

## Related

- [[2026-10-14-decision-input-device-identity-stays-the-device-name]] — why identity is the name while exclusion is the UID
