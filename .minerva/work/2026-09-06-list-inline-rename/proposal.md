# Proposal: list-inline-rename

**Date**: 2026-09-06
**Status**: Draft

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
dependency, and the pane's existing rename is untouched.

- `RecordingList` gains an `onRename: (id: string, topic: string) => void` prop and one piece
  of state, `editing: { id, draft } | null`.
- The row's select button gets `onDoubleClick`, which opens the editor with the current topic as
  the draft (`""` for an untitled recording, never the placeholder text). The single click still
  selects — a double-click fires two selects first, which is harmless.
- While `editing.id` matches a row, that row renders an `<input>` in place of the name, with
  `aria-label="Rename <label>"`, auto-focused with its text selected so typing replaces the name.
- The editor closes on a single path, `blur`: Enter and Escape both call `blur()` on the input,
  Escape after setting a cancel flag. The blur handler either commits or discards, and because
  it runs exactly once per edit, a commit cannot double-fire (Enter's commit followed by the
  unmount's blur) and a cancel cannot be undone by a trailing blur.
- Commit sends the trimmed draft to `onRename` only when it is non-empty **and** differs from
  the current topic; otherwise the editor just closes. Empty-and-different is treated as
  "abandon", matching the pane.
- `App` generalises its existing `handleRename` to take an id, wires the list's `onRename` to
  it, and keeps the pane's callback as the `viewingSessionId` case of the same function.

Considered and rejected:

- *Extract a shared inline-title editor used by both the pane and the list.* Cleaner, but it
  rewrites working pane code for a ~30 line saving and widens a change that should stay small.
- *Double-click selects the row and opens the pane's heading editor.* Keeps one editor, but
  the input appears far from where the user double-clicked and does not match the request.

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

## Open Questions

None.
