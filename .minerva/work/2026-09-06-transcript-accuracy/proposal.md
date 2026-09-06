# Proposal: transcript-accuracy

**Date**: 2026-09-06
**Status**: Draft

## Goal

A darric recording no longer invents words for silence, cuts sentences at arbitrary
eight-second boundaries, or shows a vanished device as "retrying" forever. Every transcript
line comes from audio that a voice-activity detector classified as speech, segments end at
pauses in speech, the 16 kHz feed whisper hears is band-limited before it is decimated, and a
capture source that cannot be rebuilt for a minute is marked failed and left alone.

## Why

Debugging session `f46c9a22` on 2026-09-06 (confirmed by a local repro against the real
large-v3-turbo model) found:

1. **Silence is transcribed.** Both output taps carried digital silence — the MacBook speakers
   are not the active output, and nothing was playing through the AirPods — and every eight
   seconds each produced the line "Thank you." Eight seconds of zeros through large-v3-turbo
   decodes to exactly that with mean token probability 0.75; whisper.cpp's own
   `no_speech_thold` / `logprob_thold` / `entropy_thold` do not suppress it. The app's only
   silence handling (`transcription/mod.rs:29-33`) is a log line. The stop-time flush sends a
   sub-two-second tail through the same path and both microphones emitted "Thank you." at stop.
2. **Fixed eight-second windows chop utterances** (`audio/segmenter.rs`), so lines like "Even
   with the big ox, I won't let me get" / "all the way back." are split mid-phrase, each half
   decoded with zero context.
3. **The linear resampler has no anti-alias filter** (`audio/resample.rs:29-56`): everything
   above 8 kHz in a 48 kHz capture folds down into the speech band before whisper sees it.
4. **A Continuity iPhone microphone that disappeared after enumeration retries forever**
   (`audio/source.rs:154-160`): the supervisor rebuilds by name, backs off to ten seconds, and
   never leaves `Retrying`. The UI shows a pulsing red "retrying" for the whole session.
5. **The stripped AI feature left an API key in `settings`** (`ai.claude.api_key`). Nothing reads
   it; it is a secret at rest for no reason. This is not accuracy work — it is included in
   phase 1 opportunistically because it is one migration plus one test and came out of the same
   investigation.
6. **`recorded_at` is the transcription time, not the capture time**, so across devices the
   order of lines is the order whisper finished them, and the MCP `get_transcript` doc has to
   apologise for it. An ordering fix rather than an audio one; it sits in phase 2 because phase
   2 already reshapes `SegmentJob` and `TranscribedLine`.

The user's stated priority is accuracy over speed.

## Approach

Phase 1 fixes the reported symptoms (1, 4, 5); phase 2 is the accuracy work (2, 3, 6).

Feeding each device's previous line to whisper as an `initial_prompt` was designed and then
dropped at the user's decision: a garbled line would prime every following segment during
continuous speech, whisper's known repetition failure, and the pause-based segments already
hand whisper whole utterances. Each segment is decoded without cross-segment context.

### whisper-rs 0.14 → 0.16

`whisper-rs = "0.16"` (sys 0.15, a whisper.cpp with VAD support). The state API changed:
`full_n_segments()` returns a count, segments come from `state.get_segment(i)` as a
`WhisperSegment` with `to_str_lossy()`, `no_speech_probability()`, and per-token probabilities.
`Transcriber::transcribe` is rewritten against that API; nothing else in the crate touches
whisper-rs.

### Silero VAD, bundled

