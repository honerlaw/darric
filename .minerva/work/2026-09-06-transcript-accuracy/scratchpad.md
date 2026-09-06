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
- [reviewed — folded] completion verification (phase 1): Verifier revise — criteria 1, 3, 4, 8a reproduced and met; criterion 2 unmet as worded ("verified in the running app" not literally done, mic path not on hardware) → added a real-microphone hardware test (passed) and triggered Phase 2.5 replan to reword criterion 2
- [reviewed — folded] new-plan acceptance (replan, criterion 2): Skeptic accept; folded 1 (idle-tap-is-silence stated as inference, not fact) and 2 (user confirmation gates phase-2 ship and is recorded, not merely asked); 3 subsumed by 1; 4–6 confirmatory
- [rechecked — clean] new-plan acceptance (replan, criterion 2): fold-audit confirmed items 1–3 addressed; its one low note (phase-2 precondition only in prose) folded as a precondition line on phase 2 in `## Phases`
- [reviewed — clean] completion verification (phase 1, second pass): Verifier accept on criteria 1, 2 (as reworded), 3, 4, 8a; no regressions from the docs commits
- [decided] review triage (phase 1): 7 FIX / 1 SUGGEST (already promoted) / 3 IGNORE, none contested (solo gate); no load-bearing divergence, no replan-vs-FIX
- [decided] promote (phase 1, Mode B): four knowledge entries — VAD constraint, silence bug, tap-permission reference (contradicts the 2026-09-05 entry), say/stdin reference; Mode A deferred to the final phase; no TODOs cleared the deferral bar

## Work notes 2026-09-06 (phase 1)

- whisper.cpp applies VAD only in `whisper_full`; `whisper_full_with_state` (what whisper-rs
  `WhisperState::full` calls) ignores `params.vad`. Confirmed by reading whisper.cpp in
  whisper-rs-sys 0.15 and by a scratch run: with `enable_vad(true)`, 8 s of zeros still decoded
  to "Thank you.". So `transcription::vad::Gate` runs Silero itself and concatenates speech
  regions with 100 ms gaps, mirroring `whisper_full`'s own preprocessing.
  → promoted to .minerva/knowledge/2026-09-06-constraint-whisper-rs-state-api-never-applies-whisper-cpp-vad.md
- `WhisperSegment::no_speech_probability()` read 0.00 for every segment (speech and silence,
  greedy and beam) in this build. Not usable as a gate here; not investigated further.
- Beam 5 vs greedy on the 8.6 s fixture: 1.96 s vs 2.06 s — within noise on large-v3-turbo.
- `say` spawned from the cargo test harness hangs forever unless stdin is `Stdio::null()`;
  from an interactive shell it returns in a second. Fixture generation closes stdin.
  → promoted to .minerva/knowledge/2026-09-06-reference-say-hangs-under-cargo-test-unless-stdin-is-closed.md
- Contradicts [[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]]:
  `audio::hardware::taps_transcribe_only_the_device_that_played` tapped MacBook Pro Speakers
  from a plain `cargo test` binary and got audio. TCC grants the audio-capture permission to
  the responsible process — the terminal app that launched `tauri dev` and this test — so once
  the dev app has been granted it, test binaries under the same terminal hold it too. The
  entry's "never" was true only before that grant existed.
  → promoted to .minerva/knowledge/2026-09-06-reference-a-test-binary-holds-the-audio-permission-of-its-terminal.md
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

## Review triage 2026-09-06 (phase 1)

Mode: local-diff (fresh-context subagent; no PR yet). Findings 1–4 minerva audit, 5–11 code review.

- [IGNORED] #1 low proposal.md — accuracy test named as `transcription::bench`; lives in `transcription::accuracy`. Promote rewrites Approach.
- [SUGGESTED] #2 medium knowledge/2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission — stale "never"; test binaries inherit the terminal's grant. → promoted to .minerva/knowledge/2026-09-06-reference-a-test-binary-holds-the-audio-permission-of-its-terminal.md
- [IGNORED] #3 low knowledge/2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu — beam adds decoder work; watch item already in Open Questions.
- [IGNORED] #4 low transcription/mod.rs — `transcribe` returns `Option<String>`, `TranscriptSegment` removed; Approach predates it. Promote rewrites.
- [FIXED] #5 medium src/components/DeviceRow.tsx — "stopped retrying after a minute" shown for output taps that never retry → per-direction title, test added; README says taps are tried once.
- [FIXED] #6 medium src-tauri/src/audio/source.rs — give-up wiring only in a 75 s ignored test → loop extracted as `supervise` over an injected build; four CI tests (never builds / dies at once / recovers / shutdown) in under a second.
- [FIXED] #7 low src-tauri/src/audio/source.rs — a stream that builds then dies at once reset the streak → `STABLE_AFTER` 5 s: only a stream that stayed up ends the streak.
- [FIXED] #8 low src-tauri/src/transcription/vad.rs — parallel ignored tests raced the `.tmp` write → `WRITE_LOCK` with a re-check; the loader comment no longer carries the safety argument.
- [FIXED] #9 low src-tauri/src/lib.rs — whisper.cpp printed ~10 lines per VAD call to stderr → `log_backend` feature + `install_logging_hooks()`; off unless RUST_LOG names `whisper_rs`.
- [FIXED] #10 low transcription/vad.rs + mod.rs — duplicated `noise()` and silent-case list → `fixture::silent_cases()` shared by the CI VAD test and the ignored accuracy test.
- [FIXED] #11 low transcription/mod.rs — join and floor logic only reachable through the model → `join_pieces` / `too_short` extracted, three unit tests.

Review fixes: source.rs — `supervise` refactor, `STABLE_AFTER`; vad.rs — `WRITE_LOCK`; mod.rs — `join_pieces`, `too_short`, `fixture::silent_cases`; lib.rs — whisper log hooks; DeviceRow.tsx — `stateTitle`; README — taps tried once.
