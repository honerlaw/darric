# Scratchpad: transcript-accuracy

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06

- [reviewed — folded] scope check: one unit, two phases (silence-gate, utterance-segmentation), not decomposed (Skeptic accept; item 1 load-bearing — criterion 8 spanned both phases — split into 8a/8b; items 2–3 reworded in Why; item 4 stale, tree clean; item 6 becomes a ship-report note)
- [rechecked — clean] scope check: fold-audit confirmed all six items addressed; no new concerns
- [reviewed — folded] approach: A — standalone Silero VAD in transcribe + energy cut points + bundled model (Skeptic revise; folded 1 prompt carry-over bounded by 15 s recency / 30 words / cleared on silence, 3 noise floor updates only from non-speech frames, 4 integer timestamp→sample arithmetic; also folded 6 byte-compare + loader single-flight comment, 7/8/9 wording; dismissed 5 as re-weighting (beam retained, wording softened), 10 no action; rejected B duplicates VAD per device on its own thread, C leaves every other hallucination)
- [rechecked — escalated] approach: fold-audit found item 1 (prompt carry-over bound) only partially addressed — a bad line could chain through continuous speech; asked; user chose to drop prompt carry-over from the unit entirely (escalation 1 of 3)
- [reviewed — clean] whole-proposal soundness: Skeptic accept; noted and clarified in text: ignored accuracy test is run locally before each ship (not CI), phase-1 one-line join can merge two utterances in one 8 s window (accepted until phase 2), exact_u32_from_f64 becomes pub(crate), fixture is say+afconvert as in the prototype

## Work notes 2026-09-06 (phase 1)

- whisper.cpp applies VAD only in `whisper_full`; `whisper_full_with_state` (what whisper-rs
  `WhisperState::full` calls) ignores `params.vad`. Confirmed by reading whisper.cpp in
  whisper-rs-sys 0.15 and by a scratch run: with `enable_vad(true)`, 8 s of zeros still decoded
  to "Thank you.". So `transcription::vad::Gate` runs Silero itself and concatenates speech
  regions with 100 ms gaps, mirroring `whisper_full`'s own preprocessing.
- `WhisperSegment::no_speech_probability()` read 0.00 for every segment (speech and silence,
  greedy and beam) in this build. Not usable as a gate here; not investigated further.
- Beam 5 vs greedy on the 8.6 s fixture: 1.96 s vs 2.06 s — within noise on large-v3-turbo.
- `say` spawned from the cargo test harness hangs forever unless stdin is `Stdio::null()`;
  from an interactive shell it returns in a second. Fixture generation closes stdin.
- Contradicts [[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]]:
  `audio::hardware::taps_transcribe_only_the_device_that_played` tapped MacBook Pro Speakers
  from a plain `cargo test` binary and got audio. TCC grants the audio-capture permission to
  the responsible process — the terminal app that launched `tauri dev` and this test — so once
  the dev app has been granted it, test binaries under the same terminal hold it too. The
  entry's "never" was true only before that grant existed.
- Hardware test ran with the AirPods disconnected, so only one output device existed; the
  "other taps produce nothing" half was exercised earlier in the live session repro rather
  than in that run.
- The accuracy and hardware tests are `#[ignore]` (model + hardware); CI runs the bundled-VAD
  silence tests, the migration test and `should_give_up`.
- Completion Verifier (phase 1) marked criterion 2 unmet: "verified in the running app" was not
  literally done. Added `audio::hardware::a_microphone_hears_what_the_speakers_play` (production
  `run_source` + resampler on every input device while `afplay` plays the fixture): MacBook Pro
  Microphone → verbatim line. With the tap test, both halves of the capture path are now
  exercised on real devices; only the Tauri glue (`CaptureEngine`, `persist_and_emit`) is not,
  and it is unchanged by phase 1 apart from `Ok(None)` handling in the pool.
