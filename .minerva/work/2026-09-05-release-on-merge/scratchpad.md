# Scratchpad: release-on-merge

## Balanced decisions 2026-09-05

- [reviewed — folded] scope check: one unit, one PR, unphased (Skeptic endorsed the scope call itself; folded its trigger-granularity point by widening `paths-ignore` to `**/*.md`/`LICENSE` — but rejected its suggested positive allow-list, which fails by silently shipping no release when a new source path appears; also folded the tag scheme, `permissions`, and the keep-forever retention call into the proposal as explicit decisions rather than implementation detail)
- [reviewed — folded] approach: B, native two-arch matrix + `gh release create` (Skeptic confirmed B over `tauri-action` and universal-binary, and verified bundle paths, per-arch DMG filenames, the `.minerva` exclusion mechanism and the tag-collision reasoning; folded seven corrections — `macos-13` is retired so the Intel leg is `macos-15-intel`, per-arch `upload-artifact` names to avoid a v4 409, an arch-keyed `rust-cache`, `--target "$GITHUB_SHA"` plus a checkout to close the tag race, idempotent create/`--clobber` for re-runs, `workflow_dispatch` as a recovery hatch, and `xattr -d com.apple.quarantine` as the real unsigned-app remedy)
- [decided] whole-proposal soundness: internally consistent, bounded to one workflow file plus one npm script and docs, no public interface or cross-cutting contract (solo gate)
- [reviewed — clean] completion verification: all 10 success criteria met (Verifier reproduced each independently and additionally ran the Rust half — clippy pedantic/nursery/cargo and rustfmt — which the checklist had conservatively deferred to CI; both clean, nothing folded)

## Review finding 2026-09-05

Eight findings from the code-review pass; six fixed, one dissolved with a fix, one ignored.

FIXED:
1. `download-artifact` only warns on a pattern that matches nothing, so re-running just the release job after one of the two artifacts had expired would have clobbered the release with a single architecture and gone green — the exact silent half-release `if-no-files-found: error` guards against on the upload side. Now asserts both DMGs are present before uploading.
2. `workflow_dispatch` had no ref restriction and the release job's guard was only "not a pull request", so dispatching from a feature branch would publish a real prerelease named `main-<sha>` from code that never merged. The guard now names `push` and `workflow_dispatch` on `refs/heads/main` explicitly.
3. A single ref-level concurrency group looked like it serialised releases, but GitHub keeps only one run pending per group — a third merge landing during the first one's build evicts the second's pending run, and that merge never gets a release. Real runs are now keyed per commit; PR dry runs stay keyed per PR ref so a new push still supersedes the old one.
4. `permissions: contents: write` sat at workflow level, so the build job held a repo-write token while running `npm ci` and cmake/whisper.cpp build scripts. Workflow default is now `contents: read`, with write granted only on the release job.
5. Because the concurrency key included `github.event_name`, a manual re-cut and the push run for the same commit landed in different groups and could race for the same tag. Subsumed by the per-commit key in (3).
6. `gh release view` matched the tag name only, so a release already standing under that tag for a different commit would have had another commit's binaries clobbered into it. The existing release's `targetCommitish` is now compared against `$GITHUB_SHA` and a mismatch fails loudly.

DISSOLVED: the README's "every merge" claim was stronger than the implementation guaranteed — true again once (3) gave every merge its own run.

IGNORED: the macOS setup block (cmake / rust-toolchain / rust-cache / setup-node / npm ci) is now duplicated across four job definitions and could be a composite action. No failure scenario, and the refactor reaches `check.yml`, which is outside this diff.

Verified by simulating the release script against a stubbed `gh` across all five states: zero DMGs, one DMG (expired artifact), two DMGs with no release, two DMGs re-running on its own release, and a tag owned by a different commit. Only the two-DMG cases proceed.
