# `.nvmrc` is the only place the Node version is written; workflows read it

**Date**: 2026-09-05
**Type**: decision
**Summary**: both workflows use `node-version-file: .nvmrc` rather than a `node-version:` literal, so bumping Node is a one-line edit — do not "simplify" a workflow back to a hardcoded version
**Context**: .minerva/work/2026-09-05-node-24-everywhere

## Context

Before this, `node-version: 24` appeared in four `setup-node` steps across `check.yml` and
`release.yml`, and nothing else in the repo recorded a Node version at all — no `.nvmrc`, no
`engines`, nothing a version manager reads.

The README, meanwhile, still listed `Node.js (v18+)` as the prerequisite. Six majors below what CI
had actually required for some time, with nothing anywhere that could have noticed.

## Decision

`.nvmrc` holds the version. Everything else defers to it:

- both workflows use `node-version-file: .nvmrc` in all four `setup-node` steps;
- `package.json` declares `engines.node` as `">=24"` so npm warns on a too-old runtime;
- the README names 24 and points at `.nvmrc`;
- `@types/node` tracks the runtime major (`^24`), so TypeScript is not told that a newer Node's
  API surface exists.

**Do not replace `node-version-file` with a `node-version:` literal.** The indirection is the whole
point: the README drift above is what four independent copies of a version number produce, and a
fifth copy is what a "simpler" workflow edit would add. Bumping Node should be one line.

`engines` is deliberately `>=24` rather than `^24`: the risk being managed is a runtime that is too
old, not one that is too new. There is deliberately no `engine-strict=true` — that converts the
advisory into a hard `npm install` failure, which is a bigger gate than the problem warrants.

## Implications

- Bump Node by editing `.nvmrc`. CI follows on the next run; no workflow edit is needed or wanted.
- `engines` and `.nvmrc` can legitimately disagree upward — a contributor on Node 26 satisfies
  `engines` while not running what CI runs. That is disclosed, not accidental.
- **`.nvmrc` is not universally honoured by version managers**, so the README says which do what
  rather than implying they are equivalent: `nvm use` and fnm read it, asdf needs
  `legacy_version_file = yes` in `~/.asdfrc`, and Volta ignores it entirely (it wants
  `volta pin node@24`). A contributor on Volta who is told "your version manager picks it up" gets
  no error and the wrong Node.

## Related

- [[2026-09-05-reference-macos-13-is-retired-and-macos-15-intel-is-the-last-x86-64-image]] — the other toolchain version pinned in these workflows, and the one that expires on a schedule