`src-tauri/models/ggml-silero-v5.1.2.bin` (885 KB, MIT) is committed and `include_bytes!`'d.
On `Transcriber::new` the file in the model directory (`model::model_dir()`, extracted from
`default_model_path`) is read and compared byte-for-byte with the embedded copy; if absent or
different it is rewritten through a `.tmp` + rename, and the path is kept on the `Transcriber`.
A full comparison rather than a length check because `exists()`-style validity is exactly what
[[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] warns about, and 885 KB is
cheap to read. `Transcriber::new` is reached only through `loader::get_or_load`'s single flight
(the other caller is the ignored bench), so two writers cannot race; a comment on
`Transcriber::new` states that dependency. No download: huggingface.co is blocked on the user's
work network. A sibling session is moving the whisper model itself to a GitHub Release asset
for the same reason; riding that path was considered and rejected for a sub-megabyte file that
can simply live in the repo, and the two are not the same mechanism by design — the whisper
model is 1.6 GB and must be fetched, the VAD model is 885 KB and need not be.

### Transcription gate

whisper.cpp applies its integrated VAD only in `whisper_full`, the entry point that uses the
context's own state; `whisper_full_with_state`, which is what whisper-rs's `WhisperState::full`
calls, ignores `params.vad` entirely. Prototyped on 2026-09-06: with `enable_vad(true)` set,
8 s of zeros still decoded to "Thank you." So the gate runs the VAD itself.

`Transcriber` holds a `Mutex<WhisperVadContext>` (Silero runs on the CPU and takes ~20 ms for
8 s of audio, so the lock is uncontended in practice). `transcribe` does, in order:

1. Drop inputs shorter than 0.5 s (the flush floor).
2. `segments_from_samples` with `WhisperVadParams` at whisper.cpp's defaults (threshold 0.5,
   min speech 250 ms, min silence 100 ms, pad 30 ms). Zero segments → return no lines; the
   encoder never runs, which is why a silent tap now costs ~20 ms per segment instead of ~2 s.
3. Build the filtered buffer the way `whisper_full` does: the detected speech regions
   concatenated with 100 ms of silence between them.
4. `state.full` on that buffer with `SamplingStrategy::BeamSearch { beam_size: 5, patience: -1.0 }`
   and `set_language(Some("en"))`. On large-v3-turbo the decoder is four layers, so beam search
   adds little over the encoder; one prototype run measured 1.96 s beam vs 2.06 s greedy on an
   8.6 s sample, which is within noise and says only that beam is not expensive here. It is the
   accuracy-over-speed choice the user asked for; the phase-1 ship report watches the dropped
   segment count.
5. Join whisper's sub-segments into **one transcript line per audio segment**, space-separated.
   Today each sub-segment is its own row, and beam search tends to split mid-sentence
   ("…second week" / "of October…"); one line per segment reads as one utterance, which is also
   what phase 2's pause-based segments will be. Until phase 2 lands, a fixed 8 s window that
   happens to hold two utterances yields them on one line; accepted for the phase-1 window.

`WhisperSegment::no_speech_probability()` was evaluated as a second gate and not used: in the
prototype it read 0.00 for every segment, speech or silence, under both greedy and beam
decoding, so whatever it measures in this build it did not separate the two. The VAD is the
gate. What the VAD gates is silence and non-speech noise; music, television or another
person's call reaching an output tap is speech-like and still reaches whisper. That is the
same audio a human would transcribe, and it is outside this unit.

The concatenated buffer discards the real timing between speech regions. Nothing downstream
reads sub-segment timestamps — `recorded_at` is the segment's capture time — so this is a
known simplification, recorded here so a future phrase-level timing feature does not read
positions out of this buffer.

Sample offsets from the VAD's centisecond timestamps stay in integers: whisper.cpp derives
them from integer sample positions, so `start`/`end` are converted with the existing
`coreaudio::exact_u32_from_f64` pattern (round, then a checked conversion; the helper is
private to that module and is moved to `pub(crate)` rather than duplicated) and multiplied by
160 in `u64`; no `as` casts on floats, per the lint policy.

Prototype results (scratch crate on whisper-rs 0.16, real large-v3-turbo, Metal):

| Input                                             | VAD kept        | Result         |
| ------------------------------------------------- | --------------- | -------------- |
| 8 s digital zero                                  | 0 of 8.00 s     | skipped, 19 ms |
| 8 s noise RMS 0.003                               | 0 of 8.00 s     | skipped, 26 ms |
| 1.5 s noise tail                                  | 0 of 1.50 s     | skipped, 5 ms  |
| 8.6 s speech synthesized with `say` + `afconvert` | 8.56 s          | verbatim       |
| same, 3 s silence each side                       | 8.58 of 14.62 s | verbatim       |
| same plus noise RMS 0.01                          | 8.43 s          | verbatim       |

