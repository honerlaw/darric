# Proposal: release-on-merge

**Date**: 2026-09-05
**Status**: Shipped (2026-09-05)

## Goal

Every merge to `main` that can change the shipped binary cuts a GitHub prerelease carrying macOS
`.dmg` builds for both Apple Silicon and Intel. Knowledge-reconciliation merges, and merges that
touch only prose, cut nothing.

## Why

darric has no distribution path. `check.yml` verifies — typecheck, lint, format, tests, clippy —
and stops there. Nothing anywhere runs `tauri build`: there is no release workflow, no tag trigger,
no artifact upload, and `package.json` has `tauri:dev` but no `tauri:build`. `tauri.conf.json` is
already fully configured to bundle (`bundle.active: true`, `targets: "all"`, macOS entitlements and
`Info.plist`), so the capability is one manual `npm run tauri build` away — and that manual step is
the whole problem. There is no way to hand anyone a build of `main`.

### Why reconcile merges are already excluded, and why that is not enough

[[2026-09-05-reference-github-token-actions-trigger-no-workflows]] records that
`knowledge-reconcile.yml` opens _and merges_ its own PR with `GITHUB_TOKEN`, and GitHub creates no
workflow run for anything `GITHUB_TOKEN` does. A reconcile merge therefore fires **no `push` event
on `main` at all** — a release workflow triggered on `push` never sees it, with no filter written.

That mechanism is load-bearing but invisible, and the same entry names swapping `GITHUB_TOKEN` for
a PAT as "a real option, not a fix that has been applied". A release cut by a token change nobody
connected to releases is a bad surprise, so the exclusion is also stated explicitly in the trigger
rather than left resting on a side effect.

## Approach

**A new `.github/workflows/release.yml` that builds natively on one runner per architecture and
attaches both DMGs to a single prerelease per merge.**

### Trigger

```yaml
on:
  push:
    branches: [main]
    paths-ignore: [".minerva/**", "**/*.md", "LICENSE"]
  pull_request:
    paths: [".github/workflows/release.yml"]
  workflow_dispatch:
```

A **deny-list, not an allow-list.** An allow-list of shippable directories (`src/**`,
`src-tauri/**`, …) fails by silently shipping no release when a new source path appears; a deny-list
fails by cutting one redundant release. For a release pipeline the first failure is invisible and
the second is merely wasteful, and no code file can ever match the deny patterns. This is
[[2026-09-05-pattern-an-automated-gate-must-be-scoped-to-what-its-pipeline-changed]] applied to a
trigger: prefer the degradation that cannot silently drop work.

The `pull_request` leg exists because a `push`-only workflow **cannot be validated before it
merges**. Scoped to this one file, it means any PR editing the release workflow builds both
architectures for real, while the release step is suppressed — so this PR exercises its own build
path. `workflow_dispatch` is the manual re-cut hatch, matching `knowledge-reconcile.yml`.

### Build matrix — native per architecture, never cross-compiled

`whisper-rs` is built with the `metal` feature and compiles whisper.cpp's C/C++ through cmake.
Cross-compiling that from arm64 to x86_64 is the single largest build risk available here, so each
architecture gets its own native runner:

| Runner           | Arch      | DMG                        |
| ---------------- | --------- | -------------------------- |
| `macos-15`       | `aarch64` | `Darric_0.1.0_aarch64.dmg` |
| `macos-15-intel` | `x64`     | `Darric_0.1.0_x64.dmg`     |

`macos-13` — the label a reader reaches for first — **was retired on 2025-12-04** and would fail
immediately. `macos-15-intel` is the current x86_64 image and is supported through Fall 2027; it is
the last Intel line, so the Intel leg has a known expiry. Both legs pin `macos-15` rather than
following `macos-latest` (as `check.yml` does) because a release build should be reproducible and
matched across architectures, not floating.

Steps mirror `check.yml`'s Rust job — `cmake --version || brew install cmake`,
`dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache` (`workspaces: src-tauri`), node 24, `npm ci`
— then `npm run tauri:build`, a new script beside the existing `tauri:dev`.

Two matrix-specific hazards `check.yml`'s single-job usage has never exercised:

- **`Swatinem/rust-cache` keys on `runner.os`, which is `macOS` for both legs.** Without an arch in
  the key the two caches collide, which is a cross-architecture build failure arriving by a
  different door than the one the native-runner choice exists to close.
- **`actions/upload-artifact@v4` rejects a duplicate artifact name with a 409.** Each leg uploads
  under `dmg-<arch>`; the release job downloads with `merge-multiple: true`.

The upload sets `if-no-files-found: error`. A build that produces no DMG must fail the job — the
alternative is a green run that cuts an empty release.

### Release identity

One immutable prerelease per qualifying merge, tagged `main-<short-sha>`.

`version` is `0.1.0` in `tauri.conf.json`, `Cargo.toml` and `package.json` and has never been
bumped, so any version-derived tag collides on the second merge. A rolling `latest-main` release
overwritten in place was the other candidate; per-commit tags were chosen because they keep history
and match "cut a release" literally. Releases are kept, not pruned.

The release job takes `--target "$GITHUB_SHA"`. Without it `gh release create` tags whatever is at
the tip of `main` when the command runs, and a macOS build takes long enough that another merge can
land first — silently pointing `main-<sha>` at a different commit than the one in the DMG. It also
checks out the repo, so `gh` has repo context.

Creation is idempotent, because re-running a half-failed release job is the normal recovery path:

