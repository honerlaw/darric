# `npm run format` failed on any machine that had built the app

**Date**: 2026-09-05
**Type**: bug
**Summary**: `.prettierignore` did not exclude the git-ignored `src-tauri/gen/` Tauri schemas, so `npm run format` failed locally for anyone who had run a build — while CI stayed green because it formats before those files exist
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Tauri generates capability schemas into `src-tauri/gen/schemas/` during a build. They are
git-ignored by `src-tauri/.gitignore:7` (`/gen/schemas`), so they never appear in a diff.

## Finding

`.prettierignore` listed `dist/` and `src-tauri/target/` — the other two generated directories —
but not `src-tauri/gen/`. Prettier does not read `.gitignore`, so `prettier --check .` walked
the generated schemas and reported all four JSON files as unformatted:

```
[warn] src-tauri/gen/schemas/acl-manifests.json
[warn] src-tauri/gen/schemas/capabilities.json
[warn] src-tauri/gen/schemas/desktop-schema.json
[warn] src-tauri/gen/schemas/macOS-schema.json
```

The failure was invisible in CI and unavoidable locally. CI's TypeScript job runs `npm ci` then
`npm run format` and never builds the Rust side, so the directory does not exist when it looks.
Every developer who had ever run `npm run tauri:dev` or `tauri build` had it, so `npm run check`
failed for them and passed on `main`.

Fixed by adding `src-tauri/gen/` to `.prettierignore` alongside the two entries already there.

## Implications

A check that is green in CI and red on every developer machine is worse than one that is red in
both: the failure carries no signal, so the natural response is to distrust the check rather
than the configuration. When a lint or format tool has its own ignore file, generated output has
to be listed in _that_ file — being git-ignored is not enough.

## Related

- [[2026-09-05-reference-clippy-ceiling-configured-not-enforced]] — the mirror image: a rule CI does not enforce, rather than a check CI cannot see fail
