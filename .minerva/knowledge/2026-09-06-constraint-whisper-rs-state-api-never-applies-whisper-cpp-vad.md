# whisper.cpp applies its VAD only in `whisper_full`, which whisper-rs never calls

**Date**: 2026-09-06
**Type**: constraint
**Summary**: `enable_vad` on `FullParams` does nothing through whisper-rs, because `whisper_full_with_state` skips VAD; run `WhisperVadContext` yourself and feed whisper the speech
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

whisper-rs 0.16 exposes whisper.cpp's Silero VAD two ways: `FullParams::set_vad_model_path` +
`enable_vad`, which asks whisper.cpp to gate its own decoding, and a standalone
`WhisperVadContext`. The obvious integration is the first. It was prototyped against the real
large-v3-turbo model to close the silence hallucination in this unit.

## Finding

With `enable_vad(true)` set and a valid model path, eight seconds of digital zeros still decoded
to "Thank you." — identical to the un-gated result — and whisper.cpp's log never mentioned VAD.

In whisper.cpp (the copy bundled with whisper-rs-sys 0.15) the `params.vad` branch lives in
`whisper_full` and `whisper_full_parallel` only. `whisper_full_with_state`, the entry point that
takes a caller-owned state, has no such branch. whisper-rs's `WhisperState::full` calls
`whisper_full_with_state`, and whisper-rs exposes no context-level `full`, so through the
bindings the flag is dead.

The standalone `WhisperVadContext::segments_from_samples` works: zeros, low noise and a
1.5 s tail return zero segments in 5–26 ms; synthesized speech returns its regions. Concatenating
those regions with 100 ms of silence between them — what `whisper_full` itself does — and decoding
that buffer gives verbatim transcripts with and without silence padding.

## Implications

- Gate before whisper, in application code, with `WhisperVadContext`; do not set
  `set_vad_model_path` / `enable_vad` and expect anything.
- A silent buffer then never reaches the encoder, so a silent device costs milliseconds per
  segment instead of a full decode.
- `WhisperSegment::no_speech_probability()` read 0.00 for every segment, speech or silence, under
  both greedy and beam decoding in this build; it is not a usable second gate here.
- If whisper-rs later adds a context-level `full`, the integrated path may start working; the
  standalone gate would still be preferable because it lets the app skip the decode entirely.

## Related

- [[2026-09-06-bug-whisper-transcribes-silence-as-thank-you]] — the bug this constraint was found while fixing
- [[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]] — why skipping the decode for silence matters on one GPU
