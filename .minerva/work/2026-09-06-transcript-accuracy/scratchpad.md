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
- [decided] ship + cleanup (phase 1): PR #49 merged (squash) after one CI lint fix; reconciliation owned by CI (knowledge-reconcile.yml, ran green, four entries catalogued on main); worktree teardown deferred — phase 2 outstanding; phase-2 branch cut from main
- [reviewed — clean] completion verification (phase 2): Verifier accept on criteria 5, 6, 7, 8b; re-ran the pipeline test itself; disclosed deviations (MIN 2 s, 64 taps) judged necessary, not gaming
- [decided] review triage (phase 2): 7 FIX / 0 SUGGEST / 0 IGNORE, none contested (solo gate); no load-bearing divergence
- [escalated to user] phase-2 ship precondition (criterion 2 confirmation): asked the user to record with nothing playing on the phase-1 build; answer "output lines still appear" — but the main checkout was two commits behind origin/main (before PR #49) and the dev app restarted at 14:10 was built from it; session 210a131d shows the old behaviour ("Thank you." from the speakers tap at 18:11:20 and 18:11:27, "_Tonk_" on the mic, per-sub-segment lines). Not a phase-1 defect; main fast-forwarded to origin/main; re-test on the real phase-1 build still pending (escalation 2 of 3)
- [decided] promote (final phase, Mode A, partial): six knowledge entries written (segmenter decision, recorded_at decision, clock-drift bug, resampler reference, CI clippy reference, say-bounded-child reference superseding the phase-1 say entry); `## Approach` rewritten to reality; `**Status**` and the scratchpad archive deferred until phase 2 actually ships, because the record must not say Shipped before it is; no TODOs cleared the deferral bar; no `**Closes**` (#16/#17 untouched)

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

## Work notes 2026-09-06 (phase 2)

- `MIN_SEGMENT` is 2 s, not the 3 s the Approach text said: success criterion 5's stream
  (2 s speech, 0.6 s pause) cannot produce two segments under a 3 s minimum, and the criterion
  is the contract. A remark shorter than the minimum waits until 2 s are buffered and then
  leaves on the next pause, so latency is bounded at ~2.4 s.
  → promoted to .minerva/knowledge/2026-09-06-decision-segments-end-at-pauses-found-by-an-energy-detector.md
- The noise floor also creeps up 0.2 %/frame during frames classified as speech (capped at the
  frame's own level). Without it, steady room noise louder than 4× the initial floor reads as
  speech forever and pauses are never seen; with it, such noise is reclassified within about
  half a minute while a ten-second utterance moves the floor by ~2 %.
- Leading silence is trimmed to one pause's worth, in whole frames, so a long silence neither
  fills the buffer nor mis-aligns the 20 ms frame grid. The first cut of the pause test came out
  one frame short until the trim was frame-aligned.
- Resampler: 64 taps, not 32. A 32-tap Blackman sinc has a ~8 kHz transition band at 48 kHz,
  which leaves a 12 kHz tone well above the 40 dB floor criterion 7 asks for; 64 taps measures
  under it. Cutoff 0.9 × the lower Nyquist. Every position stays integer; the ratio enters as
  gcd-reduced `u16`s.
  → promoted to .minerva/knowledge/2026-09-06-reference-a-windowed-sinc-resampler-needs-64-taps-for-40-db.md
- `recorded_at` is now the segment's capture start. `db::sessions::transcript_lines` already
  orders by it, so the on-screen transcript interleaves devices in speech order for free.
  → promoted to .minerva/knowledge/2026-09-06-decision-recorded-at-is-the-capture-time.md
- Pipeline test (segmenter → VAD → whisper on the `say` fixture spoken twice with silence
  around): the segmenter cut each pass at the natural pause between its two sentences, giving
  four whole-sentence lines stamped in order and a 410 ms tail that produced nothing.
- `say` hung once more even with stdin closed, while returning in a second from a shell. The
  fixture generator now polls the child and kills it after 30 s, retrying once, and the fixture
  is synthesized once per test process. The phase-1 knowledge entry's "stdin null fixes it" is
  necessary but not sufficient; the bound is the real protection.
  → promoted to .minerva/knowledge/2026-09-06-reference-say-under-cargo-test-needs-a-bounded-child-not-just-closed-stdin.md
- VAD `speech_pad_ms` raised from whisper.cpp's 30 to 100: one room-microphone run came back
  "Quarterly numbers…" without "The"; the next run at 30 ms was verbatim, so the loss is
  acoustic variance, and the extra 140 ms per region is cheap insurance.

## Review triage 2026-09-06 (phase 2)

Mode: local-diff (fresh-context subagent; no PR yet). Findings from the code review; the minerva audit found no spec or knowledge violation beyond the two disclosed deviations already in the phase-2 work notes.

- [FIXED] #1 high transcription/vad.rs — the 100 ms VAD padding described in the notes was never in the commit (a patch that reported success had not written the file) → `vad_params()` with `SPEECH_PAD_MS = 100`.
- [FIXED] #2 high audio/segmenter.rs — `started_at` anchored only on an empty buffer, so after a delivery gap every stamp was extrapolated from the session's first callback → `anchor()` re-anchors when a chunk arrives more than 100 ms later than the buffered audio accounts for; test `a_delivery_gap_re_anchors_the_clock`.
- [FIXED] #3 medium audio/segmenter.rs — no test of the stamp after leading-silence trimming with a continuous clock → `feed_from` helper and `speech_after_a_long_silence_is_stamped_when_it_was_heard`.
- [FIXED] #4 low audio/segmenter.rs — misleading "cut where the pause began" comment and an unreachable `cut == 0` guard → comment rewritten, guard removed.
- [FIXED] #5 low audio/segmenter.rs — `take` re-classified the retained pause with floor updates, compounding the rise → `classify_new_frames(update_floor)`; the re-derivation passes `false`.
- [FIXED] #6 low README.md — "a linear resampler used to fold…" read as present tense → "the previous linear resampler folded…".
- [FIXED] #7 low db/sessions.rs + src/hooks/useTranscript.ts — stale comment about why rowid and timestamp order diverge; and the live view appended lines in arrival order while a reload sorts by `recorded_at`, so a reopened session could reorder what was shown live → comment rewritten; live lines are inserted by `recorded_at` (`insertByRecordedAt`, test "places a line by its capture time, not its arrival").

Review fixes: vad.rs — speech pad; segmenter.rs — anchor(), floor re-derivation, comments, two tests; sessions.rs — comment; README; useTranscript.ts + test.

## Criterion-2 confirmation status

- 2026-09-06: attempted against a stale build (main was at 44f14ba, before #49); result invalid.
  Main fast-forwarded to edeedb3. Awaiting a re-test on the phase-1 build: restart
  `npm run tauri:dev`, record with nothing playing, confirm no `output` lines. Phase 2 ships
  only after that is recorded here (or in `archive/scratchpad.md` once promoted).
