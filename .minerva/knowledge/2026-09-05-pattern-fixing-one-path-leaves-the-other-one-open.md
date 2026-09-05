# A defect reached by two paths is not fixed until both are attributed

**Date**: 2026-09-05
**Type**: pattern
**Summary**: transcript lines could land under the wrong recording via a broadcast event _and_ via a slow initial query; the unit fixed the event path the issue named, and review found the query path producing the identical wrong screen
**Context**: .minerva/work/2026-09-05-session-scoped-ui-state

## Context

`useTranscript` fills a pane from two sources: a `getSessionTranscript` query when the selection
changes, and a live `transcript_chunk` event stream that is deliberately kept alive for 20 s past
the end of a recording so whisper's asynchronous flush lines still arrive.

A filed issue described the event half: the payload carried no session identity, so switching
recordings inside the linger window appended the stopping session's lines to whichever session
was now on screen. The unit added `session_id` to the payload and filtered on it.

## Finding

The query half had the same defect and no issue:

```ts
void getSessionTranscript(sessionId).then(setLines); // no stale-response guard
```

Select a recording, click a different one before its fetch resolves, and the first response lands
in the second pane. **Identical wrong screen, different path in** — and it needs no unusual timing
beyond a slow query.

Both halves are one question the code was not asking: _does this data belong to what is currently
displayed?_ The event path answers it with an id on the payload; the query path answers it by
comparing the resolved `sessionId` against the ref before calling `setLines`. Neither is
sufficient alone, and fixing only the one the issue named leaves the user-visible bug reachable.

The same shape appeared twice more in this unit, both found by review rather than by the issues:
a "recording now" dot gated on `isRecording` when three sibling readers already checked
`isStopping`, and a dropped-segment count attributed to a session but never reset when a
different one started.

## Implications

- **An issue names a path, not a defect.** Before closing one, ask what _else_ writes to the
  surface it describes. Here: "what else can put lines in this pane?" — a question the issue's
  own text did not prompt, because it was written from the reproduction rather than from the
  component.
- **Where sibling readers exist, match them.** Three of four consumers of the recording state
  already conjoined `isStopping`; the fourth was written from the issue's wording instead of
  from its neighbours, and shipped a dot that pulsed "recording now" through the whole stopping
  window.
- **Attribution belongs at every entry point to a shared surface**, not at the one that was
  reported. Once a pane is fed by more than one source, "which session is this for?" is part of
  each source's contract.

## Related

- [[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]] — the other half of this unit: the same surface, over-shared rather than under-attributed
- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — another defect that every individually-correct hop failed to reveal
- [[2026-09-05-pattern-one-error-slot-many-writers-needs-provenance]] — see also
