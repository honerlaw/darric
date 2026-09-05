# Scratchpad: reconcile-pr-auto-merge

## Quick decisions 2026-09-05

- [decided] pre-flight: no collision. Adjacent — local unit `2026-09-05-session-scoped-ui-state` (peer `darric-64` confirmed `MINERVA-BUSY`, diff is src/ + src-tauri/ only, nothing under `.github/workflows/`). Stale — `minerva/reconcile-ci/{33976970204,33980496074,33984364842}` are residue of merged PRs #8/#10/#12. `darric-a5` replied `MINERVA-IDLE`. No open issue matches the seed.
- [decided] scope check: single unit, one PR, one file (`.github/workflows/knowledge-reconcile.yml`). No phasing — well inside the quick path.
- [decided] approach: lint-then-merge inside the reconcile job, `--auto` with a direct-squash fallback. Dominant over bare `gh pr merge --auto --squash` (measured: 0 check-runs and 0 statuses on PR #25's head `d68be90`, no required status checks on `main` → GitHub likely refuses `--auto` as clean, making the literal fix a silent no-op; and it would ship ungated auto-merge on the repo's only unlinted PR). Rejected direct-push-to-main: `required_pull_request_reviews` blocks it and minerva's reconciliation contract forbids it.
- [decided] whole-proposal soundness: no public interface, blast radius is one CI workflow in the user's own repo, revertable in one commit. The divergence from `minerva:cleanup`'s "do not merge another way" rule is named in the proposal rather than glossed — that rule binds cleanup, which stands down here, and the owner has merged 6/6 of these by hand today.

## Notes

- `minerva:cleanup` Step 0 stand-down anchor is `grep -rl "knowledge_fix.py" .github/workflows/` (cleanup `references/reconciliation.md:38`). The workflow must keep that literal or cleanup and CI both start opening index-rewriting PRs and race on `index.md`. Flagged independently by peer `darric-64`.
- `knowledge_lint.py` `main()` returns `1 if errors else 0` — warnings (uncatalogued entry, missing reciprocal) do not fail. Those are exactly what `knowledge_fix.py` repairs, so a post-fix corpus should lint clean.

## Review finding 2026-09-05

- [minerva audit / SUGGEST] `[[2026-09-05-reference-knowledge-wiki-is-ci-gated]]` says the `Knowledge Wiki` job "runs on every pull request and every push to main, whether or not the change touches `.minerva/`". That is a statement about `check.yml`'s config, and it is true of the config — but GitHub does not trigger workflows for pull requests opened with `GITHUB_TOKEN`, so the one PR type that rewrites `index.md` receives no checks at all. Measured on PR #25 head `d68be90`: `check-runs.total_count: 0`, combined status `total_count: 0`. Standing fact about the system → new `reference` entry at promote, cross-linked to the existing one. Not an edit to that entry: promote is add-only and the entry is not wrong about what it actually claims.
- [minerva audit] Spec fidelity: diff matches the proposal's Approach 1:1. No divergence, no replan.
- [minerva audit] `CLAUDE.md` linting policy: compliant — the change adds a gate rather than suppressing one. No `.minerva/reference/` exists in this repo, so the workflow header comment is the only doc surface, and it is updated in the diff.

## Notes

- End-to-end validation path: this unit's own promote adds `.minerva/knowledge/` entries, so merging its PR pushes to `main` under the workflow's `paths` filter and fires the _new_ reconcile job. That run is the first real answer to whether GitHub accepts `gh pr merge --auto` on a check-less PR or refuses it as clean — watch its log in Phase 7 for which branch of the `if` it took.

## Review triage 2026-09-05

Code review (fresh-context subagent; no PR existed yet) returned six findings. All six triaged **FIX** — three of them load-bearing, which triggered the replan recorded in `replan.md`.

| #   | Finding                                                                                                                                      | Disposition                                                        |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| 1   | `--auto` returning 0 means "enabled", not "merged"; a required check added to `main` wedges reconciliation forever while the job stays green | FIX — load-bearing → replan                                        |
| 2   | Gating on _any_ lint error lets one pre-existing unfixable defect block every future reconciliation                                          | FIX — load-bearing → replan                                        |
| 3   | Fallback branch hard-codes a false diagnosis; `--auto`-then-direct has a false-red double-merge window                                       | FIX — subsumed by #1's removal of `--auto`                         |
| 4   | Merging with `GITHUB_TOKEN` suppresses `check.yml`'s push-to-main run, so the merged tree is never validated                                 | FIX — load-bearing → replan                                        |
| 5   | `gh pr merge "$BRANCH"` re-resolves head-ref→PR when `$PR_URL` is already in hand                                                            | FIX                                                                |
| 6   | Two factually wrong comments (three warning families not two; the linter _does_ run in `check.yml`)                                          | FIX — documentation for behavior this diff touched, never deferred |

Both F6 claims verified in source before accepting: `reciprocal-manual` is a third warning family (`knowledge_lint.py:432`) that `knowledge_fix.py:293` refuses on every run by design; `check.yml`'s `Knowledge Wiki` job does run the linter on every PR but this one.

Gate logic verified locally against a mocked linter across six scenarios — clean/clean → merge; pre-existing error unchanged → merge (the F2 wedge, now absent); new error → block; pre-existing + new → block naming only the new line; linter `rc=2` → crash exit; empty baseline + pre-existing error → block (stricter, never looser).

Reviewer's aside, noted and correct: the workflow edit was uncommitted at review time, so `git diff main...HEAD` showed only the work-unit files. It is committed at ship.

### Second review round

The reviewer re-read the rewrite, confirmed all six original fixes (including re-deriving F2's premise from the pinned scripts and running the helper's status propagation under `bash -euo pipefail` for rc 0/1/2), and returned six new findings. All six FIX; none changed the approach, so no second replan — criteria 5, 10 and 11 were sharpened instead.

| #   | Finding                                                                                                                                                                                                            | Fix                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| 1   | `FETCH_HEAD` is main's tip, not this run's merge commit — blames a third party's defect on this PR, or verifies a pre-merge tree and reports "verified" (false green on the exact drift the step exists to catch)  | Capture `mergeCommit.oid`, `git fetch --depth=1 origin <oid>`, check out the oid. Guarded for an unreadable oid |
| 2   | `GITHUB_RUN_ID` is stable across "Re-run failed jobs", so a re-run reuses the failed attempt's branch — 422 at `gh pr create`, or a non-fast-forward push                                                          | `${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}`                                                                        |
| 3   | `set -uo pipefail` does not clear the runner's inherited `errexit`, so the baseline step's "a crash here cannot fail the run" comment was unenforced                                                               | Explicit `set +e` — verified under `bash -e -o pipefail`                                                        |
| 4   | `errors-$1.txt` shared a namespace with `errors-before.txt`; a stage label of `before` would compare the baseline against itself and pass **unconditionally** — fail-open, in a helper whose job is to fail closed | `errors-baseline.txt` / `errors-stage-$1.txt`                                                                   |
| 5   | Post-merge crash branch never said the merge had already landed, inviting the re-run that trips #2                                                                                                                 | Caller-side `::error::` naming merge status; annotations moved out of the helper onto stdout                    |
| 6   | `.minerva-tools/` untracked but not ignored — if committed, the post-merge checkout aborts                                                                                                                         | Added to `.gitignore`                                                                                           |

The reviewer's one unsettled item (whether the runner parses `::error::` from stderr) was dissolved rather than researched: the helper now emits plain text to stderr and every `::error::` is raised by a caller on stdout, which also fixed #5.

## Promote partition 2026-09-05

**PROMOTE (3)** — `2026-09-05-reference-github-token-actions-trigger-no-workflows`, `2026-09-05-pattern-auto-merge-on-a-pr-that-can-carry-no-checks`, `2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed`. Written add-only; `index.md` and `overview.md` untouched, per the promote/reconcile split.

**MERGE INTO PROPOSAL** — the shipped five-stage design replaced the original `## Approach`; Status set to `Shipped (2026-09-05)`.

**DISCARD** — the `Quick decisions` log, the local gate harness, and the `GITHUB_RUN_ID`-is-stable fact (captured in the code comment at its point of use, which is where a future editor will need it).

**TODO — none filed.** Four candidates were checked against the deferral bar and none has a writable failure scenario:

- _Use a PAT/App token so reconcile PRs get real CI._ The only `check.yml` job relevant to a `.minerva/knowledge/**`-only diff is `Knowledge Wiki`, which this job now runs itself. Recorded as an option in the reference entry rather than deferred as work.
- _`overview.md` is still not refreshed by CI._ Pre-existing, documented in the workflow header, untouched by this change.
- _Post-merge drift is reported but not repaired._ Verified it self-heals: `knowledge_fix.py` removes stale catalog lines on the next reconcile run (docstring line 13, `apply` summary line 404).
- _A pre-existing error the baseline absorbs could go unnoticed._ It cannot go quiet: broken links and duplicate ids are error-severity, and `check.yml`'s `Knowledge Wiki` job runs on every ordinary PR, so the next one goes red until it is fixed.
