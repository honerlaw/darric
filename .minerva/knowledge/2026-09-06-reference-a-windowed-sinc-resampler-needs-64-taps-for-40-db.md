# The band-limited resampler needs 64 taps to keep a 12 kHz tone 40 dB down after decimation

**Date**: 2026-09-06
**Type**: reference
**Summary**: a 32-tap Blackman sinc leaves a ~8 kHz transition band at 48 kHz, so 64 taps at 0.9× the lower Nyquist are used; it costs ~1 % of real time per stereo 48 kHz stream in release
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

`audio::resample` interpolated linearly with no filter, so everything above 8 kHz in a 48 kHz
capture folded down into the speech band before whisper heard it. The replacement is a
stateful windowed-sinc interpolator, one per capture stream, with the read position carried as
a fixed-point `u64` numerator over 16 000 and the rate ratio entering as gcd-reduced `u16`s —
no float is ever cast to an index, per the lint policy.

## Finding

The proposal said 32 taps. A Blackman window's transition band is about 5.5/N of the input
rate — roughly 8 kHz at 48 kHz for 32 taps — so a 12 kHz tone sits mid-transition and is
nowhere near the 40 dB the success criterion asks for. With 64 taps and the cutoff at 0.9× the
lower Nyquist the 12 kHz residue measures under −40 dB after decimation, 1 kHz passes within
1 dB, output is push-size independent to 1e-5, and 16 kHz input passes through untouched.

Cost: the per-output-sample loop evaluates 64 sinc terms with `sin` and `cos`; measured at
about 1 % of real time for a stereo 48 kHz stream in a release build, so it is fine on the
callback thread. A per-phase weight table (3 phases at 48 kHz, 441 at 44.1 kHz) would remove
the trigonometry entirely if that ever matters.

## Implications

- Do not "optimise" the tap count down without re-running
  `a_tone_above_the_new_nyquist_is_removed_not_aliased`; it is the test that fails first.
- `ratio()` reduces by gcd and converts through `u16`; a rate whose reduced form exceeds
  65 535 against 16 000 (none any device reports) would saturate silently.
- The filter's `HALF` (32) samples of history mean the first output lags the input by 2 ms and
  the last 32 input samples of a stream are never emitted; the stop-time flush does not miss
  anything audible.

## Related

- [[2026-09-06-decision-segments-end-at-pauses-found-by-an-energy-detector]] — downstream of the resampler on the same callback
