# jsdom fires no blur when a focused element is unmounted

**Date**: 2026-09-06
**Type**: constraint
**Summary**: jsdom fires no blur on unmount, so an exactly-once assertion cannot pin a blur-driven commit
**Context**: .minerva/work/2026-09-06-list-inline-rename (see git history if the worktree has been cleaned up)

## Context

The sidebar's inline rename editor closes on one path only: Enter and Escape call `blur()` on
the input, and the `onBlur` handler commits or discards. The design exists so a commit cannot
fire twice — once from Enter and once from the blur that follows — and the headline test
asserted `onRename` was called exactly once after Enter.

Review found that test green for the wrong reason. Chrome and WebKit dispatch `blur` when the
focused element is removed from the DOM; jsdom dispatches nothing (verified: zero `blur` and
`focusout` events on removal). A regression that made Enter commit directly and unmount the
input would still count one call under jsdom while the real Tauri webview sent two
`update_session` writes.

## Finding

Under jsdom, unmounting the focused element produces no `blur` or `focusout` event. Any test
that relies on the unmount-blur to surface a double commit, a leaked handler, or a
commit-on-close cannot observe it. An "exactly once" count on a blur-driven handler therefore
passes for both the intended design and the regression it was written against.

The mechanism has to be pinned directly: stub `HTMLElement.prototype.blur` to a no-op for one
case and assert that the key press alone does **not** commit and the editor stays open. With
blur inert, a design that commits on Enter still fires and a design that routes through blur
does nothing — which is the only observation that separates them.

## Implications

- A count assertion on a blur-driven commit is necessary but not sufficient in this suite;
  pair it with a blur-stub case whenever blur is the single close path.
- The same blindness covers cleanup on unmount: jsdom will not show that a handler ran, or
  failed to run, because focus left a removed element.
- Behaviour that depends on the unmount-blur in the real webview must be reasoned about from
  the code, not the tests; `2026-09-05-pattern-renderhook-reads-callbacks-fresh-so-stale-closures-cannot-fail`
  records the same class of test that passes against a reverted fix.

## Related

- [[2026-09-05-pattern-renderhook-reads-callbacks-fresh-so-stale-closures-cannot-fail]] — a test that stays green against the regression it exists to catch
- [[2026-09-06-constraint-user-event-setup-replaces-navigator-clipboard]] — another test-environment behaviour that silently differs from the browser
- [[2026-09-05-reference-matchmedia-stub-pins-tests-to-light-mode]] — the suite's third environment stub that hides a branch
