# A transcript line's `recorded_at` is when its audio was captured, not when whisper finished

**Date**: 2026-09-06
**Type**: decision
**Summary**: every segment carries the wall-clock time of its first sample through the pool into `recorded_at`, and the live view inserts lines by it, so two devices' lines sort back into speech order
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

`persist_and_emit` stamped each line with `Utc::now()` as it was written — the moment whisper
finished, seconds after the audio and in whatever order the workers completed across devices.
The display orders by `recorded_at`, so two devices' lines interleaved in transcription order,
and the MCP `get_transcript` description had to apologise for it.

## Finding

`Segment { samples, captured_at }` leaves the segmenter with the time its first sample arrived;
`SegmentJob` and `TranscribedLine` carry it; `audio::recorded_at(line)` renders it for both
the insert and the `transcript_chunk` event. `db::sessions::transcript_lines` already ordered
by `recorded_at`, so the reload view interleaves devices in speech order for free. The live
view appended chunks in arrival order, which would have disagreed with a reload; it now inserts
each line after the last line stamped at or before it.

Rowid order — the MCP cursor — is still insertion order and therefore transcription-completion
order; the divergence from timestamp order is now routine rather than a race, and the tool
description says to sort by `recorded_at`.

## Implications

- Anything that orders lines for a human sorts by `recorded_at`; anything that pages sorts by
  rowid. They differ by design.
- A stamp is only as good as the segmenter's clock; the re-anchoring rule in
  [[2026-09-06-bug-capture-stamps-drifted-after-a-delivery-gap]] is what keeps it honest.
- The `transcript_chunk` payload still carries `recorded_at`; the frontend contract is unchanged.

## Related

- [[2026-09-06-decision-segments-end-at-pauses-found-by-an-energy-detector]] — where the stamp originates
- [[2026-09-06-constraint-a-table-rebuild-renumbers-transcript-rowids-and-every-mcp-cursor]] — why the cursor stays on rowid
- [[2026-09-06-bug-capture-stamps-drifted-after-a-delivery-gap]] — the failure the stamp is guarded against
