# Verifying a sequence of steps says nothing about whether the block runs at all

**Date**: 2026-09-05
**Type**: pattern
**Summary**: a completion check confirmed the flush/shutdown ordering inside `stop()` was correct, and it was — but the `if let Some(pool)` guard above it was never true, so none of the verified steps executed
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Phase 2's completion Verifier was asked, among other things, to confirm that
`CaptureEngine::stop()` flushes each segmenter's trailing partial segment into the transcription
pool **before** shutting the pool down — because flushing afterwards would silently drop the last
segment of every recording.

It read the function, traced the ordering, and reported it correct. The ordering _was_ correct.

An independent code review then found that the pool field was always `None`, so the guarded block
containing that correct ordering never ran (see
[[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]]). The exact failure the check was
commissioned to rule out — losing the last segment of every recording — was live the whole time.

## Finding

The question "are these steps in the right order?" and the question "does this code execute?" are
different questions, and answering the first is not evidence for the second. A reviewer directed
at an ordering will read the ordering; the enclosing condition is contextual, and a guard whose
value is decided fifty lines earlier in a different function reads as ambient rather than as the
thing under test.

This is not the reviewer being careless — it answered exactly what it was asked, correctly. The
defect is in the shape of the question.

## Implications

- When commissioning a check on logic inside a conditional, make the **reachability** of that
  block part of the question: "confirm this block runs, and that its steps are ordered correctly".
- Prefer verification that would notice absence. A test asserting the trailing segment is
  transcribed would have failed; a reading of the ordering could not.
- Treat a feature-gating `Option`, `if let`, or early return in the path under review as part of
  the surface to verify, not as background.
- Generalises past reviews: a claim of the form "X is correct" is only as strong as the implicit
  "and X happens".

## Related

- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — the defect that got through
- [[2026-09-05-bug-a-losing-rename-became-a-silent-none-transcriber]] — see also
- [[2026-09-05-decision-capture-engine-requires-a-transcriber]] — see also
- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — see also
