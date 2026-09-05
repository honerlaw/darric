# One shared error slot with many writers cannot be cleared by any of them without provenance

**Date**: 2026-09-05
**Type**: pattern
**Summary**: `useSession` exposes one `error` string that five commands write and `App` renders in one bar; adding `setError(null)` to two more commands meant a successful delete silently erased a still-true model-download failure
**Context**: .minerva/work/2026-09-05-surface-failed-session-writes

## Context

`useSession.remove` and `update` were the only commands with no `try/catch`, so a rejected
`delete_session` reached nothing that displays it (#30). The obvious fix was to copy the shape
`start`, `stop` and `resume` already use — including their opening `setError(null)`, which carries
this comment:

```ts
// A failed download's message would otherwise outlive the retry that fixed it
// and sit in the error bar for the rest of the session.
setError(null);
```

Copying it looked like consistency. Review caught that it was not.

## Finding

**`start` and `resume` may clear `error` because they _are_ the retry of the thing that failed.**
A model download fails, the user presses Record again, and that press is an attempt at the very
operation whose failure is on screen. Clearing is correct there because the producer and the clearer
are the same operation.

`remove` and `update` are unrelated operations sharing the same slot. Once they clear it too:

- The speech-model download fails with `disk full`. That bar is the **only** notice — `App` never
  renders `modelReady`, and the Record button stays enabled.
- The user deletes any unrelated old recording. It succeeds.
- The download failure is gone permanently, with no way to bring it back.

A second case in the same class: delete A fails with `database is locked`; the user gives up on A
and deletes B instead; B's entry clears A's message while A is still sitting in the sidebar
undeleted.

**The slot has no provenance, so "clear the error" cannot mean "clear my error".** The fix is to
give the clearing operation a memory of what it wrote:

```ts
const writeErrorRef = useRef<string | null>(null);
// on failure:  writeErrorRef.current = message; setError(message);
// on success:  clear only if the slot still holds that same message
```

Anything another subsystem has written on top is left alone.

## Implications

- **Before copying an error-clearing idiom, ask whether the copying command is the retry of the
  failure it would clear.** If it is not, the copy is a silent erase of someone else's message.
- **A shared notification slot with N writers needs provenance before any writer may clear it** — not
  before it may _write_, which is last-writer-wins and usually fine.
- The general shape here is one variable answering questions it cannot answer at once, which is the
  same failure as [[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]] one level
  down: that entry has one value and two _readers_, this one has one slot and many _writers_.
- A single `error: string | null` is the cheapest thing to reach for and it stops scaling at exactly
  two producers. Worth knowing before adding a third.

## Related

- [[2026-09-05-pattern-shared-state-cannot-be-cleared-for-one-reader]] — the same "one variable, two questions" failure from the reader side
- [[2026-09-05-bug-a-functional-updater-reads-a-ref-after-the-caller-has-moved-on]] — the bug introduced while implementing this fix
- [[2026-09-05-pattern-fixing-one-path-leaves-the-other-one-open]] — see also
