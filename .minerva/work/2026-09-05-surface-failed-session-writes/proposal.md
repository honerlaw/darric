# Proposal: surface-failed-session-writes

**Date**: 2026-09-05
**Status**: Draft
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

**Make the two outliers match the three commands that already get this right, and make the one
optimistic UI update conditional on the write having succeeded.**

1. **`remove` catches, and reports whether it deleted.** It gains the `try/catch` +
   `setError(String(e))` shape `start` / `stop` / `resume` already use, plus the `setError(null)` on
   entry that keeps a failed attempt's message from outliving the retry that fixed it. Its return
   type becomes `Promise<boolean>` — `true` only when the backend actually deleted. A caller needs
   that answer, and inventing it from the refreshed `sessions` list at the call site would be
   guesswork.

2. **`App.handleDelete` deselects only on success**, and does it with the functional-setState guard
   `useSession.resume` already uses for `startingSessionId` — clear the selection only if it is
   still the id this call deleted, so a selection change mid-delete is not clobbered:

   ```ts
   void removeSession(id).then((deleted) => {
     if (deleted) setViewingSessionId((current) => (current === id ? null : current));
   });
   ```

3. **`activeSessionId` gets the same functional guard** inside `remove`, which also drops it from the
   callback's dependency array — it was read from a closure that could be a render behind.

4. **`update` catches too.** Its stakes are lower — an unchanged title is visible feedback that
   nothing happened — but it is the other half of the same gap, and leaving it is the pattern cited
   above. It keeps returning `void`: no caller has an optimistic update to gate.

**The alternative considered and rejected**: clearing the selection declaratively, from an effect
that watches for the viewed session disappearing out of `sessions`. It would cover more paths than
this one (a session deleted from elsewhere, say), but `sessions` is empty on the first render before
`refresh()` resolves, so it needs a has-loaded guard to avoid clearing a valid selection at mount.
That is more moving parts than the defect warrants, and it would not make the failed delete any more
visible — which is what #30 is actually about.

## Success criteria

1. A rejected `delete_session` sets the error state, and its message is visible in `App`'s error bar.
2. A rejected `delete_session` leaves the recording selected — the user is not dropped onto the
   "Select a recording" placeholder for a recording that still exists.
3. A successful delete still clears the selection, and still clears `activeSessionId` when it was the
   active recording.
4. A selection change during an in-flight delete is not clobbered when that delete resolves.
5. A rejected `update_session` sets the error state rather than rejecting unhandled.
6. A previous failure's message does not survive a later successful `remove` / `update`.
7. Neither `remove` nor `update` rejects to its caller any more, so no call site needs a `.catch`.
8. Every new behaviour is mutation-tested: reverting each fix individually fails the suite.
9. `npm run check` passes (typecheck, typecheck:node, lint, format, clippy, rustfmt, tests).

## Open Questions

None.
