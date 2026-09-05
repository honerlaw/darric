# Proposal: delete-confirm-and-nav-resume

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)

## Goal

Two small, independent UI corrections to the recorder chrome:

1. The sidebar's delete affordance — today a bare `×` glyph that deletes on the first click — becomes
   a trash-can icon whose click opens a confirmation modal. Nothing is deleted until the user confirms.
2. The "Resume recording" button moves out of the `RecorderPane` footer and into the top nav beside
   `Record`, and appears only when there is an existing recording that can actually be resumed.

## Why

**Delete is irreversible and unguarded.** `delete_session` removes the session row and its transcript.
`RecordingList` renders the trigger as a `×` at 13px, revealed on row hover, immediately adjacent to the
row's own select button. A mis-aimed click on a crowded sidebar destroys a recording with no undo and no
prompt. The `×` also reads as "dismiss" or "close" rather than "delete" — the glyph understates what the
control does. A trash can names the action, and a confirmation modal makes the destructive step
deliberate.

**Resume is filed under the wrong thing.** Starting capture is a global, chrome-level action: `Record`
lives in the header, and `Stop` replaces it there. `Resume` is the same action against an existing
session, but it sits at the bottom of the transcript pane, below a scrolling transcript, in a footer that
appears and disappears. The two ways to begin capturing are in two different places, and the one that is
harder to find is the one that continues work already in progress. Putting `Resume` next to `Record`
makes "how do I start capturing?" have one answer in one place.

The move also fixes an implicit gate. `canResume` is `!isRecording && downloadProgress === null` — it
never checks that a session is _selected_, because `RecorderPane` returns its placeholder early when
`session === null` and the footer was therefore unreachable. In the header there is no such early return,
so the condition has to be stated: Resume is offered only when a recording is selected.

## Approach

**Two independent changes; no backend work, no new dependencies.** Both halves needed more than the
relocation of markup they looked like — the details below are what shipped, not what was planned.

### 1. Trash icon + confirmation modal (`RecordingList`, new `ConfirmDialog`)

The repo has no icon library and draws its few glyphs by hand, so the trash can is an inline SVG in
`RecordingList` — stroke-based, `currentColor`, `aria-hidden` so the accessible name still comes from
the button's `aria-label`.

The confirmation is a new `src/components/ConfirmDialog.tsx`: a `fixed` backdrop (so the sidebar's
`overflow-y-auto` cannot clip it) around a panel with `role="dialog"`, `aria-modal="true"`,
`aria-labelledby` and `aria-describedby`. `RecordingList` owns `pendingDeleteId`; `App` is untouched
by this half. The pending session is **re-resolved from the live `sessions` array on every render**
rather than latched beside the id, so a session that disappears while the prompt is open closes it
instead of leaving a confirm button wired to a stale id.

Three things here are not obvious and are the reason the component is 100 lines rather than 30:

- **Dismissal is keyed to `mousedown`, not `click`.** A click's target is the common ancestor of its
  press and its release, so pressing on the dialog's body text and releasing over the dim area
  dispatches the click _on the backdrop_ — a click-based backdrop closes the dialog mid-selection,
  and `stopPropagation` on the panel structurally cannot see it. `e.target === e.currentTarget` on
  the click does not fix it either, for the same reason. See
  [[2026-09-05-bug-a-click-targets-the-common-ancestor-of-its-press-and-release]].
- **`aria-modal="true"` is a promise the code has to keep.** Without containment, three Tabs from
  Confirm reached the Record button behind the backdrop, where Enter starts a recording under a modal
  the user cannot see past. The document keydown handler now cycles Tab and Shift+Tab over the
  panel's buttons. See [[2026-09-05-constraint-aria-modal-promises-inertness-that-nothing-enforces]].
- **Focus is restored to the opener on close**, and the hover-revealed trigger gained
  `focus-visible:opacity-100` — a keyboard user was otherwise tabbing onto an invisible button and
  being dropped on `<body>` when they cancelled.

`window.confirm` was rejected as unstyleable OS chrome; lifting `pendingDeleteId` into `App` was
rejected because a `fixed` overlay already escapes the sidebar's clipping and there is one caller.

### 2. Resume in the header (`Header`, `App`, `RecorderPane`)

`RecorderPane` loses its resume props and its footer entirely. `Header` gains `onResume` and a single
`resumeTarget: string | null` — **not** the `canResume: boolean` originally planned. One prop both
gates the button and names it (`aria-label={`Resume recording “${resumeTarget}”`}`), because a control
in global chrome no longer sits under the recording it acts on and nothing else says which one it is.
The visible text stays `Resume`; the styling follows the header's pill button so Record and Resume
read as one group.

`App` computes the target:

```ts
resumeTarget={
  viewingSession !== null && !isRecording && !isStarting && downloadProgress === null
    ? sessionLabel(viewingSession)
    : null
}
```

Three of those clauses are the old `canResume` plus the selection gate `RecorderPane`'s early return
used to supply implicitly — relocating a control deletes the preconditions its mount point was
quietly providing. The fourth, `!isStarting`, is new: `useSession.resume` leaves the gate open for the
whole in-flight resume, which was harmless in a footer the user had just clicked out from under but in
the header would sit enabled beside a Record button already reading "Starting…". See
[[2026-09-05-pattern-relocating-a-control-drops-the-context-its-mount-point-supplied]].

`sessionLabel` moved from `RecordingList` into `lib/utils` so the row, the delete trigger's label, the
prompt and the Resume button all name a recording the same way — they previously diverged for a topic
of `""`, where the trigger's accessible name collapsed to a bare "Delete".

## Success criteria

1. The sidebar's per-row delete control renders a trash-can icon, not a `×`, and keeps its
   `Delete <topic>` accessible name.
2. Clicking that control deletes nothing — it opens a dialog naming the recording.
3. Confirming in the dialog calls `onDelete` with that recording's id exactly once and closes the dialog.
4. Cancelling — via the cancel button or Escape — closes the dialog and calls `onDelete` zero times.
5. The Resume button renders in the header next to Record, and no Resume control remains in
   `RecorderPane`.
6. Resume is absent when nothing is selected, when a recording is in flight, while the speech model is
   downloading, and while a start is in flight; present when a stopped recording is selected and none of
   those hold.
7. Clicking Resume in the header resumes the _selected_ recording — the existing `App` tests that drive
   resume through the accessible name `Resume recording` still pass, unmodified.
8. Every new behaviour is mutation-tested: reverting each fix individually fails the suite.
9. `npm run check` passes (typecheck, typecheck:node, lint, format, clippy, rustfmt, tests).

## Deferred work

One item, filed as [#30](https://github.com/honerlaw/darric/issues/30): `useSession.remove` has no
`catch` and `App.handleDelete` calls it with a bare `void`, so a failed `delete_session` leaves the
user having explicitly confirmed a delete that silently does not happen. Pre-existing — this unit
raised its stakes rather than creating it, so it was filed rather than folded in.

## Open Questions

None.
