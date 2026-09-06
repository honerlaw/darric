# Replan: transcript-accuracy

## Replan 2026-09-06 — criterion 2 cannot be verified in the app UI autonomously

**Original plan.** Success criterion 2 required confirming, in the running Tauri app, that
output devices carrying nothing produce no `output` lines and that a spoken sentence on a
microphone still produces its line.

**What changed.** The completion Verifier held the criterion to its literal wording and found it
unmet: nothing in this run can drive the Tauri UI (press Record, speak, press Stop), and no
UI-automation harness exists for the app. What could be done was done against real devices
through the production capture code, as two ignored hardware tests in `src-tauri/src/audio/mod.rs`:

- `hardware::taps_transcribe_only_the_device_that_played` — every real output device is tapped
  with the production `OutputTap`, the spoken fixture is played through the default output with
  `afplay`, and each tap's capture is transcribed through the production `Transcriber`. Result:
  the device that played it produced the verbatim sentence and the assertion "exactly one device
  carried the speech" passed. Only one output device was present in that run (the AirPods had
  disconnected), so the run did not observe what a second, idle tap delivers.
- `hardware::a_microphone_hears_what_the_speakers_play` — every real input device is captured
  through the production `source::run_source` (cpal stream, `to_16k_mono` resampler) while the
  fixture plays through the speakers. Result: the MacBook Pro Microphone produced the verbatim
  sentence.

That an idle tap delivers digital silence is an inference from the debugging session that
started this unit (both idle taps produced exactly the line digital zeros produce), not
something the hardware run observed. Digital silence producing no line is covered by the
accuracy test and by the CI-run `vad::tests::silence_and_noise_are_not_speech`.

The code between those tests and the app is `CaptureEngine::start` / `persist_and_emit` in
`audio/mod.rs` and the pool's worker loop; phase 1 changed only the worker loop, to emit no line
on `Ok(None)`.

**New plan.** Criterion 2 becomes:

> Through the production capture code against real devices — the ignored hardware tests
> `taps_transcribe_only_the_device_that_played` and `a_microphone_hears_what_the_speakers_play`
> — an output tap that carried speech produces it verbatim, and a microphone that heard the
> speech produces its line. Digital silence produces no line (the accuracy test and the CI-run
> VAD test); that an idle tap delivers digital silence is inferred from the debugging session,
> not observed by the hardware run, which had one output device. The phase-1 ship report asks
> the user to make one recording in the app with nothing playing and confirm that no `output`
> lines appear; phase 2 does not ship until that confirmation is recorded in the scratchpad,
> and a failed confirmation is a phase-1 defect to fix before phase 2.

No other criterion changes. Approach unchanged.
