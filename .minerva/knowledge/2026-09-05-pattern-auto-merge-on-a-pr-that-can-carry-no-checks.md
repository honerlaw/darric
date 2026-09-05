# On a PR that can carry no checks, `--auto` being _accepted_ is the failure mode

**Date**: 2026-09-05
**Type**: pattern
**Summary**: `gh pr merge --auto` exiting 0 means auto-merge was enabled, not that anything merged — so on a PR whose required checks can never report it parks the PR forever while the job goes green, where a direct merge would have failed loudly
**Context**: .minerva/work/2026-09-05-reconcile-pr-auto-merge

## Context

`minerva:cleanup`'s reconciliation path ends in `gh pr merge --auto --squash`, and copying that
into `knowledge-reconcile.yml` was the obvious way to make its PR merge itself. The stated
reason for preferring `--auto` over a direct merge was that it is the branch-protection-respecting
form: if required checks were ever added to `main`, the workflow would start honouring them
without further edits.

That reasoning is backwards for this PR type, and the review caught it before it shipped.

## Finding

`--auto` is designed to accept a **blocked** PR — that is its whole purpose — and it returns 0 on
having _enabled_ auto-merge, not on having merged. The two diverge permanently when the PR's
checks can never arrive, which is exactly the case for a PR opened with `GITHUB_TOKEN`
([[2026-09-05-reference-github-token-actions-trigger-no-workflows]]).

Add a required status check to `main` and the next reconciliation PR sits at "Expected — waiting
for status to be reported" forever. `enablePullRequestAutoMerge` succeeds, `gh` exits 0, the job
prints "auto-merge enabled" and goes **green**, and nothing merges again. Each subsequent push
adds another orphan PR and branch. Raising `required_approving_review_count` above 0 does the
same, since a `GITHUB_TOKEN` PR cannot self-approve.

A plain `gh pr merge --squash` against that same blocked PR gets a 405, exits non-zero, and fails
the job. That is the better outcome by a wide margin: the wedge is identical either way, but one
version announces itself and the other reports success while reconciliation silently stops.

## Implications

- Prefer `--auto` when a PR's checks will genuinely report and you want to wait for them. Prefer a
  direct merge when the PR structurally cannot produce the checks it would be waiting on.
- Treat "the more conservative-looking flag" as a claim to test, not a default. `--auto` _looks_
  like the safe choice because it defers to branch protection; the deference is what makes it
  fail silently here.
- More generally: a command whose success means "a future action is scheduled" is not
  interchangeable with one whose success means "the action happened". If nothing later verifies
  the outcome, only the second kind can be trusted to report failure.

## Related

- [[2026-09-05-reference-github-token-actions-trigger-no-workflows]] — why these PRs can never carry a check for `--auto` to wait on
- [[2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed]] — the other scoping error found in the same review
