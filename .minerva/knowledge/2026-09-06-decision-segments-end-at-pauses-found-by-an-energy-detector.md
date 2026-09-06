# Segments end at pauses found by a cheap energy detector; the VAD decides what is speech

**Date**: 2026-09-06
**Type**: decision
**Summary**: the audio callback cuts segments at 400 ms of quiet after at least 2 s (25 s cap) using per-frame RMS against an adaptive floor; Silero, in the transcriber, still owns speech-vs-not
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

The segmenter used to emit a fixed eight seconds regardless of what was being said, so
utterances were split mid-phrase and each half decoded with no context. Whisper does best on
whole utterances, so the cut had to move to where the speaker paused. Two things constrained
how: the segmenter runs on the Core Audio callback thread, which must not block, and the
Silero gate in `transcription::vad` already decides — off that thread — whether a segment has
speech at all.

## Finding

`audio::segmenter::Segmenter` classifies every 20 ms frame by RMS against a noise floor and
emits a segment when the buffer holds at least 2 s and the last 400 ms were non-speech, or at
25 s regardless. The floor updates from non-speech frames (drops at once, rises ≤ 2 %/frame)
and creeps ≤ 0.2 %/frame during speech, capped at the frame's own level — so a ten-second
utterance moves it ~2 % while steady room noise louder than 4× the initial floor, which first
reads as speech, is reclassified within about half a minute. With no speech heard yet only one
pause's worth of audio is kept, in whole frames, so silence never fills the buffer and the frame
grid stays aligned. The pause that ended a segment stays behind to open the next one.

The detector only chooses cut points. A noisy room degrades to cuts at the 25 s cap, never to
silence being transcribed, because the VAD still gates every segment before whisper. On the
spoken fixture the segmenter cut each pass at the natural pause between its two sentences.

The minimum is 2 s rather than the 3 s first proposed: the success criterion's stream (2 s of
speech, 0.6 s of pause, twice) cannot yield two segments under a 3 s minimum.

## Implications

- Cut points and speech detection are two mechanisms in two places on purpose; do not move
  Silero onto the callback thread to "improve" the boundaries, and do not let the energy
  detector drop audio.
- A remark shorter than 2 s waits until 2 s are buffered and leaves on the next pause, so the
  transcript lags speech by at most ~2.4 s plus decode time.
- The segmenter's clock must be re-anchored on delivery gaps; see
  [[2026-09-06-bug-capture-stamps-drifted-after-a-delivery-gap]].

## Related

- [[2026-09-06-bug-whisper-transcribes-silence-as-thank-you]] — the gate that makes an energy detector safe as the cutter
- [[2026-09-06-decision-recorded-at-is-the-capture-time]] — what each segment's start time becomes
- [[2026-09-06-bug-capture-stamps-drifted-after-a-delivery-gap]] — the clock rule this design needs
