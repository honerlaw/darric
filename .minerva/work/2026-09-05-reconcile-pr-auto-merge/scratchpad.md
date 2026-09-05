# Scratchpad: reconcile-pr-auto-merge

## Quick decisions 2026-09-05

- [decided] pre-flight: no collision. Adjacent — local unit `2026-09-05-session-scoped-ui-state` (peer `darric-64` confirmed `MINERVA-BUSY`, diff is src/ + src-tauri/ only, nothing under `.github/workflows/`). Stale — `minerva/reconcile-ci/{33976970204,33980496074,33984364842}` are residue of merged PRs #8/#10/#12. `darric-a5` replied `MINERVA-IDLE`. No open issue matches the seed.
- [decided] scope check: single unit, one PR, one file (`.github/workflows/knowledge-reconcile.yml`). No phasing — well inside the quick path.
- [decided] approach: lint-then-merge inside the reconcile job, `--auto` with a direct-squash fallback. Dominant over bare `gh pr merge --auto --squash` (measured: 0 check-runs and 0 statuses on PR #25's head `d68be90`, no required status checks on `main` → GitHub likely refuses `--auto` as clean, making the literal fix a silent no-op; and it would ship ungated auto-merge on the repo's only unlinted PR). Rejected direct-push-to-main: `required_pull_request_reviews` blocks it and minerva's reconciliation contract forbids it.
- [decided] whole-proposal soundness: no public interface, blast radius is one CI workflow in the user's own repo, revertable in one commit. The divergence from `minerva:cleanup`'s "do not merge another way" rule is named in the proposal rather than glossed — that rule binds cleanup, which stands down here, and the owner has merged 6/6 of these by hand today.

## Notes

- `minerva:cleanup` Step 0 stand-down anchor is `grep -rl "knowledge_fix.py" .github/workflows/` (cleanup `references/reconciliation.md:38`). The workflow must keep that literal or cleanup and CI both start opening index-rewriting PRs and race on `index.md`. Flagged independently by peer `darric-64`.
- `knowledge_lint.py` `main()` returns `1 if errors else 0` — warnings (uncatalogued entry, missing reciprocal) do not fail. Those are exactly what `knowledge_fix.py` repairs, so a post-fix corpus should lint clean.
