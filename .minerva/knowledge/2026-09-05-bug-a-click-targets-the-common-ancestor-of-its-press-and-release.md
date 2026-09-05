# A click targets the common ancestor of its press and its release, so a backdrop cannot use it to mean "outside"

**Date**: 2026-09-05
**Type**: bug
**Summary**: the confirmation modal dismissed on a backdrop `click`, but selecting the dialog's own body text and releasing over the dim area dispatches that click on the backdrop — closing the dialog mid-selection, past a `stopPropagation` that could not see it, while a test claimed the case was covered
**Context**: .minerva/work/2026-09-05-delete-confirm-and-nav-resume

## Context

`ConfirmDialog` gates every recording delete. It is a `fixed` backdrop with the panel nested
inside, and it dismissed the way click-outside is normally written:

```tsx
<div className="fixed inset-0 …" onClick={onCancel}>
  <div role="dialog" onClick={(e) => { e.stopPropagation(); }}>
```

A test asserted the containment held, and passed:

```tsx
await user.click(screen.getByText(/cannot be undone/));
expect(screen.getByRole("dialog")).toBeInTheDocument();
```

## Finding

**A `click` event's target is not where the mouse went down. It is the nearest common ancestor
of the `mousedown` target and the `mouseup` target.**

So press on the dialog's body text, drag left to select it, release over the dim area:
`mousedown` → the `<p>`, `mouseup` → the backdrop, and the browser dispatches one `click` **on
the backdrop**. The panel's handler never runs, because the event never passes through the
panel. The dialog vanishes while the user is selecting its text.

Two things made this survive review-by-reading:

- **`stopPropagation` is structurally unable to catch it.** It only helps for events that
  propagate _through_ the panel, and this one does not.
- **The test asserted the guarantee, not the behaviour.** `user.click` is a stationary
  press-and-release on one element, which the `stopPropagation` _did_ handle. The test's comment
  described the drag case; the test performed a different one. It passed for a reason other than
  the one it was written for.

The obvious repair is also wrong: `if (e.target === e.currentTarget) onCancel()` on the _click_
still cancels, because in the drag case the target genuinely **is** the backdrop.

The fix is to key dismissal to where the gesture **started** — `onMouseDown` on the backdrop with
`e.target === e.currentTarget`. `stopPropagation` on the panel then becomes unnecessary and was
removed.

## Implications

- **Click-outside means "the press started outside", not "the click resolved outside".** Any
  dismiss-on-backdrop should listen on `mousedown`/`pointerdown`, not `click`. This applies to
  menus, popovers and drawers as much as to modals.
- **A containment test must move the mouse.** Asserting with a stationary `user.click` inside the
  panel cannot distinguish a correct implementation from this one. Drive it with separate
  `fireEvent.mouseDown` / `mouseUp` / `click` on different elements.
- Generalising: when a test's comment names a scenario the test's _actions_ do not perform, the
  suite records an intention rather than a guarantee. That gap is invisible while everything is
  green — see [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] for
  the same shape asked of a code path rather than an assertion.

## Related

- [[2026-09-05-constraint-aria-modal-promises-inertness-that-nothing-enforces]] — the other defect in the same dialog, also a promise the code did not keep
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — see also
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — see also
