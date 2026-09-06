# Proposal: whisper-model-github-release

**Date**: 2026-09-06
**Status**: Draft

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

1. **One-time release, outside the tree.** Create a release on the fixed tag `models`:
   `gh release create models --target main --latest=false --title "Model assets" --notes …`,
   then `gh release upload models ggml-large-v3-turbo.bin`. The tag is a stable, unversioned
   home — assets are named by model file and the app pins exact bytes by checksum, so the tag
   never needs a version suffix. `--latest=false` keeps the DMG prereleases at the top of the
   Releases page. Re-download the uploaded asset and hash it to prove the upload is byte-exact.
2. **`src-tauri/src/model.rs`.** `MODEL_URL` becomes
   `https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin`. Add
   `const MODEL_SHA256`. Feed every chunk to a `sha2::Sha256` inside the existing stream loop;
   after the loop — and **before** `tokio::fs::rename` — compare the lower-hex digest with the
   constant and return an `AppError::Audio("model download failed checksum …")` on mismatch, so
   the existing failure path in `ensure_model` removes the `.tmp` and emits
   `model_download_error`; on a match, log one line naming the verified digest. Also treat a
   stream that ends with fewer bytes than `Content-Length` as a failure, guarded on `total > 0`
   so a server that sends no `Content-Length` is not misreported (the checksum catches
   truncation too, but the message should say "truncated" rather than "checksum"). Add `sha2 = "0.10"` to
   `Cargo.toml` — already in `Cargo.lock` transitively, so no new compiled code.
3. **Testable seam, minimal.** Extract the digest comparison into a pure
   `fn verify_digest(actual_hex: &str) -> Result<()>` with an inline `#[cfg(test)] mod tests`
   covering match and mismatch. Keep the diff small: a sibling unit
   (`transcript-accuracy`, live session `darric-3f`) is generalising the downloader in this
   file at the same time, and both branches will merge into it. Sequencing: whichever branch
   lands first wins; the other rebases. If the sibling's `ensure_file(url, filename)` lands
   first, this unit's checksum threads through that function rather than `download`.
4. **README.** Say where the model is downloaded from and why (huggingface.co is blocked on
   some networks), that the download is checksum-verified, and how to place the file by hand if
   neither host is reachable.

### Candidate approaches considered

- **A — GitHub Release asset + pinned checksum (chosen).** Reachable wherever the app is
  installable; no new interface; one-time upload; the checksum makes the served bytes
  self-validating.
- **B — Configurable URL (env var or setting) defaulting to A.** Adds a public interface and a
  settings surface for a third host nobody has today, in a file a sibling unit is refactoring.
  Out of scope for this ask; it can be its own unit.
- **C — Ordered fallback: GitHub first, Hugging Face second.** Rejected: every machine that can
  install the app can reach GitHub release assets, and a fallback that silently switches hosts
  hides a network problem worth surfacing.
- **D — Bundle the model in the DMG.** A 1.6 GB installer on every merge to main. Rejected.

## Success criteria

- Ordering: the `models` release asset is live and verified (the next two criteria) **before**
  the code change is pushed for review. Once merged, every install downloads from the new URL,
  so a PR whose asset is not yet live would break first launch for everyone.
- `MODEL_URL` in `src-tauri/src/model.rs` is
  `https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin`, and
  `curl -sIL` of that URL ends in a 200 with `content-length: 1624555275`.
- The `models` release exists and is not marked latest (`gh release view models --json isLatest`
  reports `false`), so the DMG prereleases keep the Releases page's Latest badge.
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
