# Replan: reconcile-pr-auto-merge

## 2026-09-05 — the merge gate was scoped wrong in three directions

### Original plan

After `gh pr create`, run `knowledge_lint.py` on the branch's corpus and fail the job on any
error; then `gh pr merge --auto --squash "$BRANCH"`, falling back to a direct
`gh pr merge --squash "$BRANCH"` if GitHub refuses to enable auto-merge. `--auto` was ordered
first on the reasoning that it is "the branch-protection-respecting form", so the workflow would
start honouring required checks by itself if any were ever added to `main`.

### What changed

Review found that each of the three scopes is wrong, and the `--auto` rationale is backwards.

1. **`--auto` returning 0 means "auto-merge enabled", not "merged" — and here those diverge
   permanently.** Reconciliation PRs carry zero checks *by construction* (GitHub does not run
   workflows for `GITHUB_TOKEN`-opened PRs). Add a required status check to `main` — exactly the
   change the original comment invited — and the PR sits at "Expected — waiting for status to be
   reported" forever: `enablePullRequestAutoMerge` is *accepted* (a blocked PR is what it is
   for), `gh` exits 0, the job prints "auto-merge enabled" and goes green, and the fallback never
   runs. Reconciliation stops with no signal while orphan PRs and branches accumulate. Raising
   `required_approving_review_count` above 0 does the same, since a `GITHUB_TOKEN` PR cannot
   self-approve. A required check that can never report is a permanent wedge, not graceful
   honouring — the property `--auto` was chosen for does not exist on this PR type.

2. **Failing on *any* lint error is a regression, not a gate.** `knowledge_fix.py` structurally
   refuses to repair broken `[[…]]` links (`knowledge_fix.py:293`) and anything touching a
   duplicate id; both are error-severity in `knowledge_lint.py` (`broken-link`, `id`). `main` has
   no required status checks, so a red `Knowledge Wiki` job can be merged by hand and leave the
   corpus in precisely that state. From then on every knowledge push opens a PR and fails the
   gate on an unrelated pre-existing defect — the index stops updating and dead PRs pile up,
   where before this change the PR merged by hand and the index stayed current. The gate must
   fail on errors *this reconciliation introduced*, not on errors it inherited.

3. **Merging with `GITHUB_TOKEN` suppresses `check.yml`'s `push: branches:[main]` run.** That
   run is what validated the merged tree when a human clicked Merge. The in-job lint sees the
   pre-merge branch, which is not the same tree if `main` moved in between: an unrelated PR
   deleting a knowledge entry that this reconciliation just catalogued produces index drift on
   `main` with nothing left to report it.

A fourth, smaller correction: two comments were factually wrong — there are **three** warning
families, not two (`reciprocal-manual` is one the fixer refuses on every run, by design,
forever), and the header implied `check.yml` does not run the linter, when it does for every PR
but this one.

### New plan

The same job and the same file; the step becomes an explicit pipeline whose stages are each
scoped to what they can actually know.

1. **Baseline.** A new `Baseline knowledge lint` step, before `Apply mechanical fixes`, lints
   `main`'s corpus and keeps the sorted error lines in `${RUNNER_TEMP}`. Pre-existing errors are
   surfaced as a `::warning::` and never block. A crash here yields an empty baseline, which can
   only make the gate below stricter, never looser.

2. **Gate on new errors only.** After the fix and after the PR is opened, lint again and take
   `comm -13 before after`. Fail only on error lines this reconciliation added. A linter exit
   status above 1 is a crash rather than a lint failure and fails the job on its own.

3. **Merge directly.** `gh pr merge --squash "$PR_URL"` — no `--auto`. If branch protection ever
   rejects the merge, the command fails and the job goes red, which is visible; that is strictly
   better than `--auto` parking the PR behind a check that can never report. `$PR_URL` rather
   than `$BRANCH` also drops gh's head-ref-to-PR lookup, which is a resolution step with a
   failure mode and no benefit when the exact URL is already in hand.

4. **Verify what actually landed.** Fetch and check out the merged default branch and lint it,
   scoped against the same baseline. This restores the post-merge `Knowledge Wiki` signal the
   `GITHUB_TOKEN` merge suppresses. It cannot prevent the drift — it reports it, which is what
   the suppressed `check.yml` run did.

5. Comments corrected per the fourth point above.

### Success criteria — replacing the original list

1. A `Baseline knowledge lint` step records `main`'s pre-existing lint errors before
   `knowledge_fix.py` runs, reports them as a `::warning::`, and does not block on them.
2. After the PR is opened, the job lints the reconciled corpus and fails **only** on error lines
   absent from the baseline, with a `::error::` annotation naming the PR, leaving it open and
   unmerged.
3. A `knowledge_lint.py` exit status greater than 1 fails the job as a crash, distinctly from a
   lint failure.
4. On a clean gate the job merges with `gh pr merge --squash "$PR_URL"` and contains no
   `--auto`.
5. After merging, the job checks out the merged default branch and lints it against the same
   baseline, failing with a `::error::` if the merge landed new errors.
6. The "nothing pending" early exit is unchanged — no PR, no gate, no merge, exit 0.
7. The workflow still contains the literal `knowledge_fix.py`, keeping `minerva:cleanup`'s
   stand-down grep matching.
8. The workflow parses as YAML, every step's script body is `bash -n`-clean, and no unguarded
   non-zero exit turns a successful reconciliation red.
9. No comment in the file asserts that `check.yml` does not run the linter, or that a post-fix
   corpus is warning-free.