```sh
gh release view "$TAG" >/dev/null 2>&1 || gh release create "$TAG" --target "$GITHUB_SHA" --prerelease ...
gh release upload "$TAG" dmg/*.dmg --clobber
```

### Signing

None. No Apple Developer certificates exist in the repo or in CI, so the DMGs are unsigned. On
Apple Silicon a downloaded unsigned app carries the quarantine attribute and macOS reports
**"Darric is damaged and can't be opened"** — not the milder "unidentified developer" prompt, and a
user who sees it concludes the download is corrupt. The README therefore documents the actual
remedy, `xattr -d com.apple.quarantine`, rather than "right-click → Open", which does not clear it.

### Rejected alternatives

- **`tauri-apps/tauri-action`'s three-job recipe** (create-release → matrix build with `releaseId` →
  publish). Canonical and handles bundle discovery, but adds a third-party action to a CI that uses
  only `actions/*`, `dtolnay/rust-toolchain` and `Swatinem/rust-cache`, and hides the asset naming
  and release identity this proposal needs to control.
- **A single universal binary** (`--target universal-apple-darwin` on one runner). One download for
  users, but it cross-compiles the x86_64 half of whisper.cpp/Metal on an arm64 host — the exact
  risk the native matrix exists to avoid, for a build that has never been proven in this repo.

### What changed in review

Six defects in the workflow were found by the code-review pass and fixed before shipping. Five of
them shared a shape worth naming: **a control that looks like it guarantees something, and does
not.**

1. **The concurrency group did not serialise — it evicted.** `cancel-in-progress: false` promises
   only that a _running_ release is not cancelled; a group holds one _pending_ run, so a third
   merge landing during the first one's build cancelled the second's run outright and that commit
   never got a release. Real runs are now keyed per commit, PR dry runs per PR ref. The "every
   merge" claim in this proposal and in the README was false until this was fixed, and is true
   now. Promoted as
   [[2026-09-05-constraint-a-concurrency-group-queues-one-run-and-evicts-the-rest]].
2. **`if-no-files-found: error` did not carry across the artifact handoff.** `download-artifact`
   has no equivalent input and only _warns_ when its pattern matches nothing, so re-running just
   the release job — the documented recovery path — after one artifact had expired would have
   published a one-architecture release and gone green. The release step now asserts both DMGs are
   present. Promoted as
   [[2026-09-05-reference-actions-download-artifact-warns-where-upload-errors]].
3. **`workflow_dispatch` was not restricted to `main`.** The release job's guard was only "not a
   pull request", so dispatching from a feature branch would have published a real prerelease
   named `main-<sha>` from code that never merged. The guard now names the two legal cases.
4. **`gh release view` matched the tag name only.** A release already standing under that tag for
   a different commit would have had another commit's binaries clobbered into it; the existing
   release's `targetCommitish` is now compared against `$GITHUB_SHA`.
5. **`permissions: contents: write` sat at workflow level**, handing a repo-write token to the
   build job while it runs `npm ci` and cmake/whisper.cpp build scripts. The workflow default is
   now `contents: read`, with write granted only on the release job.
6. Including `github.event_name` in the concurrency key put a manual re-cut and the push run for
   the same commit in _different_ groups, where they could race for the same tag — subsumed by the
   per-commit key in (1).

The release script was verified against a stubbed `gh` across all five reachable states: zero
DMGs, one DMG, two DMGs with no existing release, two DMGs re-running on its own release, and a
tag owned by a different commit. Only the two-DMG cases proceed.

One review finding was not acted on: the macOS setup block (cmake / rust-toolchain / rust-cache /
setup-node / npm ci) now appears in four job definitions across two workflows and could be a
composite action. It has no failure scenario and the refactor reaches `check.yml`, outside this
diff.

## Success criteria

1. `.github/workflows/release.yml` triggers on `push` to `main` with
   `paths-ignore: [".minerva/**", "**/*.md", "LICENSE"]`, on `pull_request` limited to that file,
   and on `workflow_dispatch`.
2. The build matrix is `macos-15`/`aarch64` and `macos-15-intel`/`x64`, with no `--target` flag and
   no cross-compilation anywhere.
3. `Swatinem/rust-cache` is keyed per architecture, and each leg uploads under a distinct artifact
   name.
4. A leg producing no DMG fails (`if-no-files-found: error`).
5. The release job attaches **both** DMGs to one prerelease tagged `main-<short-sha>`, pinned with
   `--target "$GITHUB_SHA"`.
6. Re-running the release job on an existing tag succeeds rather than erroring.
7. The release job is suppressed on `pull_request` runs, so this PR builds both architectures
   without cutting a release.
8. `package.json` has `"tauri:build": "tauri build"`, and CI invokes it rather than a raw `tauri`
   call.
9. `README.md` documents both downloads and the `xattr -d com.apple.quarantine` step.
10. `npm run check` passes — the new YAML is prettier-clean.

## Open Questions

- **"Various builds" is read as both architectures**, not as multiple bundle formats. Only `.dmg`
  ships; `.app.tar.gz` would double the assets for no distinct use. Reversible either way.
- The Intel leg's runner label is supported through Fall 2027 and is the last x86_64 image. Nothing
  to do now; the Intel row simply comes out of the matrix when it lapses. Recorded as
  [[2026-09-05-reference-macos-13-is-retired-and-macos-15-intel-is-the-last-x86-64-image]].
- Signing is deferred, not dismissed — it is gated on a paid Apple Developer account rather than on
  the workflow. Filed as #37.
