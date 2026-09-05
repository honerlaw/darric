# Moving a control to global chrome drops the preconditions and the identity its old mount point supplied

**Date**: 2026-09-05
**Type**: pattern
**Summary**: "Resume recording" moved from the entity-scoped `RecorderPane` into the header and silently lost two things the pane had provided for free — the guarantee that a recording was selected, and any statement of _which_ recording it acts on
**Context**: .minerva/work/2026-09-05-delete-confirm-and-nav-resume

## Context

Both ways to begin capturing should sit together, so "Resume recording" moved out of
`RecorderPane`'s footer and into the header beside Record.

In the footer, `canResume` was `!isRecording && downloadProgress === null`. That looks like the
complete condition and is not: `RecorderPane` returns a placeholder early when `session === null`,
so the footer was unreachable without a selection. The early return was silently ANDing a third
clause onto the gate.

## Finding

**A component's mount point supplies context to everything inside it, and relocation removes that
context without removing anything from the code.** Two distinct things were lost here.

1. **A precondition.** `viewingSession !== null` had to be written out explicitly, because the
   header has no early return to supply it. Copying `canResume` across unchanged would have
   rendered Resume with nothing to act on.

2. **The target's identity.** In the footer the button sat under the recording it would continue,
   directly below that recording's title — the answer to "which one?" was the surrounding
   component. In global chrome nothing says it. A screen-reader user tabbing to a bare "Resume
   recording" cannot tell what they are about to append audio to. The fix collapsed the boolean into
   the thing that carries the answer: `resumeTarget: string | null` both gates the button and names
   it (`Resume recording “Standup”`).

This is the mirror image of
[[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]]. There, an enclosing
component's precondition was ANDed onto a feature that needed the opposite state, and the feature
could never render. Here, removing the enclosure _deleted_ preconditions that were load-bearing.
Same mechanism, opposite direction — which is why "just move the JSX" is never the whole change.

**A green suite says nothing about which clauses of a compound guard it covers.** The relocated gate
ended up with four (`viewingSession !== null && !isRecording && !isStarting && downloadProgress ===
null`), and the suite passed with `!isRecording` deleted — the case needs _two_ recordings and a
selection change mid-capture to appear at all. Deleting each clause in turn is what found it.

The same check caught a test that read stronger than it was: adjacency was asserted as
`resume.parentElement === record.parentElement`, which any two children of the header satisfy. A
mutation that moved the button elsewhere in the header survived it. `record.previousElementSibling
=== resume` kills it.

## Implications

- **Before moving a control, enumerate what its current ancestors guarantee**: early returns,
  conditional mounts, and props threaded in by the parent. Each is a clause that has to be written
  out by hand at the new site or it is gone.
- **A control in global chrome must name its target.** Entity-scoped placement answers "which one?"
  by position; global placement answers it only if the accessible name does.
- **Prefer one prop that carries the answer over a boolean plus a lookup.** `resumeTarget: string |
null` gates _and_ names; `canResume: boolean` could only gate, and the name had to be invented
  somewhere else.
- **Mutation-test each clause of a compound guard separately.** A guard with four clauses needs four
  deletions, not one green run — and an assertion about DOM structure should be mutated by moving
  the element, not by checking the assertion reads plausibly.
- Keeping the accessible name stable across the move (`/Resume recording/`) let the pre-existing
  `App` tests drive the relocated button unmodified. Tests that were written against the old
  location and still pass are stronger evidence the move preserved behaviour than tests rewritten
  alongside it.

## Related

- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — the same mechanism in the opposite direction: a mount point ANDing a precondition on rather than removing one
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — the guards a shape-driven change cannot see; that unit's example was this very button, passed `!isRecording` instead of a per-session flag
- [[2026-09-05-constraint-aria-modal-promises-inertness-that-nothing-enforces]] — see also, the other accessibility gap in the same change
- [[2026-09-05-decision-strip-darric-to-a-recorder]] — see also
- [[2026-09-05-pattern-renderhook-reads-callbacks-fresh-so-stale-closures-cannot-fail]] — see also
