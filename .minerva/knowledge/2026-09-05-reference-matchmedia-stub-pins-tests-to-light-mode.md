# The `matchMedia` test stub is process-wide and pins every test to light mode

**Date**: 2026-09-05
**Type**: reference
**Summary**: `src/test/setup.ts` installs a never-restored `matchMedia` returning `matches: false` with a no-op change listener, so `App`'s dark branch and its listener lifecycle go unexercised
**Context**: .minerva/work/2026-09-05-model-download-progress (see git history if the worktree has been cleaned up)

## Context

jsdom implements no `window.matchMedia`, and `App` calls it on mount to follow the OS colour
scheme. Before a stub existed, no test could render `App` at all — the effect threw before any
assertion ran. `src/test/setup.ts` now installs one in `beforeAll`.

## Finding

The stub is installed once in `beforeAll` and **never restored**, so it is in effect for every
test file in the suite. It reports `matches: false` and its `addEventListener` /
`removeEventListener` are no-ops.

Two consequences:

- `App`'s colour-scheme effect is exercised only along its light branch. A change that inverted
  the `classList.toggle` argument would fail no test.
- The listener add/remove pair is never observed, so dropping the `removeEventListener` in the
  effect's cleanup — a real listener leak — would also fail no test.

This is a coverage blind spot the stub creates, not a defect in shipped behavior. It was
introduced deliberately: without it there is no `App`-level testing at all, which is a larger
loss than the branch it hides.

## Implications

- Any future work on theming or on `App`'s effect lifecycle needs to make the stub
  controllable (a settable `matches`, and listener spies) before its tests mean anything.
- Because the stub is global rather than per-file, a test that needs different behavior must
  override it explicitly; it cannot assume jsdom's absence of `matchMedia`.

## Related

- [[2026-09-06-constraint-user-event-setup-replaces-navigator-clipboard]] — see also
