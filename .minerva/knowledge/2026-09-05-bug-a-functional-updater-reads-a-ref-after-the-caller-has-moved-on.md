# A functional `setState` updater reads a ref after the caller returns, so nulling the ref first makes it a no-op

**Date**: 2026-09-05
**Type**: bug
**Summary**: `clearOwnWriteError` nulled `writeErrorRef` and then called `setError` with an updater that compared against that ref — the updater ran later, saw the null this same call had just written, matched nothing, and cleared nothing
**Context**: .minerva/work/2026-09-05-surface-failed-session-writes

## Context

Fixing [[2026-09-05-pattern-one-error-slot-many-writers-needs-provenance]] meant clearing the shared
`error` slot only when it still holds the message this command put there. The first implementation
read the wrong way round:

```ts
const clearOwnWriteError = useCallback((): void => {
  setError((current) => (current === writeErrorRef.current ? null : current));
  writeErrorRef.current = null; // <- runs first, in practice
}, []);
```

## Finding

**A functional `setState` updater is a closure React invokes during the next render pass, not a
callback that runs inside `setError`.** By the time `(current) => ...` executes, the synchronous
line below it has already run, so `writeErrorRef.current` is `null`. The comparison becomes
`current === null` — false whenever there is a message to clear — and the function reliably does
nothing at all, in exactly the case it exists for.

The failure is silent in both directions: no error, no warning, and the state simply stays as it
was. What caught it was an existing test asserting the message _was_ cleared after a retry
succeeded. Without that test the ref would have read as working — the "leave another subsystem's
message alone" case passes either way, because doing nothing is the correct outcome there.

The fix is to capture the ref before mutating it:

```ts
const own = writeErrorRef.current;
writeErrorRef.current = null;
setError((current) => (current === own ? null : current));
```

## Implications

- **An updater passed to `setState` must close over values, not over mutable containers the same
  function is about to change.** Read a ref into a `const` first; the `const` is what the updater
  should compare against.
- The reason to use the functional form at all — avoiding a stale render-time read — does not make
  it immune to staleness. It moves the read _later_, which is a different hazard in the same family.
- **A guard whose failure mode is "does nothing" needs a positive test.** Every negative assertion
  around this ref passed against the broken version, because "leave it alone" and "silently do
  nothing" are indistinguishable from outside.

## Related

- [[2026-09-05-pattern-one-error-slot-many-writers-needs-provenance]] — the fix this bug was introduced while implementing
- [[2026-09-05-pattern-state-changed-only-in-finally-reads-as-a-dead-click]] — see also, another state write whose timing was the defect
- [[2026-09-05-bug-the-session-start-guard-is-check-then-act]] — see also
