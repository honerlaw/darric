# The Clippy "ceiling" in `AGENTS.md` is configured but not enforced

**Date**: 2026-09-05
**Type**: reference
**Summary**: `AGENTS.md` calls `all=deny` + pedantic + nursery + cargo "the ceiling", but only `all` and `correctness` are deny — pedantic and nursery are warn, and no lint command passes `-D warnings`, so those violations stay green
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

`AGENTS.md` states:

> Clippy is configured with `all = { level = "deny" }`, `pedantic`, `nursery`, and `cargo`
> lints. This is the ceiling — do not weaken it.

alongside the hard rule that `#[allow(...)]` is forbidden outside `#[cfg(test)]` blocks.

## Finding

The configuration and the enforcement do not match.

`src-tauri/Cargo.toml` sets only two of the four groups to `deny`:

```toml
all = { level = "deny", priority = -1 }
correctness = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
```

`package.json`'s `lint:rust` runs `cargo clippy --manifest-path src-tauri/Cargo.toml
--all-targets` with no `-D warnings`, and `check.yml`'s Rust job sets no `RUSTFLAGS`. So a
pedantic, nursery or cargo violation prints a warning and the build stays green.

This is observable today: five pre-existing `#[allow]` sites — in `model.rs`, `audio/mod.rs`,
`audio/microphone.rs`, `transcription/mod.rs` and `transcription/speaker_tracker.rs` — violate
the `AGENTS.md` rule and have survived because nothing mechanically objects.

## Implications

- The no-`#[allow]` rule is upheld by whoever is reading the diff, not by CI. Treat it as a
  review obligation.
- Ironically, the `#[allow]` sites that do exist are all suppressing _pedantic_ lints, which are
  the ones that would not have failed the build anyway.
- Raising `pedantic`/`nursery` to `deny`, or adding `-D warnings`, would make the stated ceiling
  real — but would first require clearing the existing warnings.

## Related

- [[2026-09-05-reference-knowledge-wiki-is-ci-gated]] — the same repo's other gate, where the stated rule and the enforced rule do agree
- [[2026-09-05-bug-prettierignore-misses-generated-tauri-schemas]] — see also
- [[2026-09-06-reference-ci-clippy-runs-newer-than-a-local-toolchain]] — see also
