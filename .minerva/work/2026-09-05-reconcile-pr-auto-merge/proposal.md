# Proposal: reconcile-pr-auto-merge

**Date**: 2026-09-05
**Status**: Draft

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

**Lint the reconciled corpus inside the job, then merge, in
`.github/workflows/knowledge-reconcile.yml`.** One file, one job, no new workflow and no new
permissions — `contents: write` and `pull-requests: write` are already granted.

1. **Gate.** After `gh pr create`, run
   `python .minerva-tools/plugins/minerva/scripts/knowledge_lint.py .minerva/knowledge` against
   the branch's corpus. This is the same invocation `check.yml`'s `Knowledge Wiki` job makes,
   run here because GitHub will not run it there. The linter exits non-zero on errors only
   (duplicate ids, index drift, broken `[[…]]` wikilinks) and exits 0 on warnings
   (uncatalogued entry, missing reciprocal) — which are precisely the conditions
   `knowledge_fix.py` just repaired, so a clean run is the expected outcome.

2. **Merge.** `gh pr merge --auto --squash "$BRANCH"`, falling back to a direct
   `gh pr merge --squash "$BRANCH"` when GitHub refuses to enable auto-merge. The fallback is
   not belt-and-braces: with no required status checks and no required reviews there is
   nothing for auto-merge to wait on, and GitHub rejects
   `enablePullRequestAutoMerge` on a pull request that is already in a clean state. Trying
   `--auto` first is still the right order — it is the branch-protection-respecting form, so if
   required checks are ever added to `main` the workflow starts honouring them with no further
   edit.

3. **Failure is loud and leaves the PR.** If the linter errors, the step emits a `::error::`
   annotation and exits non-zero *after* the PR exists. The reconciliation work is preserved
   for a human to inspect and merge or fix; the red job is the notification. This is the one
   path that still ends with a PR awaiting a human, and it should.

**Constraint carried through unchanged**: the workflow must keep containing the literal string
`knowledge_fix.py`, because `minerva:cleanup`'s stand-down grep is what stops cleanup and CI
from both opening index-rewriting PRs and racing on `index.md`. The change adds a second
occurrence (`knowledge_lint.py`) and removes none.

### Rejected alternatives

- **`gh pr merge --auto --squash` alone** — the literal reading of the request, and what
  `minerva:cleanup` does. Rejected on the measured evidence above: with nothing blocking these
  PRs, `--auto` is likely to be refused outright, leaving the user still merging by hand while
  the workflow reports success. It also ships an ungated auto-merge on the repo's only unlinted
  PR.
- **Commit the reconciliation straight to `main`, no PR** — rejected. `main` carries
  `required_pull_request_reviews`, so a direct push is blocked; and minerva's reconciliation
  contract is explicit that reconciliation always goes through its own PR.

### A deliberate divergence, named

`minerva:cleanup`'s `references/reconciliation.md` says that if `gh pr merge --auto` is
rejected, the run must "report the PR URL and stop rather than merging another way". That rule
binds `minerva:cleanup`, which does not run here, and its stated rationale is that "a human
merging it at their convenience is a correct outcome". The repo owner has merged 6/6 of these
by hand today and is asking for them to stop needing that. The direct-merge fallback is that
choice, applied to a PR type whose content is regenerated by a pinned script rather than
authored — and it is gated by a linter run the rule's author assumed CI was already applying.

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
