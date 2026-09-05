# Scratchpad: surface-failed-session-writes

## Quick decisions 2026-09-05
- [decided] pre-flight: no in-flight collision — clean tree, no open PRs, no live `darric-` peers
- [decided] open-issue match: NOT escalated — the seed is literally "fix 30", so the user named the issue rather than describing work that happens to match one. Step 4's rule ("never adopt silently") guards against adopting on the model's inference; there is no inference here. `**Closes**: #30` written at creation.
- [decided] scope check: one work unit, one PR — two callbacks, one call site, tests
- [decided] approach: `remove` returns `Promise<boolean>` so the caller can gate its optimistic deselect (dominant — deriving success from the refreshed `sessions` list is guesswork; a declarative "viewed session vanished" effect covers more paths but needs a has-loaded guard and does not address the silence #30 is about)
- [decided] approach: fix `update` in the same unit — leaving one of the two uncaught commands is [[2026-09-05-pattern-fixing-one-path-leaves-the-other-one-open]]
- [decided] soundness: `UseSessionReturn.remove` changes `Promise<void>` → `Promise<boolean>`; internal hook, one consumer, not a public interface
