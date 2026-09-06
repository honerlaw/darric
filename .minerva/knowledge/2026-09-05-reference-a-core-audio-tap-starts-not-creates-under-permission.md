# A Core Audio tap is refused at start, not at creation — and `cargo test` can never hold the permission

**Date**: 2026-09-05
**Type**: reference
**Summary**: `AudioHardwareCreateProcessTap` succeeds without the audio-recording permission and only `AudioDeviceStart` fails, so error handling written around creation reports success on a tap that can never deliver audio
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Output capture builds three objects per device: a process tap, a private aggregate device whose
tap list contains it, and an IOProc on that aggregate. Running this against the real built-in
speakers, without the audio-recording permission granted:

```
tapping "MacBook Pro Speakers" (BuiltInSpeakerDevice)
!! TAP DID NOT START: AudioDeviceStart failed: OSStatus 268451843 (0x10004003)
```

## Finding

**The permission is enforced at `AudioDeviceStart`, not at tap creation.**
`AudioHardwareCreateProcessTap` returned success. `AudioHardwareCreateAggregateDevice` returned
success. `AudioDeviceCreateIOProcIDWithBlock` returned success. Only the final start was refused.

That ordering is counterintuitive and it matters: code that validates the tap by checking whether
creation succeeded will conclude everything is fine and then silently record nothing. The check
has to be on the start call.

It is also useful diagnostically — reaching `AudioDeviceStart` proves the construction sequence,
the toll-free-bridged `CFDictionary`, the tap-list structure and the UUID plumbing are all
well-formed enough for Core Audio to accept, which narrows a failure a long way.

**A `cargo test` binary can never hold this permission.** macOS TCC grants against a bundle
identifier and shows the string from the bundle's `Info.plist`. A bare test binary has neither, so
there is nothing to prompt for and nothing to grant. The `NSAudioCaptureUsageDescription` this
app declares lives in the _app bundle's_ `Info.plist`.

## Implications

- Output capture cannot be runtime-verified by any test in this repo. It requires launching the
  built app and granting the permission — an automated suite can verify enumeration, construction
  and teardown, and nothing beyond that.
- Do not treat a green `cargo test` as evidence that tapping works. It is evidence that tapping
  _sets up_.
- The tap uses the audio-recording permission, not screen recording: `NSAudioCaptureUsageDescription`
  in `Info.plist`, and `com.apple.security.device.audio-input` in the entitlements. The
  `com.apple.security.screen-capture` entitlement that used to be declared belonged to a
  ScreenCaptureKit path that was never implemented.
- `OSStatus 268451843` (`0x10004003`) is not a four-character code and does not appear in
  `AudioHardwareBase.h`; treat it as "the tap is not permitted to run".

## Related

- [[2026-09-05-bug-forgetting-a-block-leaked-it-and-masked-a-use-after-free]] — the other phase-3 finding, also invisible to the test suite
- [[2026-09-06-reference-a-test-binary-holds-the-audio-permission-of-its-terminal]] — contradicts
