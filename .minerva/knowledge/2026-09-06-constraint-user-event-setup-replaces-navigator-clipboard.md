# `userEvent.setup()` replaces `navigator.clipboard`, discarding any stub installed before it

**Date**: 2026-09-06
**Type**: constraint
**Summary**: `@testing-library/user-event` installs its own clipboard on `setup()`, so a `writeText` mock from `beforeEach` is never called — spy on `navigator.clipboard.writeText` after `setup()` instead
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

`McpChip` copies the `claude mcp add` command with `navigator.clipboard.writeText`. jsdom has
no clipboard, so the test installed one with `Object.defineProperty(navigator, "clipboard",
{ value: { writeText: vi.fn() } })` in `beforeEach`, clicked the chip with user-event, and
asserted the mock was called. It never was: zero calls, while the chip visibly flipped to
"Copied", so the write had gone somewhere.

## Finding

`userEvent.setup()` attaches user-event's own `Clipboard` implementation to `navigator`
(it supports `readText`/`writeText` so that copy/paste interactions can be simulated), replacing
whatever was there. A stub installed earlier in the same test is silently discarded. The
working pattern is to take the spy after setup:

```ts
const user = userEvent.setup();
const clipboardWrite = vi.spyOn(navigator.clipboard, "writeText");
await user.click(chip);
expect(clipboardWrite).toHaveBeenCalledWith(/* … */);
```

A test that triggers the click without user-event — `chip.click()` under fake timers — keeps
the `beforeEach` stub and works either way.

## Implications

- Any component test asserting a clipboard write through user-event must spy after
  `setup()`; a `beforeEach` stub alone produces a false failure that looks like the component
  never wrote.
- The same applies to anything else user-event installs on setup; check its `setup` options
  before stubbing a navigator API.

## Related

- [[2026-09-05-reference-matchmedia-stub-pins-tests-to-light-mode]] — the other navigator-level stub in this suite whose lifecycle is easy to misread
- [[2026-09-06-constraint-jsdom-fires-no-blur-on-unmount]] — see also
