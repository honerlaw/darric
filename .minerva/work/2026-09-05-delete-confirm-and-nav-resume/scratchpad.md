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
