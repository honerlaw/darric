# Scratchpad: release-on-merge

## Balanced decisions 2026-09-05

- [reviewed — folded] scope check: one unit, one PR, unphased (Skeptic endorsed the scope call itself; folded its trigger-granularity point by widening `paths-ignore` to `**/*.md`/`LICENSE` — but rejected its suggested positive allow-list, which fails by silently shipping no release when a new source path appears; also folded the tag scheme, `permissions`, and the keep-forever retention call into the proposal as explicit decisions rather than implementation detail)
- [reviewed — folded] approach: B, native two-arch matrix + `gh release create` (Skeptic confirmed B over `tauri-action` and universal-binary, and verified bundle paths, per-arch DMG filenames, the `.minerva` exclusion mechanism and the tag-collision reasoning; folded seven corrections — `macos-13` is retired so the Intel leg is `macos-15-intel`, per-arch `upload-artifact` names to avoid a v4 409, an arch-keyed `rust-cache`, `--target "$GITHUB_SHA"` plus a checkout to close the tag race, idempotent create/`--clobber` for re-runs, `workflow_dispatch` as a recovery hatch, and `xattr -d com.apple.quarantine` as the real unsigned-app remedy)
- [decided] whole-proposal soundness: internally consistent, bounded to one workflow file plus one npm script and docs, no public interface or cross-cutting contract (solo gate)
