# Whisper transcribes silence as "Thank you.", so every idle output tap said it every segment

**Date**: 2026-09-06
**Type**: bug
**Summary**: 8 s of digital zeros through large-v3-turbo decodes to "Thank you." at token probability 0.75; fixed by a Silero VAD gate in front of whisper, not by whisper.cpp's thresholds
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

A live recording on 2026-09-06 captured both output devices and three microphones. The MacBook
speakers were not the active output and nothing was playing through the AirPods, yet both output
taps produced the transcript line "Thank you." every eight seconds for the whole session, and
both microphones produced it once more the instant the recording stopped.

## Finding

The taps delivered digital silence, and whisper hallucinates on silence. Reproduced locally
against the real model: eight seconds of zeros → "Thank you." with mean token probability 0.75;
noise at RMS 0.0005–0.01 → "."; a 1.5 s noise tail → "Thank you." again. whisper.cpp's own
`no_speech_thold`, `logprob_thold` and `entropy_thold` left all of it unchanged, and the app's
only silence handling in `Transcriber::transcribe` was a log line. The stop-time flush sent every
device's sub-two-second tail down the same path, which is the second symptom.

The fix is `transcription::vad::Gate`: the bundled Silero VAD classifies the segment, a segment
with no speech produces no line without touching the encoder, and the speech regions of one that
has some are concatenated and decoded. Inputs under 0.5 s are dropped outright. After the fix the
same six silent inputs produce nothing and synthesized speech — plain, padded with silence,
or with noise added — transcribes verbatim, through the transcriber and through the production
taps and microphone on real devices.

## Implications

- Any audio path that can be silent for a whole window (an idle output tap, a muted mic, the
  flush tail) must be VAD-gated before whisper; RMS thresholds do not do it — noise at RMS 0.003
  still produced text.
- The VAD model (885 KB) ships inside the binary and is written to the model directory on first
  use, compared byte-for-byte; it is never downloaded.
- Each audio segment now yields at most one transcript line; whisper's sub-segments are joined.

## Related

- [[2026-09-06-constraint-whisper-rs-state-api-never-applies-whisper-cpp-vad]] — why the gate is application code rather than a whisper.cpp flag
- [[2026-09-05-decision-capture-engine-requires-a-transcriber]] — the transcriber the gate now lives inside
- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — why the bundled model is compared by bytes, not by presence
