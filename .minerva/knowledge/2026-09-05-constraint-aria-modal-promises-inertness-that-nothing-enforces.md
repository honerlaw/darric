# `aria-modal="true"` promises the rest of the page is inert, and nothing enforces it

**Date**: 2026-09-05
**Type**: constraint
**Summary**: adding `aria-modal` without focus containment tells assistive technology the background is unreachable while Tab still walks straight out of the dialog into live controls behind the backdrop — three Tabs from the delete prompt reached Record, where Enter starts a recording
**Context**: .minerva/work/2026-09-05-delete-confirm-and-nav-resume

## Context

`ConfirmDialog` was written with `role="dialog"`, `aria-modal="true"`, `aria-labelledby`, Escape
to cancel and autofocus on the confirm button. That is the checklist most modal guidance gives,
and it looks complete.

## Finding

**`aria-modal` is an assertion, not a mechanism.** It changes nothing about focus order. With the
delete prompt open and focus on Confirm, the real forward tab order in this app was:

```
Capture MacBook Mic → body → Resume recording → Record → Standup 30m
```

Every one of those is behind a dimmed backdrop that makes it unclickable, and every one is still
reachable by keyboard:

- Tab ×3 then Enter **starts a recording** while a modal delete prompt is on screen.
- Tab ×1 then Enter toggles a capture device.
- Shift+Tab ×3 reaches a sidebar row; Enter there changes `viewingSessionId` while the dialog
  stays open still naming the **previously** selected recording — so the user is looking at one
  recording and confirming the deletion of another.

The attribute makes this worse rather than neutral: a screen-reader user is told the background is
inert and has no reason to check.

Two further omissions travel with it, and both are keyboard-only so both are invisible in a
screenshot:

- **Focus is never restored.** After Escape, `document.activeElement` was `<body>`. A keyboard user
  who tabs down a long sidebar to reach a delete trigger and then cancels is dropped at the top and
  must traverse it again.
- **The trigger is `opacity-0` until row hover with no `focus-visible` counterpart**, so the button
  they tabbed onto was invisible while focused.

## Implications

- **If you write `aria-modal="true"`, you owe the focus trap.** Cycle Tab and Shift+Tab over the
  dialog's own focusables, or mark the app root `inert`. Do not ship the attribute alone.
- **A dialog owns the focus it takes.** Capture `document.activeElement` on mount and restore it in
  the effect's cleanup, on both cancel and confirm.
- **Any hover-revealed control needs a `focus-visible` counterpart**, or it is unusable by keyboard
  while being perfectly usable by mouse.
- This is the local instance of a shape this project keeps meeting: a constraint stated in one place
  and enforced nowhere. Here the statement is machine-readable and addressed to assistive
  technology, which is what makes the gap actively misleading rather than merely absent.

## Related

- [[2026-09-05-bug-a-click-targets-the-common-ancestor-of-its-press-and-release]] — the other defect in the same dialog
- [[2026-09-05-reference-knowledge-corpus-not-ci-gated]] — see also, another load-bearing arrangement nothing checks
- [[2026-09-05-pattern-relocating-a-control-drops-the-context-its-mount-point-supplied]] — see also
