# Anything GITHUB_TOKEN does in CI triggers no workflow, including this repo's reconciliation PR

**Date**: 2026-09-05
**Type**: reference
**Summary**: a pull request opened with `GITHUB_TOKEN` gets zero check runs and a merge it performs fires no `push` event, so `check.yml` never sees the knowledge-reconciliation PR nor the `main` it lands on — and adding a required status check to `main` would block those PRs forever
**Context**: .minerva/work/2026-09-05-reconcile-pr-auto-merge

## Context

`.github/workflows/knowledge-reconcile.yml` opens its reconciliation PR with
`GH_TOKEN: ${{ github.token }}`. GitHub deliberately does not create workflow runs for events
triggered by `GITHUB_TOKEN`, to stop workflows recursing into themselves.

Measured on PR #25, head `d68be90`:

```
gh api repos/:owner/:repo/commits/d68be90.../check-runs --jq .total_count   # 0
gh api repos/:owner/:repo/commits/d68be90.../status      --jq .total_count   # 0
```

## Finding

Both halves of the rule bite here, and they are easy to miss separately.

- **No checks on the PR.** `check.yml` triggers on `pull_request` with no path filter, so
  [[2026-09-05-reference-knowledge-wiki-is-ci-gated]] reads as though every PR is linted. That is
  true of every pull request except this one — and this is the only PR that rewrites `index.md`,
  the file the linter's index-drift error exists to protect.
- **No push event on merge.** When the workflow merges its own PR, `check.yml`'s
  `push: branches: [main]` run does not fire either. Back when a human clicked Merge, that run
  validated the merged tree; automating the merge silently removed it.

`knowledge-reconcile.yml` now runs `knowledge_lint.py` itself at both points — once on the branch
before merging, once on the squash commit afterwards — because nothing else will.

## Implications

- **Do not make `Knowledge Wiki` (or any job) a required status check on `main` while
  reconciliation PRs are opened with `GITHUB_TOKEN`.** A required check that can never report
  leaves every reconciliation PR blocked forever. See
  [[2026-09-05-pattern-auto-merge-on-a-pr-that-can-carry-no-checks]] for why the failure is
  silent rather than loud.
- A green `check.yml` on `main` says nothing about a commit that arrived via reconciliation. The
  reconcile job's own post-merge lint is the only signal for those.
- Escaping this needs a PAT or a GitHub App token instead of `GITHUB_TOKEN`; that is a real
  option, not a fix that has been applied.

## Related

- [[2026-09-05-reference-knowledge-wiki-is-ci-gated]] — the entry this qualifies: true for every pull request but the reconciliation one
- [[2026-09-05-pattern-auto-merge-on-a-pr-that-can-carry-no-checks]] — the merge-mechanism consequence of having no checks
