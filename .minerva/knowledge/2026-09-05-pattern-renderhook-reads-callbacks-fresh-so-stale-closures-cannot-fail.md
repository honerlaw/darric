# `result.current` is re-read on every access, so a stale-closure fix cannot be failed by a test that calls through it

**Date**: 2026-09-05
**Type**: pattern
**Summary**: converting `remove` from a closure read of `activeSessionId` to a functional `setState` was a real fix, and reverting it passed all 89 tests — every test reached the callback through `result.current`, which always hands back the newest one

**Context**: .minerva/work/2026-09-05-surface-failed-session-writes

## Context

`useSession.remove` read `activeSessionId` out of its own closure:

```ts
if (activeSessionId === id) setActiveSessionId(null); // captured at render
```

It was converted to `setActiveSessionId((current) => (current === id ? null : current))`, which
also dropped `activeSessionId` from the callback's dependency array. The unit claimed every new
behaviour was mutation-tested. Review reverted this one and the whole suite stayed green.

## Finding

**`renderHook`'s `result.current` is a live getter.** Every `result.current.remove(...)` in a test
resolves the callback _at call time_, so it is always the one built from the latest render — the
closure it captured is never stale by the time the test invokes it. A test written that way cannot
distinguish the two implementations, no matter how many cases it covers.

The stale window is real, and opening it takes an interleaving no single call can produce:

1. Record and stop session A. `activeSessionId` stays `"A"` — it deliberately outlives the
   recording ([[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]]).
2. Start deleting A against a slow backend. `remove` is now suspended at its `await`, holding the
   closure it was created with.
3. Press Record again before the delete resolves. `activeSessionId` becomes the new session's id.
4. The delete resolves. The suspended closure compares its captured `"A"` against the `id` it was
   asked to delete, matches, and clears — nulling the id of the recording that is now running.

The sidebar's live dot disappears, every `viewingSessionId === activeSessionId` gate in
`RecorderPane` goes false, and the dropped-segment counter resets, all while capture continues.

To pin it, the test has to **start the call, change the state, and only then let the call finish** —
all inside one `act`, so the suspended closure is genuinely the old one.

## Implications

- **A mutation that survives is not automatically a mutation that does not matter.** Ask whether the
  test _could_ have failed: for a stale-closure fix reached through `result.current`, the answer is
  structurally no.
- **Testing a stale closure requires an in-flight call.** A sequence of completed `await act(...)`
  blocks re-reads the callback between each one and never overlaps them.
- The same blind spot applies to any React testing helper that re-resolves through a live getter —
  including `screen` queries against a component that re-renders between assertions.
- This is the mechanism behind a claim like "every new behaviour is mutation-tested" being true in
  intent and false in fact. The check that catches it is running the mutation, not writing the
  criterion.

## Related

- [[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]] — why `activeSessionId` outlives its recording, which is what makes this window reachable
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — the same question asked of a code path rather than a test's reach
- [[2026-09-05-pattern-relocating-a-control-drops-the-context-its-mount-point-supplied]] — see also, another compound guard whose clauses were not separately covered
- [[2026-09-06-constraint-jsdom-fires-no-blur-on-unmount]] — see also
