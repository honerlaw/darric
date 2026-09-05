# `phase_progress` misreads a shipped phase on a squash-merging repo

**Date**: 2026-09-05
**Type**: bug
**Summary**: `phasing.md` feeds `phase_progress()` from `git branch --merged`, which cannot see a squash-merged branch — so a phase that has shipped reads as pending and would be re-shipped forever
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

`minerva:ship` and `minerva:cleanup` decide which phase of a phased unit to act on by calling
`work_status.phase_progress(phases, merged_branches, slug)`. The snippet both skills are told to
use, in `plugins/minerva/skills/propose/references/phasing.md`, builds `merged_branches` like this:

```bash
git branch --merged <default-branch> --format='%(refname:short)'
```

## Finding

`git branch --merged` reports branches whose tip is an ancestor of the default branch. A
**squash** merge creates a single new commit on the default branch and leaves the source branch's
own commits unreferenced, so the branch is never an ancestor and never appears.

Observed directly on this repo: PR #7 shipped phase 1 and was squash-merged, after which

```
{'phased': True, 'total': 3, 'merged': 0, 'next_position': 1,
 'next_branch': '2026-09-05-strip-to-recorder'}
```

— a shipped phase reported as the next one to ship. An orchestrator following that would re-ship
phase 1 indefinitely and never reach phase 2.

`references/merge-detection.md` already knows about this: its per-worktree check queries
`gh pr list --head <branch> --state merged` _before_ falling back to `git branch --merged`,
precisely because the local check misses squash merges. The phasing snippet does not, so the two
halves of the same skill disagree.

A second, smaller trap compounds it: a phase branch **freshly cut and not yet committed to** is
identical to the default branch and therefore _is_ an ancestor, so it reads as already merged.
Phase resolution is only trustworthy once the phase has at least one commit.

## Implications

- Feed `phase_progress` from merged **pull requests** on any repo that squash-merges, not from
  `git branch --merged`. The `gh pr list --state merged` query in `merge-detection.md` is the
  shape to copy.
- Do not resolve a phase against a branch with zero commits; cut the branch, commit, then resolve.
- Until the snippet is corrected upstream, a phased unit on a squash-merging repo needs its
  merged-set computed by hand at ship time.

## Related

- [[2026-09-05-constraint-phases-must-use-the-canonical-list-form]] — the other way a phased unit silently stops being treated as phased
