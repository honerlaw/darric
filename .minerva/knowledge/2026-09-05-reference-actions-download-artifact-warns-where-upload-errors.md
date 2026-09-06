# `download-artifact` warns on an empty match where `upload-artifact` can be made to error

**Date**: 2026-09-05
**Type**: reference
**Summary**: `actions/upload-artifact@v4` takes `if-no-files-found: error` but `actions/download-artifact@v4` has no equivalent — a `pattern` matching nothing is a warning and the step still succeeds, so a guard on the upload side does not carry to the download side
**Context**: .minerva/work/2026-09-05-release-on-merge

## Context

`release.yml` builds a `.dmg` on one native runner per architecture, uploads each under
`dmg-<arch>`, and a later job pulls both back with:

```yaml
- uses: actions/download-artifact@v4
  with:
    pattern: dmg-*
    merge-multiple: true
    path: dmg
```

The upload side is guarded — `if-no-files-found: error`, so a leg that bundled nothing fails
rather than contributing an empty release.

## Finding

The guard does not survive the handoff, and the asymmetry is easy to read past because the two
actions are named as a pair.

`pattern: dmg-*` matching **one** artifact instead of two is not an error. The step warns, exits
0, and the job continues with half the files it expected. Downstream, `gh release upload "$tag"
dmg/*.dmg` finds one DMG, uploads it, and also exits 0 — a fully green run that publishes a
release for one architecture, with the only signal a warning line in a collapsed log group.

This is reachable by the pipeline's own documented recovery path: re-running just the release job
is what the comments tell you to do after a partial failure, and artifacts expire (90 days by
default) while a release does not. Re-run late enough and one artifact is gone.

The fix is an explicit count, because there is no input to set:

```sh
found=$(find dmg -type f -name '*.dmg' | wc -l | tr -d ' ')
if [ "$found" -ne 2 ]; then
  echo "::error::expected 2 DMGs, found $found"
  exit 1
fi
```

## Implications

- Where a job consumes artifacts produced by a matrix, assert the **count** you expect. The
  producing side's `if-no-files-found` says nothing about what the consuming side received.
- Re-running one job of a workflow is a different input state from the original run — artifacts
  may have expired, and a step that reads "whatever is there" behaves differently than it did.
- Treat a step that can do less than asked and still exit 0 as a fail-open default until proven
  otherwise.

## Related

- [[2026-09-05-constraint-a-concurrency-group-queues-one-run-and-evicts-the-rest]] — the other fail-open default in the same workflow
- [[2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed]] — prefer the degradation that tightens rather than loosens
