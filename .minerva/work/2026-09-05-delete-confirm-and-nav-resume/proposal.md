# Proposal: delete-confirm-and-nav-resume

**Date**: 2026-09-05
**Status**: Draft

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

**Two independent changes; no backend work, no new dependencies, no shared state lifted into `App` that
isn't already there.**

### 1. Trash icon + confirmation modal (`RecordingList`, new `ConfirmDialog`)

The repo has no icon library and draws its few glyphs by hand, so the trash can is an inline SVG in
`RecordingList` — stroke-based, `currentColor`, sized to the existing 13px control so hover/opacity
behaviour and the `aria-label` are unchanged. `aria-hidden` on the SVG keeps the accessible name coming
from the label, which is what the existing tests read.

The confirmation lives in a new `src/components/ConfirmDialog.tsx`: an overlay `div` with
`role="dialog"`, `aria-modal="true"`, an `aria-labelledby` title, a destructive confirm button and a
cancel button. It is `position: fixed`, so the sidebar's `overflow-y-auto` does not clip it. Escape
cancels; the confirm button takes focus on open.

`RecordingList` owns the `pendingDeleteId` state. Confirming calls the existing `onDelete(id)` prop and
clears the state; cancelling clears it alone. `App` is untouched by this half — the delete flow is a
sidebar concern and lifting it would only add prop drilling.

The two alternatives were weaker. `window.confirm` is one line but renders an OS chrome dialog outside
the app's design language and cannot be styled or reliably asserted against. Lifting `pendingDeleteId`
into `App` and rendering one dialog at the root buys nothing here: a `fixed` overlay already escapes the
sidebar's clipping, and there is exactly one caller.

### 2. Resume in the header (`Header`, `App`, `RecorderPane`)

`Header` gains `canResume: boolean` and `onResume: () => void`, and renders a Resume button immediately
before the Record button when `canResume` is true. It carries the visible text `Resume` and
`aria-label="Resume recording"` — the accessible name existing tests already match on, kept stable
deliberately. Styling follows the header's existing pill button rather than the pane footer's filled
accent button, so the two controls read as one group.

`RecorderPane` loses its `canResume` / `onResume` props and its footer entirely.

`App` computes:

```ts
canResume={viewingSession !== null && !isRecording && !isStarting && downloadProgress === null}
```

Three of those four conditions are today's `canResume` plus the selection gate `RecorderPane`'s early
return used to supply implicitly. The fourth, `!isStarting`, is new and deliberate: `useSession.resume`
leaves `canResume` true for the whole in-flight resume, which was harmless in a footer the user had just
clicked out from under, but in the header it would sit enabled beside a Record button already reading
"Starting…" and invite a second concurrent start. "Can actually be resumed" excludes "a start is already
running".

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

## Open Questions

None.
