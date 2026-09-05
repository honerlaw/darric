# Proposal: surface-failed-session-writes

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)
**Closes**: #30

## Goal

Make a failed `delete_session` or `update_session` visible, and stop the UI from acting as though a
delete succeeded before it has. `useSession.remove` and `update` are the only two commands in the
hook that do not catch — a rejection from either reaches nothing that displays it.

## Why

`start`, `stop` and `resume` each wrap their command in `try/catch` and push the message into the
`error` state `App` renders in the bottom bar. `remove` and `update` do not:

```ts
const remove = useCallback(
  async (id: string): Promise<void> => {
    await deleteSession(id); // no try/catch
    if (activeSessionId === id) setActiveSessionId(null);
    await refresh();
  },
  [activeSessionId, refresh],
);
```

`App.handleDelete` calls it with a bare `void`, so a rejection surfaces only as an unhandled promise
rejection in a devtools console the packaged app does not show.

This predates `2026-09-05-delete-confirm-and-nav-resume`, which changed what it costs. The delete
used to be a stray click on a bare `×`; it is now a trash-can icon plus a modal stating "This
removes the recording and its transcript. It cannot be undone." A user who reads that sentence and
presses Delete has been given an explicit promise.

**There is a second half to the same defect.** `handleDelete` deselects _before_ the delete
resolves:

```ts
const handleDelete = (id: string): void => {
  if (id === viewingSessionId) setViewingSessionId(null); // optimistic
  void removeSession(id);
};
```

So a failed delete does not merely stay silent — it also drops the user out of the recording that
still exists, onto the "Select a recording" placeholder. Fixing only the error message would leave
that behaviour intact, which is the shape
[[2026-09-05-pattern-fixing-one-path-leaves-the-other-one-open]] records.

## Approach

**Make every uncaught session command match the ones that already get this right, and make the one
optimistic UI update conditional on the write having succeeded.** What shipped differs from the plan
in two places, both found in review and both recorded below.

1. **`remove` catches, and reports whether it deleted.** Return type `Promise<boolean>`, documented
   as "the delete completed **and** the list refreshed" — not the narrower "did `delete_session`
   succeed". The two diverge when the delete lands and the following `refresh()` throws: `sessions`
   still holds the recording, so a caller that deselected on the narrow reading would point the user
   away from a row still visible in the sidebar.

   `setActiveSessionId` runs **after** the refresh, not before. The plan had it before, which
   half-applied the delete behind a `false` answer — leaving a resumable-looking row whose session
   was gone.

2. **`App.handleDelete` deselects only on success**, with the functional-setState guard
   `useSession.resume` already uses for `startingSessionId`, so a selection change during an in-flight
   delete is not clobbered. Moving the deselect after the await is what opens that window; it did not
   exist in the optimistic version.

3. **`activeSessionId` gets the same functional guard** inside `remove`, dropping it from the
   callback's dependencies. The stale window this closes needs an interleaving — stop A, begin
   deleting A, press Record again, let the delete resolve — that no single call can produce, which is
   why the first version of this unit's test suite could not fail when the fix was reverted. See
   [[2026-09-05-pattern-renderhook-reads-callbacks-fresh-so-stale-closures-cannot-fail]].

4. **`update` catches too**, and **`refresh`'s mount call site catches** — the third uncaught command,
   missed at proposal time. The catch goes at the call site rather than inside `refresh`, because
   `refresh` has to keep rejecting: its other callers all await it inside their own `try`, and that is
   what lets `remove` report a stale list.

5. **Error clearing is provenance-aware — the plan's `setError(null)` on entry was wrong.** `error`
   is one shared slot; `start`/`resume` may clear it unconditionally because they _are_ the retry of
   the thing that failed, and `remove`/`update` are not. Copying the idiom meant a successful delete
   erased a still-true model-download failure, permanently. They now clear only their own message via
   `writeErrorRef`. See [[2026-09-05-pattern-one-error-slot-many-writers-needs-provenance]], and
   [[2026-09-05-bug-a-functional-updater-reads-a-ref-after-the-caller-has-moved-on]] for the bug that
   implementing it introduced.

**The alternative considered and rejected**: clearing the selection declaratively, from an effect
watching for the viewed session disappearing out of `sessions`. It covers more paths, but `sessions`
is empty on the first render before `refresh()` resolves, so it needs a has-loaded guard to avoid
clearing a valid selection at mount — more moving parts than the defect warrants, and it would not
make the failed delete any more visible, which is what #30 is about.

## Success criteria

1. A rejected `delete_session` sets the error state, and its message is visible in `App`'s error bar.
2. A rejected `delete_session` leaves the recording selected — the user is not dropped onto the
   "Select a recording" placeholder for a recording that still exists.
3. A successful delete still clears the selection, and still clears `activeSessionId` when it was the
   active recording.
4. A selection change during an in-flight delete is not clobbered when that delete resolves.
5. A rejected `update_session` sets the error state rather than rejecting unhandled.
6. A `remove` / `update` failure message is cleared by that command's **own** later success, and
   a message another subsystem wrote in the meantime is left alone.
7. Neither `remove` nor `update` rejects to its caller any more, so no call site needs a `.catch`.
8. Every new behaviour is mutation-tested: reverting each fix individually fails the suite.
9. `remove` returning false means nothing was applied — `activeSessionId` is untouched on the
   refresh-failure path.
10. A failed `list_sessions` at mount is reported rather than rejecting unhandled behind an empty
    sidebar.
11. A start that overtakes an in-flight delete keeps its `activeSessionId`.
12. `npm run check` passes (typecheck, typecheck:node, lint, format, clippy, rustfmt, tests).

## Deferred work

Two pre-existing defects surfaced by this unit's review and correctly scoped out of it, both filed:

- [#33](https://github.com/honerlaw/darric/issues/33) — overlapping `list_sessions` refreshes are
  last-writer-wins, so a slower earlier query can resurrect a deleted row in the sidebar.
- [#34](https://github.com/honerlaw/darric/issues/34) — the actively-recording session can be deleted
  while its capture engine keeps running, writing transcript rows against a deleted session.

## Open Questions

None.
