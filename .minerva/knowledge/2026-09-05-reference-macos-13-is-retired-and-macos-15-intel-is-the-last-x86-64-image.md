# `macos-13` is retired; `macos-15-intel` is the last x86_64 GitHub-hosted image

**Date**: 2026-09-05
**Type**: reference
**Summary**: `macos-13` — the label anyone reaches for first to build Intel macOS binaries — was retired 2025-12-04; `macos-15-intel` is the current x86_64 runner and is supported only through Fall 2027, after which no hosted Intel macOS runner exists
**Context**: .minerva/work/2026-09-05-release-on-merge

## Context

darric ships a `.dmg` per architecture. `whisper-rs` is built with the `metal` feature and
compiles whisper.cpp through cmake, so cross-compiling arm64 → x86_64 is the largest build risk
available; a native runner per architecture removes it. That makes the runner labels load-bearing.

## Finding

`macos-13` was the standard answer for "the Intel macOS runner" for years, and it is what a
reader — or a model — reaches for by default. GitHub began deprecating it 2025-10-01 and retired
it fully on **2025-12-04**. It is not a slow or discouraged label today; it does not resolve, and
a workflow naming it fails on its first run.

The current x86_64 image is **`macos-15-intel`** (a `macos-26-intel` also exists). darric's
release matrix pairs it with `macos-15` so both legs build on the same macOS version:

```yaml
- runner: macos-15
  arch: aarch64
- runner: macos-15-intel
  arch: x64
```

Both are pinned rather than following `macos-latest` as `check.yml` does — a release build should
be reproducible and matched across architectures, where CI can float.

**`macos-15-intel` is supported through Fall 2027, and is the last Intel line.** Apple Silicon is
the only architecture with a future on hosted runners.

## Implications

- Before writing any macOS runner label into a workflow, check it is still live. This class of
  label is retired on a published schedule, and the failure is immediate and total rather than
  degraded.
- The Intel leg of `release.yml` has a known expiry. When `macos-15-intel` lapses, the row comes
  out of the matrix and darric ships Apple Silicon only — or an Intel build starts requiring
  cross-compilation, which is what the native matrix exists to avoid.
- `macos-latest` is not a fix for this: it tracks arm64 and cannot produce an Intel binary.

## Related

- [[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]] — the Metal dependency that makes the native-runner-per-arch choice matter
