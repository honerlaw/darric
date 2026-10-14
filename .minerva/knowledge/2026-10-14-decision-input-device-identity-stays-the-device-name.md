# Input device identity stays the device name, not cpal's stable id()

**Date**: 2026-10-14
**Type**: decision
**Theme**: audio-capture
**Summary**: Persisted disabled devices and transcript device_ids key on the name; description().name() reads the same property
**Context**: .minerva/work/2026-10-14-mic-stream-outlives-recording (see git history if the worktree has been cleaned up)

## Context

cpal 0.17 deprecated `Device::name()` in favour of `description()` (human-readable) and `id()`
(stable; the Core Audio UID on macOS). darric uses the input's name as its `CaptureDevice::id`, and
that string is persisted in the `capture.disabled_devices` setting and on every
`transcript_lines.device_id`.

## Finding

The identity stays the name, now read through one helper, `audio::device::device_name`
(`description().name()`). At source level, 0.15.3's `name()` and 0.18.2's description name
(via coreaudio-rs 0.14.2 `get_device_name`) both read `kAudioDevicePropertyDeviceNameCFString`,
Output scope, Main element. A before/after run of
`audio::device::tests::enumeration_yields_unique_stable_ids` printed "MacBook Pro Microphone"
identically on both versions. Switching to `id()` would have silently orphaned every device a
user had already disabled.

## Implications

- Moving to UIDs is possible but needs a migration of `capture.disabled_devices` and a decision
  about existing transcript lines. It is not a drop-in change.
- Two devices that share a name remain indistinguishable, as before.

## Related

- [[2026-10-14-bug-the-tap-exclusion-filter-compared-device-names-to-uids]] — where the UID _is_ the right key
