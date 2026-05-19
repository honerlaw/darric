# Agent Guidelines

## Linting Policy

**Linting must never be disabled, suppressed, or weakened — for any file, in any context.**

This applies to both TypeScript/ESLint and Rust/Clippy.

### Rules

- Do not add `// eslint-disable`, `// eslint-disable-next-line`, or `/* eslint-disable */` comments.
- Do not add `#[allow(...)]` attributes in Rust except inside `#[cfg(test)] mod tests` blocks, where `#[allow(clippy::unwrap_used)]` is the only permitted exception.
- Do not set any ESLint rule to `"off"`, `"warn"` (downgrading from `"error"`), or `0` in any config block.
- Do not add `@ts-ignore` or `@ts-expect-error` comments. Fix the underlying type error instead.
- Do not widen TypeScript compiler options to suppress errors (e.g., do not remove `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, or similar flags).
- Do not add `--no-verify` to git commands to skip hooks.

### Linting must be as strict as possible

- ESLint is configured with `strictTypeChecked` and `stylisticTypeChecked` from `typescript-eslint`. This is the ceiling — do not weaken it.
- Clippy is configured with `all = { level = "deny" }`, `pedantic`, `nursery`, and `cargo` lints. This is the ceiling — do not weaken it.
- If new lint rules become available that improve correctness or safety, add them.
- If a lint rule fires, fix the code — never suppress the rule.

### When a lint rule seems unreasonable

Fix the code to satisfy the rule. If the rule genuinely cannot be satisfied without making the code worse, raise it with the team rather than silently disabling it.

## minerva

This project uses [minerva](https://github.com/honerlaw/agent-marketplace/tree/main/plugins/minerva) for durable record discipline.

- `.minerva/decisions/` — authoritative architectural decisions. Read when starting work in this repo.
- `.minerva/work/` — historical proposals and replans. Grep when you need the reasoning behind a past feature.

Active work units live at `.minerva/work/NNN-<slug>/`. Invoke the `minerva:using-minerva` skill for the full methodology.
