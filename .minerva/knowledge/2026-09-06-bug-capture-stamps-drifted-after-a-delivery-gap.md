# Capture stamps drifted after any delivery gap because the segmenter anchored its clock once

**Date**: 2026-09-06
**Type**: bug
**Summary**: `started_at` was set only when the buffer was empty, and a pause cut never empties it, so after a rebuilt stream every later stamp extrapolated from the session's first callback; fixed by re-anchoring when a chunk arrives >100 ms late
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

The first pause-based segmenter took the wall clock when the buffer was empty and advanced it
by sample count thereafter. Review pointed out that a pause cut always leaves the 400 ms pause
behind, so on that path the buffer is never empty again and the clock is never re-read.

## Finding

A probe reproduced it: 2 s of speech and a pause (cut), then thirty seconds with no audio — a
stream being rebuilt after a failure, or callbacks dropped under load — then 3 s of speech. The
second segment was stamped 2.2 s after the session start while its audio arrived 32.6 s in.
Every later line on that device would have been thirty seconds early for the rest of the
recording, and sorted before lines from other devices that were actually spoken first.

`Segmenter::anchor` now compares each chunk's arrival (its end time minus its duration) with
where the buffered audio should end; if the chunk is more than 100 ms later than that, the
buffered audio keeps its duration and slides up to end where the chunk begins. A unit test
replays the probe.

## Implications

- Any component that timestamps a stream by counting samples needs a rule for gaps, and the
  rule needs a test that skips the clock forward — steady-state tests cannot see this.
- The 100 ms tolerance is above callback jitter and below anything a listener would notice;
  ordinary jitter never re-anchors.

## Related

- [[2026-09-06-decision-recorded-at-is-the-capture-time]] — what depends on the stamp
- [[2026-09-06-decision-segments-end-at-pauses-found-by-an-energy-detector]] — the segmenter that carries the clock