`transcription::bench` grows an `#[ignore]` accuracy test that runs those six cases through
`Transcriber::transcribe` and asserts the three silent ones yield zero lines and the three
speech ones contain "second week of October" (case-insensitive). It needs the downloaded
whisper model, like the existing pool benchmark, so CI does not run it; the implementer runs
it locally before each phase ships and quotes its output in the ship report. It is the evidence
for success criterion 1.
The speech fixture is generated at test time with macOS `say` + `afconvert` (both ship with
macOS, which is the only platform darric builds for) into a temp dir, so no audio file is
committed.

### Capture give-up

`source::run_source` records when the current failure streak began. Once a streak exceeds
`GIVE_UP_AFTER = 60 s` the supervisor logs "gave up", sets `Failed`, and exits its loop; a
successful rebuild resets the streak. The decision is a pure function
(`should_give_up(streak_started: Instant, now: Instant) -> bool`) with a unit test. The UI's
`failed` title becomes "Unavailable — stopped retrying" and the row shows a static "failed"
label the way it shows "retrying" today, with a `DeviceRow` test.

### Settings hygiene

Migration `011_drop_ai_settings.sql`: `DELETE FROM settings WHERE key LIKE 'ai.%';`, added to
`db/migrations.rs`. A test through `db::test_db()` inserts an `ai.claude.api_key` row before
the migration list is applied — not possible with `to_latest`, so the test applies migrations
up to 010, inserts, then applies 011 and asserts the row is gone.

### Utterance segmentation (phase 2)

`Segmenter` cuts at pauses instead of at a fixed sample count. Per 20 ms frame it computes RMS
and classifies the frame against a noise floor: speech when RMS exceeds
`max(4 × floor, 0.004)`. The floor updates **only from frames classified non-speech**: it drops
immediately to a quieter non-speech frame and rises toward a louder one by at most 2 % per
frame, so a loud utterance cannot drag it up and a room that gets noisier is tracked within a
few seconds. It starts at 0.001. A segment is emitted when the buffer holds at least
`MIN_SEGMENT = 3 s` and the last `PAUSE = 400 ms` of frames are all non-speech, or when it
reaches `MAX_SEGMENT = 25 s` regardless. The buffer keeps the trailing pause so the next segment
starts clean. This runs on the audio callback thread and is a handful of multiplies per frame.
The Silero gate in `transcribe` still owns the speech/non-speech decision inside a segment;
the energy detector only chooses cut points, so a noisy room degrades to today's behaviour (cut at
the cap) rather than to silence being transcribed.

Each emitted segment carries `captured_at`, the wall-clock time its first sample arrived.
`SegmentJob` and `TranscribedLine` carry it through the pool and `persist_and_emit` writes it
as `recorded_at`. The MCP `get_transcript` description and README drop the "transcription
order" caveat in favour of "lines are timestamped at capture; sort by `recorded_at`".

### Band-limited resampler (phase 2)

`resample::Resampler` replaces `resample_mono`: a stateful windowed-sinc interpolator (32 taps,
Blackman window, cutoff at 0.45 × the lower of the two rates) that carries its tail between
calls so buffer boundaries do not click. The read position is carried as the same fixed-point
`u64` numerator over `TARGET_RATE` that `resample_mono` uses today, so the fractional phase
converts to `f32` exactly and no float is cast to an index. One instance per capture source, created in
`build_stream` / `start_io_proc`. Tests: a 1 kHz tone at 48 kHz comes out at unity within 1 dB;
a 12 kHz tone comes out at least 40 dB down; a 16 kHz input passes through unchanged; output
length across many small pushes equals the length of one large push.

### Candidate approaches considered

