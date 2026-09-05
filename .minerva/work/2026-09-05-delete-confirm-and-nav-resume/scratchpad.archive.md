# Scratchpad: delete-confirm-and-nav-resume

## Quick decisions 2026-09-05

- [decided] pre-flight: no in-flight collision — all work-unit branches have MERGED PRs, no open PRs, no same-project live peers
- [decided] open-issue match: none — the three open `minerva:followup` issues (#16 #17 #18) are backend/lint, unrelated to the seed
- [decided] scope check: one work unit, one PR — two independent small UI edits, no phases needed
- [decided] approach: local `pendingDeleteId` in RecordingList + extracted `ConfirmDialog` (dominant — `window.confirm` is not the app's design language and is unassertable; lifting state to App is prop drilling for one caller)
- [decided] approach: Header gains `canResume`/`onResume`; RecorderPane's footer is deleted rather than kept in parallel (two Resume controls would be the bug being fixed)
- [decided] soundness: no public interface, no backend, no `.minerva/knowledge/` constraint in tension

## Implementation notes 2026-09-05

- `ConfirmDialog` binds Escape to `document`, not to the panel. The click that opens the dialog
  leaves focus on the trigger button until the focus effect runs, and a `onKeyDown` on the panel
  would miss an Escape pressed in that gap.
- The backdrop is click-to-dismiss, so the panel needs `stopPropagation` — without it, clicking the
  dialog's own body text closes the dialog the user is still reading. Mutation-confirmed.
- `pendingDelete` is resolved from the live `sessions` array each render rather than stored beside
  the id. A session removed from elsewhere while the prompt is open then closes it, instead of
  leaving a confirm button wired to a stale id. Mutation-confirmed.
- The header Resume button keeps `aria-label="Resume recording"` — the accessible name the existing
  `App` tests already drive resume through. Deliberate: those tests then verify the relocation
  end-to-end without being rewritten, which is stronger evidence than rewritten ones.
- Only one existing test line changed anywhere: `App.test.tsx`'s import, to add `within`. Every
  existing assertion is untouched.

## Review triage 2026-09-05

Weak assertion found and fixed before review: the header adjacency test originally asserted
`resume.parentElement === record.parentElement`, which the header satisfies for any child. A
relocation mutation survived it. Strengthened to `record.previousElementSibling === resume`, which
kills it.

Gate coverage gap found by mutation testing: dropping `!isRecording` from `App`'s `canResume` left
the suite green. `App.test.tsx` gained "withholds Resume while a recording is in flight" — it selects
a past recording mid-capture, which satisfies every other clause and isolates that one. All four
clauses are now independently killed.

### Code review findings, triaged

Fresh-context code review, 8 findings. Dispositions by the deferral bar, not by severity.

- **1 — FIXED.** The backdrop dismissed on `click`, and a click's target is the common ancestor of
  its press and its release. Pressing on the dialog's body text, dragging to select it, and
  releasing over the dim area reports the _backdrop_ — so the dialog closed mid-selection, and the
  panel's `stopPropagation` could not see it. Worse, the test asserting that case only performed a
  stationary click, which `stopPropagation` did handle: **the suite asserted a guarantee the code
  did not provide.** Dismissal is now keyed to `onMouseDown` on the backdrop with a
  `target === currentTarget` check; `stopPropagation` is gone as unnecessary. Note that the obvious
  fix — a `target === currentTarget` check on the _click_ — does not work: the drag case reports
  the backdrop as the target, so it would still cancel.
- **2 — FIXED.** `aria-modal="true"` with no focus containment. Three Tabs from Confirm reached the
  Record button behind the backdrop, where Enter starts a recording under a modal the user cannot
  see past; three Shift+Tabs reached a sidebar row, where Enter changes the selection while the
  dialog stays open naming the _previous_ recording. `aria-modal` is a promise to assistive
  technology that nothing enforces — the dialog now cycles Tab over its own buttons.
- **3 — FIXED.** Focus was moved into the dialog and never restored. Cancelling dropped a keyboard
  user on `<body>`, to re-traverse the whole sidebar. The opener is captured on mount and refocused
  in the effect's cleanup.
- **4 — FIXED.** The delete trigger is `opacity-0` until row hover, with no `focus-visible` counterpart:
  invisible while focused. Pre-existing, but this diff rewrites that exact line, and the deferral
  bar says behavior this diff touched is finished now rather than deferred.
- **5 — FIXED.** No `aria-describedby`. The body is the entire consequence statement and the reason
  the dialog exists; only the title was associated.
- **6 — FIXED.** The trigger's `aria-label` used `s.topic ?? "recording"` while every other label site
  went through `sessionLabel()`. They diverge for `topic === ""`: the label collapsed to a bare
  "Delete", colliding with the dialog's own confirm button. `sessionLabel` moved to `lib/utils` and
  is now the single answer.
- **7 — FIXED.** Resume no longer named its target. In the pane footer it was visually attached to the
  recording on screen; in global chrome nothing said which recording it appends to. The `canResume`
  boolean became `resumeTarget: string | null` — one prop that both gates the button and names it
  (`Resume recording “Standup”`). Collapsing the two is why this reads as a simplification rather
  than an added prop.
- **8 — IGNORED.** The document keydown listener is re-registered on every dialog render because
  `onCancel` is an inline arrow. Real, and harmless: the dialog renders only while it is open, and
  a `useCallback` pair to avoid two `addEventListener` calls is noise.

The review found nothing in three categories it looked at specifically: dead props or unreachable
code from the footer removal, stale-closure hazards around `pendingDeleteId`, and a Resume gate that
could leave the user unable to resume with no indication why (every false branch has visible chrome
explaining itself).

### Deferred — filed, not fixed here

`useSession.remove` has no `catch`, and `App.handleDelete` calls it with a bare `void`. `start`,
`stop` and `resume` all catch into the error bar; `remove` and `update` do not. A failing
`delete_session` now leaves the user having **explicitly confirmed** a delete that silently does not
happen — the confirmation modal raises the stakes of a gap it did not create. Pre-existing and
outside this unit's approach, so it is filed rather than folded in, following how this repo handled
the defects `2026-09-05-stop-feedback` surfaced.
