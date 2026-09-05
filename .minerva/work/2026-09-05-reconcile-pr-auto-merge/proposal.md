# Proposal: reconcile-pr-auto-merge

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)

## Goal

A knowledge-reconciliation PR opened by `.github/workflows/knowledge-reconcile.yml` must land
on `main` by itself, and must land only if the corpus it rewrote still lints clean. Today it
does neither: the workflow opens the PR and stops, and a human clicks Merge on content no CI
job has looked at.

## Why

Two facts about these PRs, both observed on this repo today.

**Nobody merges them but the user, by hand.** Six reconciliation PRs merged on 2026-09-05
(#4, #8, #10, #12, #20, #25), every one of them merged by `honerlaw`. The workflow's last
action is `gh pr create`; there is no `gh pr merge` anywhere in it. `minerva:cleanup`'s own
reconciliation path *does* call `gh pr merge --auto --squash`, but cleanup stands down in this
repo — its Step 0 runs `grep -rl "knowledge_fix.py" .github/workflows/` and skips its whole
pass when it matches, so CI is the sole reconciliation writer and the one writer that never
enables auto-merge.

**They run zero CI.** GitHub does not trigger workflows for a pull request opened with
`GITHUB_TOKEN`, so `check.yml` — including its `Knowledge Wiki` job, the
`knowledge_lint.py` gate — never fires on a reconciliation branch. Measured on #25's head
`d68be90`: `check-runs` `total_count: 0`, combined status `total_count: 0`. Branch protection
on `main` has no required status checks at all (`GET .../protection/required_status_checks` →
404) and `required_approving_review_count: 0`.

Those two facts interact, and the interaction is why the fix is not a one-liner.
[[2026-09-05-reference-knowledge-wiki-is-ci-gated]] records that `knowledge_lint.py` runs on
every pull request. That is true of every PR *except this one* — and this is the only PR type
that rewrites `index.md`, the exact file the linter's index-drift error exists to protect.
Enabling auto-merge without adding a gate would remove the last human eye from the only
unlinted PR in the repo. Adding the gate the PR should have had makes the merge defensible.

## Approach

*Rewritten at promote to match what shipped. The original design — lint the whole corpus, then
`gh pr merge --auto` with a direct fallback — was replaced after review; `replan.md` records why.*

**`.github/workflows/knowledge-reconcile.yml` lints its own PR and merges it.** One workflow, one
added step, plus one line in `.gitignore`. No new permissions: `contents: write` and
`pull-requests: write` were already granted.

The job is a five-stage pipeline, and each stage is scoped to what it can actually know.

1. **Baseline** (new `Baseline knowledge lint` step, before `Apply mechanical fixes`). Lint
   `main`'s corpus and keep the sorted error lines in `${RUNNER_TEMP}/errors-baseline.txt`.
   Pre-existing errors are surfaced as a `::warning::` and never block. The step clears `errexit`
   explicitly — the runner's default shell sets it, and `set -uo pipefail` does not clear it — so
   nothing here can fail the run. A crash leaves an empty baseline, which can only make the gate
   below stricter, never looser.

2. **Gate on new errors only.** After the PR is opened, a `new_lint_errors()` helper lints again
   and emits `comm -13 baseline stage`. The job fails only on error lines *this* reconciliation
   introduced. A linter exit status above 1 is treated as a crash rather than a lint failure, and
   fails on its own. Gating on absolute cleanliness instead would let one inherited defect the
   fixer structurally refuses to repair — a broken `[[…]]` link, a duplicate id — block every
   future reconciliation forever
   ([[2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed]]).

3. **Merge directly.** `gh pr merge --squash "$PR_URL"` — deliberately not `--auto`. On a PR that
   can carry no checks, `--auto` succeeding is the bad outcome: it accepts a blocked PR, exits 0,
   and parks reconciliation forever behind a check that can never report, with the job green. A
   direct merge that branch protection rejects fails loudly instead
   ([[2026-09-05-pattern-auto-merge-on-a-pr-that-can-carry-no-checks]]). `$PR_URL` rather than
   `$BRANCH` also drops gh's head-ref-to-PR lookup.

4. **Verify what actually landed.** Read the squash commit's oid from the merged PR, fetch that
   oid, check it out, and lint it against the same baseline. Not `main`'s tip: the tip may be a
   later unrelated merge, which would blame this run for someone else's defect, or — from a
   lagging read — the pre-merge commit the baseline was taken on, which passes trivially and
   verifies nothing. This restores the post-merge signal that automating the merge removed, since
   a `GITHUB_TOKEN` merge fires no `push` event
   ([[2026-09-05-reference-github-token-actions-trigger-no-workflows]]). It reports drift; it does
   not prevent it.

5. **Branch ref unique per attempt.** `minerva/reconcile-ci/${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}`.
   `GITHUB_RUN_ID` alone is stable across "Re-run failed jobs", and stage 4 makes a red job after a
   successful merge possible — so a re-run would otherwise reuse the failed attempt's branch and
   die at `gh pr create` with "No commits between main and …".

`.gitignore` gains `.minerva-tools/`: it is where CI checks out the pinned minerva tooling, and if
it were ever committed, stage 4's checkout would abort with "untracked working tree files would be
overwritten" — after the merge had already landed.

**Constraint carried through unchanged**: the workflow still contains the literal
`knowledge_fix.py`, because `minerva:cleanup`'s stand-down grep is what stops cleanup and CI from
both opening index-rewriting PRs and racing on `index.md`. The change adds a second occurrence
(`knowledge_lint.py`) and removes none.

### A deliberate divergence, named

`minerva:cleanup`'s `references/reconciliation.md` says that if `gh pr merge --auto` is rejected,
the run must "report the PR URL and stop rather than merging another way". That rule binds
`minerva:cleanup`, which stands down in this repo, and its stated rationale is that "a human
merging it at their convenience is a correct outcome". The repo owner merged 6/6 of these by hand
on 2026-09-05 and asked for them to stop needing that. The direct merge is that choice, applied to
a PR type whose content is regenerated by a pinned script rather than authored — and gated by a
linter run the rule's author assumed CI was already applying.

## Success criteria

Replaced wholesale by the review — see `replan.md`, "the merge gate was scoped wrong in three
directions".

1. A `Baseline knowledge lint` step records `main`'s pre-existing lint errors before
   `knowledge_fix.py` runs, reports them as a `::warning::`, and does not block on them.
2. After the PR is opened, the job lints the reconciled corpus and fails **only** on error lines
   absent from the baseline, with a `::error::` annotation naming the PR, leaving it open and
   unmerged.
3. A `knowledge_lint.py` exit status greater than 1 fails the job as a crash, distinctly from a
   lint failure.
4. On a clean gate the job merges with `gh pr merge --squash "$PR_URL"`, and the file contains
   no `gh pr merge --auto` invocation.
5. After merging, the job checks out the squash commit **by oid** — not the default branch tip —
   and lints it against the same baseline, failing with a `::error::` that states the merge
   already landed if new errors are present.
10. The reconciliation branch ref is unique per *attempt*, not per run, so "Re-run failed jobs"
    cannot collide with the branch of the attempt that failed.
11. The baseline step cannot fail the run: it clears `errexit` explicitly rather than relying on
    `set -uo pipefail` to do so.
6. The "nothing pending" early exit is unchanged — no PR, no gate, no merge, exit 0.
7. The workflow still contains the literal `knowledge_fix.py`, keeping `minerva:cleanup`'s
   stand-down grep matching.
8. The workflow parses as YAML, every step's script body is `bash -n`-clean, and no unguarded
   non-zero exit turns a successful reconciliation red.
9. No comment in the file asserts that `check.yml` does not run the linter, or that a post-fix
   corpus is warning-free.

## Open Questions

None blocking. The original open question — whether GitHub would accept `--auto` on a check-less
PR or refuse it as clean — was dissolved rather than answered: `--auto` is gone, because on this
PR type "accepted" is the worse of the two outcomes.