- **A (chosen): Silero VAD run in `transcribe` before whisper + energy-based cut points in the
  segmenter + bundled model.** Silence never reaches the encoder; the cheap detector only picks
  boundaries. (whisper.cpp's integrated VAD was the first draft of A and is unreachable through
  whisper-rs's state API — see Transcription gate.)
- **B: run Silero ourselves per device on a dedicated thread, cut on its boundaries.** Best
  boundaries, but one more thread and channel per device, and whisper-rs's VAD context is a
  whole-buffer call with no streaming state, so it would be re-run on a sliding window anyway.
- **C: RMS threshold + a "Thank you." phrase filter, no upgrade.** A day's work, but it treats
  one hallucination string and leaves every other one, and the chopping is untouched.

### Rejected

- Downloading the VAD model from Hugging Face: blocked network, and a second copy of the
  download path.
- `rubato` for resampling: its 5.0 API is adapter-based and unfamiliar; a 32-tap sinc is
  eighty lines with tests and no dependency.
- Disabling duplicate microphones automatically: two microphones hearing the same room is a
  device-selection question the existing per-device toggle already answers.

## Success criteria

1. The `#[ignore]` accuracy test passes against the real large-v3-turbo model: the three silence
   and tail cases produce zero transcript lines and the three speech cases contain the expected
   phrase.
2. Through the production capture code against real devices — the ignored hardware tests
   `taps_transcribe_only_the_device_that_played` and `a_microphone_hears_what_the_speakers_play`
   — an output tap that carried speech produces it verbatim, and a microphone that heard the
   speech produces its line. Digital silence produces no line (the accuracy test and the CI-run
   VAD test); that an idle tap delivers digital silence is inferred from the debugging session,
   not observed by the hardware run, which had one output device. The phase-1 ship report asks
   the user to make one recording in the app with nothing playing and confirm that no `output`
   lines appear; phase 2 does not ship until that confirmation is recorded in the scratchpad,
   and a failed confirmation is a phase-1 defect to fix before phase 2. (Reworded by the
   2026-09-06 replan; originally "verified in the running app".)
3. Sixty seconds after a capture device disappears its row reads "failed", not "retrying", and
   the supervisor thread has exited; `should_give_up` has a unit test.
4. After migration 011 no `settings` row has a key beginning `ai.`; a Rust test proves it.
5. Segments end at pauses: a synthetic stream of 2 s speech-level noise, 0.6 s silence, 2 s
   noise, 0.6 s silence yields two segments, and 30 s of continuous noise yields a segment at
   25 s (unit tests on `Segmenter`).
6. `recorded_at` on a persisted line is the segment's capture-start time, not the transcription
   time (unit test on the job/line plumbing).
7. Resampler tests: unity gain at 1 kHz, ≥ 40 dB attenuation at 12 kHz for 48 → 16 kHz,
   pass-through at 16 kHz, and push-size independence.
8. (a) After phase 1: `npm run check` and `cargo test --manifest-path src-tauri/Cargo.toml`
   pass with no new lint suppressions; the README documents the bundled VAD model, one line per
   audio segment, and the give-up rule. (b) After phase 2: the same checks pass, and the README
   and the MCP `get_transcript` description document capture-time `recorded_at`, pause-based
   segmentation, and the band-limited resampler.

## Phases

1. **silence-gate** — whisper-rs 0.16, bundled Silero VAD, transcription gate, one line per
   segment, flush floor, capture give-up, migration 011, README. Criteria 1, 2, 3, 4, 8a.
2. **utterance-segmentation** — pause-based segmenter, capture-time `recorded_at`,
   windowed-sinc resampler, MCP/README wording. Criteria 5, 6, 7, 8b. Precondition: the
   criterion-2 user confirmation from phase 1 is recorded in the scratchpad.

## Open Questions

- Beam size 5 measured as roughly cost-neutral on large-v3-turbo for one sample, which is thin. If a five-device
  session shows dropped segments in `status` after phase 1, the constant is the knob; the
  phase-1 ship report says to watch that count until phase 2 lands.
