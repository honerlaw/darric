# `.minerva/knowledge/` coherence is not enforced by CI

**Date**: 2026-09-05
**Type**: reference
**Summary**: `check.yml` runs TypeScript/Rust builds and tests only — nothing runs `knowledge_lint.py`, so index drift and broken `[[…]]` wikilinks reach `main` green
**Context**: .minerva/work/2026-09-05-knowledge-wiki-migration

## Context

`.github/workflows/check.yml` is darric's only workflow. It triggers on `push` to `main` and on
every `pull_request`, and runs five jobs: TypeScript, Frontend Build, Rust, Frontend Tests, Rust
Tests. All five are source-code jobs with no path filter.

`.minerva/knowledge/` acquired a real corpus for the first time in the migration that produced
this entry. Before that there was nothing for a gate to check.

## Finding

No CI job runs `knowledge_lint.py`, so the wiki's mechanical coherence is unenforced on the
default branch. The detector reports index drift (catalog ↔ file bijection, Type-section
grouping), broken `[[YYYY-MM-DD-type-slug]]` links, and missing reciprocals as **errors**, but
only when a human invokes `minerva:lint` — nothing runs it on a PR.

A hand-edit or a merge that drops a catalog line, or renames an entry file without retargeting
the wikilinks pointing at it, therefore lands on `main` with CI green. The rot is silent: the
index still _looks_ like a catalog, and an agent following it reaches a stem that no longer
resolves.

## Implications

- Treat a green PR as saying nothing about the corpus. Run `minerva:lint` by hand after any
  change under `.minerva/knowledge/`.
- `minerva:cleanup` reconciles the index on the default branch after a merge, which repairs the
  common add-only case — but it runs only when invoked, and it is not a gate.
- Closing this would mean a sixth job in `check.yml` running the detector. That is hardening,
  not a defect: nothing is currently broken, and the failure needs a future bad edit to trigger.

## Related

- [[2026-09-05-reference-claude-md-symlinks-to-agents-md]] — the other unguarded fact about this repo's agent-records layer
