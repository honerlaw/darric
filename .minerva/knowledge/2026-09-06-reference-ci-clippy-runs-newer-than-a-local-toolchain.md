# CI's `stable` clippy is newer than a local toolchain and enforces lints the local run never saw

**Date**: 2026-09-06
**Type**: reference
**Summary**: `dtolnay/rust-toolchain@stable` resolved clippy 1.98 while the machine ran 1.93; `chunks_exact_to_as_chunks` under `-D clippy::all` failed CI after a clean local `npm run check`
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

`npm run check` ran clean locally (clippy 1.93.0), the PR was opened, and the `Rust` job failed
in under two minutes on a lint the local toolchain did not know: `chunks_exact_to_as_chunks`,
introduced in clippy 1.98 and denied through the crate's `all = "deny"` group.

## Finding

The workflow pins nothing: `dtolnay/rust-toolchain@stable` installs whatever stable is current
on the runner. A local toolchain lags by however long since the last `rustup update`. Any lint
added to `clippy::all` in between is a CI failure the local run cannot predict, and under this
repo's policy the only fix is the code change the lint asks for (`as_chunks::<4>()` here).

## Implications

- A green local `npm run check` is necessary, not sufficient; expect an occasional lint-only
  CI failure after a quiet period and fix the code, never the config.
- `rustup update stable` before a long unit closes most of the gap.
- Pinning a toolchain via `rust-toolchain.toml` would make local and CI agree; the repo has
  deliberately not done so, so the ceiling keeps rising on its own.

## Related

- [[2026-09-05-reference-clippy-ceiling-configured-not-enforced]] — the other half of how the lint ceiling behaves
