# `.minerva/knowledge/` coherence is enforced by CI

**Date**: 2026-09-05
**Type**: reference
**Summary**: `check.yml`'s `Knowledge Wiki` job runs `knowledge_lint.py` on every pull request and every push to main — index drift and broken wikilinks are errors, uncatalogued entries and missing reciprocals are warnings
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

`.github/workflows/check.yml` gained a sixth job, `Knowledge Wiki`, in commit `921fc91`. It
checks out the minerva tooling from `honerlaw/agent-marketplace` and runs:

```
python .minerva-tools/plugins/minerva/scripts/knowledge_lint.py .minerva/knowledge
```

The job has no path filter and no conditional, so it runs on every pull request and every push
to `main`, whether or not the change touches `.minerva/`.

## Finding

What fails the build and what does not is deliberate, and worth knowing before promoting:

- **Errors** — duplicate entry ids, index drift (the catalog claiming entries that do not
  exist), and a broken `[[…]]` wikilink inside a `## Related` block.
- **Warnings** — an entry with no catalog line, and a forward link whose reciprocal back-link is
  missing.

That split is what makes `minerva:promote` safe to run add-only on a work-unit branch. Promote
writes entry files and never touches `index.md` or `overview.md`; the resulting uncatalogued
entry and unreciprocated link are exactly the two warning conditions, and `minerva:cleanup`
repairs them on the default branch afterwards. A promote that "helpfully" edited the index would
be fighting the reconciliation pass, not helping it.

The practical rule: a new entry may link only to entries that already exist. Everything else
reconciles later.

## Implications

- Run `knowledge_lint.py` locally before pushing a branch that adds entries; a broken wikilink
  is an error and will fail the PR.
- Do not stage `index.md` or `overview.md` from a work-unit branch.

## Related

- [[2026-09-05-reference-knowledge-corpus-not-ci-gated]] — the state this replaced, when nothing ran the linter
