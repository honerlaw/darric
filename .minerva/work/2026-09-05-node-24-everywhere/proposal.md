# Proposal: node-24-everywhere

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)

## Goal

Node 24 is declared in exactly one place, and every consumer — CI, a local checkout, npm — reads
that place.

## Why

CI already runs Node 24, in four separate `setup-node` steps across two workflows. Nothing else in
the repo knows that:

| Where                                         | Says                                                   |
| --------------------------------------------- | ------------------------------------------------------ |
| `check.yml` × 3, `release.yml` × 1            | `node-version: 24`                                     |
| `README.md` prerequisites                     | `[Node.js](https://nodejs.org/) (v18+)`                |
| `.nvmrc` / `.node-version` / `.tool-versions` | absent — a version manager selects whatever it likes   |
| `package.json` `engines`                      | absent — npm never warns                               |
| `@types/node`                                 | `^25.8.0` — types for a major the project does not run |

The README is the proof that this drifts rather than a hypothetical: it still advertises a floor
six majors below what CI has actually required for some time, and nothing anywhere would have
caught that. A contributor following the README installs Node 18, `npm ci` succeeds without a
murmur, and the first signal is whatever breaks.

`@types/node` at `^25` is the same class of gap pointed the other way: TypeScript is told the Node
25 API surface exists, so `tsconfig.node.json` code can reference something Node 24 does not have
and typecheck clean.

## Approach

**Make `.nvmrc` the single source of truth and have every other consumer read it.**

1. **`.nvmrc`** containing `24`. This is what `nvm`, `fnm`, `volta` and `asdf` all read, so a local
   checkout selects the right major without anyone being told to.
2. **Both workflows** swap `node-version: 24` for `node-version-file: .nvmrc` in all four
   `setup-node` steps. `actions/setup-node@v4` supports this directly.
3. **`package.json`** gains `"engines": { "node": ">=24" }`, so npm warns a too-old runtime rather
   than failing obscurely later.
4. **`@types/node`** moves to `^24` to match the runtime.
5. **README** states Node 24 and points at `.nvmrc`.

The point of (2) is that it removes the failure this proposal exists to fix, rather than adding a
fifth copy of the number to keep in sync. After this, bumping Node is a one-line edit to `.nvmrc`
and CI follows automatically.

### Rejected

- **Hardcode `24` in all five places.** Simplest diff, and it recreates exactly the drift the README
  demonstrates — five copies, no mechanism keeping them equal.
- **`engine-strict=true` in `.npmrc`.** Turns the `engines` advisory into a hard `npm install`
  failure. That is a bigger blast radius than the seed asks for: it blocks a contributor on Node 22
  outright rather than warning them. `engines` + `.nvmrc` is the conventional stack; the hard gate
  can be added later if warnings prove insufficient.
- **Pin `engines` to `^24` / `24.x`.** Rejected because the risk being managed is "too old", not
  "too new" — someone on 25 is fine, someone on 18 is not.

### What changed in review

The README sentence introduced by step (5) claimed `nvm`, fnm, asdf and Volta all select the pinned
version from `.nvmrc`. Two of the four do not: asdf reads `.nvmrc` only with
`legacy_version_file = yes` in `~/.asdfrc`, which is off by default, and Volta ignores the file
entirely — it wants `volta pin node@24` in `package.json`. A contributor on Volta would have
followed that line, done nothing, and stayed on the wrong Node with no error — the same silent
class of failure this unit exists to close, reintroduced in its own documentation. The README now
says which manager does what.

## Success criteria

1. `.nvmrc` exists and contains `24`.
2. No `node-version:` literal remains in `.github/workflows/`; all four `setup-node` steps use
   `node-version-file: .nvmrc`.
3. `package.json` declares `engines.node` as `">=24"`.
4. `@types/node` is `^24` and the lockfile is updated to match.
5. `README.md` states Node 24 and references `.nvmrc`; no `v18+` remains.
6. `npm run check` passes — both typecheck configs included, since (4) changes the ambient types.

## Open Questions

None. `engine-strict` is deliberately deferred rather than left undecided; see Rejected.
