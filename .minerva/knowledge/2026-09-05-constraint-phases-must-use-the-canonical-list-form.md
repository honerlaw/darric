# A proposal's `## Phases` must be a top-level numbered list, or phasing silently disengages

**Date**: 2026-09-05
**Type**: constraint
**Summary**: `read_phases` stops at the next `#` line, so writing phases as `### 1. Name` subsections parses as zero phases — the unit ships as unphased and its later phases are stranded with no error
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

`work_status.read_phases` is the single reader for a unit's declared phases. Every consumer —
`minerva:ship`'s phase resolution, `minerva:cleanup`'s deferred teardown, and all four
orchestrators' cleanup loop — asks it, and it recognises exactly one shape:

```
_PHASES_HEADING_RE = re.compile(r"^##\s+Phases\s*$", re.IGNORECASE)
_PHASE_ITEM_RE     = re.compile(r"^(\d+)\.\s+(.*\S)\s*$")
```

It scans forward from the `## Phases` heading and **breaks at the next line starting with `#`**.

## Finding

Writing the phases as `### N. Name` subsections — the natural choice when each phase needs a
paragraph of detail — puts a `#` line immediately after the heading. The scan breaks before
reading anything and returns an empty list, which is the exact representation of "this unit is
unphased".

Nothing errors. `phase_progress` returns `{'phased': False, 'next_branch': None}`, ship omits the
`Phase:` line from its report, and `minerva:cleanup` — which defers worktree teardown only while
a declared phase is unmerged — sees no declared phases and tears the worktree down after the
first PR merges. The remaining phases have no branch, no worktree and no mention in any report.

This was caught here only because ship's phase-resolution step was run and printed
`phases declared: 0` for a proposal that visibly had three.

## Implications

- Put the canonical list directly under `## Phases`, before any subsection:
  `1. **Name** — description.` Continuation lines may wrap and are appended to the title;
  indented lines cannot be mistaken for new phases.
- Long per-phase detail goes in a _separate_ section after it (`## Phase detail`), which is free
  to use `###` subsections.
- After writing a `## Phases` section, verify it parses rather than trusting how it reads:
  `read_phases(proposal_text)` should return one entry per phase.
- The failure is silent and its damage is deferred, which is the combination that makes it worth
  a check rather than care.

## Related

- [[2026-09-05-decision-strip-darric-to-a-recorder]] — the phased unit whose declaration hit this
