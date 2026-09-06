# Proposal: whisper-model-github-release

**Date**: 2026-09-06
**Status**: Shipped (2026-09-06)

## Goal

Make the whisper model downloadable on machines that block huggingface.co: host
`ggml-large-v3-turbo.bin` as an asset on a GitHub Release in this repository, point the app's
download at it, and verify the downloaded bytes against a compiled-in SHA-256 before the file
is renamed into place.

## Why

`model::ensure_model` downloads from a hard-coded huggingface.co URL. The user's work network
blocks that host, so the app cannot obtain its model there at all; the only workaround today is
copying the 1.6 GB file by hand into `~/Library/Application Support/darric/`. That same network
already serves the app itself from this repository's GitHub Releases (the DMGs at
`github.com/honerlaw/darric/releases/download/main-<sha>/…`), so a release asset in the same
place is reachable wherever the app is installable. GitHub allows release assets up to 2 GiB;
the model is 1,624,555,275 bytes. The `ggerganov/whisper.cpp` model card on Hugging Face declares
`license: mit` for these ggml conversions, so mirroring is permitted.

Verifying the download is cheap to add at the same time. The bytes are now served from a host we
control under a name we pin, so there is an authoritative checksum: Hugging Face publishes
`1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` as the LFS object hash
(`x-linked-etag`), and the locally cached copy hashes to the same value. Today a stream that
ends early without a transport error is renamed into place and then trusted forever
([[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]]); a checksum on the download
closes that. It does not validate an already-cached file — that is issue #16, which is adjacent
and stays open.

## Approach

What shipped. See [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]]
for the decision record.

1. **One-time release, outside the tree.** `gh release create models --target main
--latest=false --title "Model assets"`, then `gh release upload models
ggml-large-v3-turbo.bin` (77 s). `--latest=false` did not keep the release off the Latest
   badge — every other release here is a prerelease, so the only full release is latest by
   definition ([[2026-09-06-constraint-make-latest-false-cannot-hide-the-only-full-release]]) —
   and the release was edited to a prerelease, which restored the no-Latest state. The tag is a
   stable, unversioned home: assets are named by model file and the app pins exact bytes by
   checksum.
2. **`src-tauri/src/model.rs`.** `MODEL_URL` is
   `https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin`
   (redirects to `release-assets.githubusercontent.com`); `MODEL_SHA256` pins the bytes. Every
   chunk feeds a `sha2::Sha256` inside the existing stream loop. After the loop and before the
   rename, `check_length(total, downloaded)` rejects a stream that ended short of or past
   `Content-Length` (skipped when the server sent none) and `verify_digest(MODEL_SHA256, …)`
   rejects any other digest; both return an `AppError::Audio`, so `ensure_model`'s existing
   failure path removes the `.tmp` and emits `model_download_error`. A match logs one line
   naming the digest. `sha2 = "0.10"` was added to `Cargo.toml`; it was already in the lock.
3. **Testable seams.** Both checks are pure functions taking their expectations as parameters,
   with an inline `#[cfg(test)] mod tests` (eight tests: match, mismatch, case sensitivity, the
   pinned constant's shape, and the four length cases). The diff was kept small because a
   sibling unit (`transcript-accuracy`, session `darric-3f`) is generalising the downloader
   into `ensure_file(url, filename)` in the same file; it had not landed when this shipped,
   and the parameterised helpers are meant to be reused by it per file.
4. **README.** Documents the source and why, the checksum, that a bad download is refused
   rather than cached (with no automatic retry), and the manual-copy fallback.

### Candidate approaches considered

- **A — GitHub Release asset + pinned checksum (chosen).**
- **B — Configurable URL (env var or setting) defaulting to A.** Rejected: a public interface
  and settings surface for a third host nobody has today, in a file a sibling unit is
  refactoring.
- **C — Ordered fallback: GitHub first, Hugging Face second.** Rejected: every machine that can
  install the app can reach GitHub release assets, and a silent host switch hides a network
  problem worth surfacing.
- **D — Bundle the model in the DMG.** A 1.6 GB installer on every merge to main. Rejected.

## Success criteria

- Ordering: the `models` release asset is live and verified (the next two criteria) **before**
  the code change is pushed for review. Once merged, every install downloads from the new URL,
  so a PR whose asset is not yet live would break first launch for everyone. (Met: asset live
  15:29Z, verified end to end 15:33Z, PR opened afterwards.)
- `MODEL_URL` in `src-tauri/src/model.rs` is
  `https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin`, and
  `curl -sIL` of that URL ends in a 200 with `content-length: 1624555275`.
- The `models` release exists and is a prerelease (`gh release view models --json isPrerelease`
  reports `true`), and `gh api repos/honerlaw/darric/releases/latest` still returns 404 as it
  did before, so nothing presents the model release as the app's latest release.
- `MODEL_SHA256` is `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`, which
  equals both Hugging Face's `x-linked-etag` for the file and `shasum -a 256` of the uploaded
  asset downloaded back from GitHub.
- A download whose digest does not match is not renamed into place: `verify_digest` returns an
  error, and the existing `ensure_model` failure path removes the `.tmp` and emits
  `model_download_error`. Covered by inline unit tests on `verify_digest` (match, mismatch).
- End-to-end on this machine: with the cached model moved aside, the app downloads the model
  from the GitHub URL, logs a successful checksum, renames it into place, and the resulting file
  hashes to `MODEL_SHA256`.
- README documents the source, the checksum verification, and the manual-copy fallback.
- `npm run check` and `cargo test` pass in the worktree; no lint suppression added (`CLAUDE.md`).

## Open Questions

- Whether the work network allows `release-assets.githubusercontent.com`, the host GitHub
  redirects release-asset downloads to (verified from a live `main-*` DMG download's `Location`
  header), can only be confirmed from that machine. Assumed yes because the DMGs download
  through that same host.
- GitHub caps a release asset at 2 GiB; this model leaves ~0.5 GB of headroom. A larger model
  would need a different host or a split asset — out of scope, noted for the future.
