# A concurrency group holds one pending run, so serialising by ref silently drops the middle of a burst

**Date**: 2026-09-05
**Type**: constraint
**Summary**: `concurrency` with `cancel-in-progress: false` does not queue runs — it keeps exactly one pending run per group and evicts the previous pending one, so a third push landing during the first one's build cancels the second's run outright and that commit is never built
**Context**: .minerva/work/2026-09-05-release-on-merge

## Context

`release.yml` cuts a prerelease per merge to `main`. The first version serialised on the ref:

```yaml
concurrency:
  group: release-${{ github.event_name }}-${{ github.ref }}
  cancel-in-progress: false
```

The intent, stated in a comment, was "never cancel a release that is already building" — and
`cancel-in-progress: false` does deliver exactly that.

## Finding

It delivers only that. The flag governs the **running** member of the group; it says nothing
about the pending one, and a group holds **at most one** pending run. A third run arriving while
one is in progress does not join a queue behind the second — it _replaces_ it, and the replaced
run is cancelled before it ever starts.

For a release pipeline keyed on `github.ref`, every push to `main` shares one group, so:

| Event         | Group state                                         |
| ------------- | --------------------------------------------------- |
| merge A lands | A running                                           |
| merge B lands | A running, B pending                                |
| merge C lands | A running, C pending — **B cancelled, never built** |

Nothing fails. B's code is not lost — C builds tip-of-main, which contains it — but no release is
ever cut for B, and the workflow's own header claimed one is cut for every merge. A macOS build
here takes long enough (two native runners compiling whisper.cpp through cmake) that a burst of
three merges is an ordinary Tuesday, not a contrived race.

The fix is to key the group by **what must not be duplicated**, which for a per-commit release is
the commit:

```yaml
group: release-${{ github.event_name == 'pull_request' && format('pr-{0}', github.ref) || format('sha-{0}', github.sha) }}
```

Per-commit groups are unique per run, so nothing is ever evicted. Serialisation is not lost where
it was actually needed: a manual `workflow_dispatch` re-cut of a commit lands in the same group as
that commit's push run and queues behind it instead of racing it for the same tag — which the
earlier key, by including `github.event_name`, had put in a _different_ group.

## Implications

- `cancel-in-progress: false` is a promise about the running job only. If you need every trigger
  to produce a run, the group key must be unique per trigger — usually `github.sha`, not
  `github.ref`.
- Ask of any `concurrency` block: _when two are already waiting, which one is thrown away?_ A ref
  key answers "the older one", silently.
- A group key including `github.event_name` splits triggers that contend for the same resource
  apart. That is right for PR dry runs (a different resource) and wrong for a manual re-run of the
  same commit (the same tag).

## Related

- [[2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed]] — the same shape one level up: a pipeline control scoped to the wrong thing fails quietly rather than loudly
- [[2026-09-05-reference-actions-download-artifact-warns-where-upload-errors]] — the other fail-open default found in the same workflow
