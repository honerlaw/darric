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

What shipped, in two PRs. Phase 1 (#49, merged 2026-09-06) is the silence gate and the capture
give-up; phase 2 is the segmentation, timestamps and resampler.

### Transcription gate (phase 1)

whisper-rs 0.14 → 0.16. whisper.cpp's integrated VAD only runs in `whisper_full`, which
whisper-rs never calls, so `transcription::vad::Gate` runs the Silero VAD itself: inputs
under 0.5 s are dropped, `segments_from_samples` (whisper.cpp defaults, speech padding 100 ms)
finds the speech regions, and a segment with none returns no line without touching the
encoder; otherwise the regions are concatenated with 100 ms gaps, as `whisper_full` does, and
decoded with `BeamSearch { beam_size: 5 }`. Whisper's sub-segments are joined into one line per
audio segment. `no_speech_probability()` read 0.00 for everything in this build and is not
used. See [[2026-09-06-constraint-whisper-rs-state-api-never-applies-whisper-cpp-vad]] and
[[2026-09-06-bug-whisper-transcribes-silence-as-thank-you]].

The 885 KB Silero model is committed at `src-tauri/models/ggml-silero-v5.1.2.bin`, embedded
with `include_bytes!`, and written to `model::model_dir()` on first use, compared byte-for-byte
and written through a `.tmp` + rename under a process-wide lock. No download.

Prompt carry-over between segments was designed and dropped at the user's decision: a garbled
line would prime every following segment during continuous speech.

### Capture give-up (phase 1)

`source::supervise` is the rebuild loop with the stream builder injected, so it is tested in
CI in milliseconds. A failure streak longer than `GIVE_UP_AFTER` (60 s) ends the loop with
`Failed`; a rebuilt stream must stay up `STABLE_AFTER` (5 s) before the streak is forgotten, so
a device that builds and dies at once is bounded too. The device row shows `failed` with a
per-direction title (output taps are tried once, never rebuilt).

### Settings hygiene (phase 1)

Migration 011 deletes every `ai.%` settings row, with a test that seeds a key at schema 010.

### Utterance segmentation (phase 2)

`Segmenter` cuts at pauses: per-20 ms-frame RMS against an adaptive floor, speech when RMS >
max(4 × floor, 0.004); a cut after 400 ms of non-speech once 2 s are buffered, or at 25 s. The
floor drops at once on a quieter non-speech frame, rises ≤ 2 %/frame on a louder one, and
creeps ≤ 0.2 %/frame during speech. Leading silence is trimmed to one pause in whole frames;
the pause that ended a segment stays to open the next. The minimum is 2 s, not the 3 s first
written here, because criterion 5's stream cannot yield two segments under 3 s. The clock is
re-anchored whenever a chunk arrives more than 100 ms later than the buffered audio accounts
for, so a rebuilt stream does not leave every later stamp early. See
[[2026-09-06-decision-segments-end-at-pauses-found-by-an-energy-detector]] and
[[2026-09-06-bug-capture-stamps-drifted-after-a-delivery-gap]].

Each segment carries `captured_at`; `SegmentJob`, `TranscribedLine` and `audio::recorded_at`
carry it into the insert and the `transcript_chunk` event. The live view inserts lines by
`recorded_at`, matching the reload order. The MCP `get_transcript` description and README say to
sort by it. See [[2026-09-06-decision-recorded-at-is-the-capture-time]].

### Band-limited resampler (phase 2)

`resample::Resampler`: a 64-tap Blackman-windowed sinc (32 was proposed; it does not reach the
40 dB criterion), cutoff at 0.9 × the lower Nyquist, one stateful instance per stream in
`build_stream` and behind a mutex in the tap's IOProc block. Positions are fixed-point `u64`,
the ratio enters as gcd-reduced `u16`s, weights are DC-normalised. About 1 % of real time per
stereo 48 kHz stream in release. See
[[2026-09-06-reference-a-windowed-sinc-resampler-needs-64-taps-for-40-db]].

### Tests

CI runs the bundled-VAD silence test, the migration test, the supervisor loop tests, the
segmenter and resampler tests, the timestamp plumbing test, and the frontend ordering test.
Ignored, run locally before each ship: the model-level accuracy test (six silent inputs → no
line; synthesized speech plain, silence-padded and with noise → verbatim), the end-to-end
pipeline test (segmenter → VAD → whisper on the fixture spoken twice → whole-sentence lines in
order), the two real-device hardware tests (taps; microphones), and the 75 s give-up test. The
fixture is synthesized with `say` + `afconvert` under a bounded child
([[2026-09-06-reference-say-under-cargo-test-needs-a-bounded-child-not-just-closed-stdin]]).

### Rejected

- whisper.cpp's integrated VAD through `FullParams` — unreachable via whisper-rs.
- Downloading the VAD model — huggingface.co is blocked on some networks; 885 KB needs no
  network.
- Running Silero per device on its own thread for cut points — one more thread and channel per
  device for boundaries the energy detector already finds well enough.
- `rubato` for resampling — an unfamiliar adapter API for eighty lines of sinc.
- An RMS threshold or a "Thank you." phrase filter — noise at RMS 0.003 still produced text.
- Auto-disabling duplicate microphones — a device-selection question the per-device toggle
  answers.

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
