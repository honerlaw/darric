# Proposal: list-inline-rename

**Date**: 2026-09-06
**Status**: Shipped (2026-09-06)

## Goal

Double-clicking a recording's name in the sidebar list turns that name into a text input, so a
recording can be renamed in place without first selecting it and clicking the pane heading.
Enter or clicking away commits the new name; Escape abandons it.

## Why

The only rename path today is the `RecorderPane` heading: select the recording, click the
28px title, type, press Enter. Renaming several recordings after a day of meetings means
bouncing between the list and the pane for each one. A double-click on the row itself is the
convention every file manager and editor sidebar uses for exactly this, and the backend already
has everything it needs — `update_session` exists and `useSession.update` wraps it with the
shared error-bar handling. This is a UI affordance, not a feature.

## Approach

**`RecordingList` owns the editor; `App` supplies the write.** No backend change, no new
dependency. What shipped:

- `RecordingList` has an `onRename: (id: string, topic: string) => void` prop and one piece of
  state, `editing: { id, draft } | null`. The row's select button opens the editor on
  `onDoubleClick` with the stored topic as the draft — `""` for an untitled recording, never the
  "Untitled recording" placeholder, which a stray Enter would otherwise store as a real name.
  The single click still selects; a double-click's two selects hit `setViewingSessionId` with
  the same id and React bails out.
- While `editing.id` matches a row, that row renders an `<input>` in place of the name,
  `aria-label="Rename <label>"`, auto-focused with its text selected.
- **Blur is the single close path.** Enter and Escape both call `blur()` on the input; Escape
  sets `cancelEditRef` first. The blur handler reads and resets the flag, then commits or
  discards. One event per edit means a commit cannot fire twice and a cancel cannot be undone by
  a trailing blur. Enter and Escape are ignored while `nativeEvent.isComposing` is set, so
  confirming an IME candidate does not commit half a word; the same one-line guard was added to
  the pane's title editor.
- Commit sends the trimmed draft to `onRename` only when it is non-empty **and** differs from
  the stored topic. An empty draft is an abandoned edit, matching the pane.
- `App.handleRename` now takes an id and sends the trimmed topic to `useSession.update`; the
  list passes it directly and the pane's callback supplies `viewingSessionId`. The old
  empty-to-`undefined` arm was unreachable and is gone.

### Tests

Eleven `RecordingList` tests cover: double-click opens, single click does not; untitled seeds
`""`; Enter commits once; blur commits; Escape abandons; empty and unchanged write nothing; a
cancelled edit does not poison the next commit; commit on an untitled recording; the editor
survives a `sessions` refresh; IME Enter is ignored; and Enter routes through blur. That last
one stubs `HTMLElement.prototype.blur` because jsdom fires no blur on unmount, so the
"exactly once" count alone could not distinguish the shipped design from a regression that
commits on Enter directly ([[2026-09-06-constraint-jsdom-fires-no-blur-on-unmount]]). One
`App` test drives a rename from the sidebar through a mocked `update_session` and asserts the
payload and the refreshed name. Every guard was mutation-tested.

### Incidental

`main` was red on `npm run format` because the reconcile PR (#43) appended a `## Related`
block to a knowledge entry without a blank line after the heading. Restored here so CI could go
green; the workflow gap is #44.

Considered and rejected: a shared inline-title editor for both the pane and the list (rewrites
working pane code for a small saving); routing the double-click into the pane's heading editor
(the input would appear far from the click).

## Success criteria

- Double-clicking a recording's name in the sidebar shows a focused text input prefilled with
  its topic; for an untitled recording the input is empty. A single click does not open it.
- Pressing Enter, or clicking elsewhere, with a new non-empty name calls `update_session` exactly
  once with that recording's id and the trimmed name, and the sidebar shows the new name.
- Pressing Escape closes the input without calling `update_session`.
- Committing an empty, whitespace-only, or unchanged name closes the input without a write.
- Selecting a row and renaming from the pane heading still work; all existing tests pass
  unmodified.
- `npm run check` passes with no new lint suppressions.

## Deferred work

- #44 — knowledge reconciliation appends `## Related` blocks that Prettier rejects, leaving main red on format (priority: medium)

## Open Questions

None.
