# A gate inside an automated pipeline must fail on what that pipeline changed, not on absolute state

**Date**: 2026-09-05
**Type**: pattern
**Summary**: gating the reconciliation job on "the corpus lints clean" would have let a single pre-existing error the fixer cannot repair block every future reconciliation forever; gating on "no error this run introduced" — a baseline diffed with `comm -13` — keeps the pipeline running while still failing closed
**Context**: .minerva/work/2026-09-05-reconcile-pr-auto-merge

## Context

`knowledge-reconcile.yml` merges its own PR, so it needed a gate to stand in for the CI that
never runs on it. The first version was the obvious one: run `knowledge_lint.py`, fail the job on
any error.

## Finding

That gate has a failure mode the "clean corpus" framing hides: **the pipeline cannot repair every
condition the gate rejects.** `knowledge_fix.py` explicitly refuses broken `[[…]]` links and
anything touching a duplicate id, and both are error-severity in `knowledge_lint.py`. `main` has
no required status checks, so a PR with a red `Knowledge Wiki` job can be merged by hand and put
the corpus in exactly that state.

From that moment, every knowledge push would open a reconciliation PR, run the fixer, and fail
the gate on an unrelated inherited defect. The index stops updating, dead PRs accumulate, and the
blocking error has nothing to do with the change being blocked. Before the gate existed, the PR
merged and the index stayed current — so the gate would have been a regression dressed as
hardening.

The fix is to scope the assertion to the delta:

1. lint **before** the fixer runs and keep the sorted error lines as a baseline;
2. lint again after, and `comm -13 baseline after`;
3. fail only on lines the second run added.

Pre-existing errors are surfaced as a `::warning::` and never block. A crash in the baseline step
leaves an empty baseline — which can only make the gate **stricter**, never looser, so the
degraded mode still fails closed.

## Implications

- Before adding a gate to a pipeline that runs unattended, ask what happens on the day it fires
  for a reason the pipeline cannot fix. If the answer is "the pipeline stops forever", the gate
  is scoped to absolute state and should be scoped to the delta instead.
- Baseline-and-diff is the general shape, and it wants a bias: choose the degradation that
  tightens the gate rather than loosening it, so a failure of the gate's own machinery cannot
  wave a defect through.
- The same review found a fail-open hazard in the helper implementing this — a stage label of
  `before` would have overwritten the baseline and compared it against itself, passing
  unconditionally. Namespacing the files apart is cheap; a fail-open path inside a fail-closed
  gate is not.

## Related

- [[2026-09-05-pattern-auto-merge-on-a-pr-that-can-carry-no-checks]] — the other scoping error found in the same review
- [[2026-09-05-reference-github-token-actions-trigger-no-workflows]] — why this job has to carry its own gate at all
