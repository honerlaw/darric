# The whisper model is served from this repo's `models` GitHub Release, pinned by SHA-256

**Date**: 2026-09-06
**Type**: decision
**Summary**: `MODEL_URL` points at the `models` release asset, not huggingface.co, and the streamed download must hash to `MODEL_SHA256` before rename
**Context**: .minerva/work/2026-09-06-whisper-model-github-release (see git history if the worktree has been cleaned up)

## Context

`model::ensure_model` fetched `ggml-large-v3-turbo.bin` from a hard-coded huggingface.co URL.
The user's work network blocks that host outright, so the app could not obtain its model there
at all, while the same network already serves the app's DMGs from this repository's GitHub
Releases. The Hugging Face model card for `ggerganov/whisper.cpp` declares `license: mit`, so
mirroring the file is permitted, and at 1,624,555,275 bytes it fits under GitHub's 2 GiB
per-asset cap with about 0.5 GB to spare.

## Finding

The model is hosted as an asset on a fixed, unversioned prerelease tagged `models` in
`honerlaw/darric`, uploaded once by hand with `gh release upload` (77 s for 1.55 GB). The tag
never needs a version suffix: assets are named by model file, and the app pins the exact bytes
by checksum. `MODEL_URL` is
`https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin`, which
GitHub redirects to `release-assets.githubusercontent.com`; reqwest follows that hop under its
default policy and the final response carries a real `Content-Length`.

The download is hashed as it streams. Before the `.tmp` is renamed into place,
`check_length(total, downloaded)` rejects a stream that ended short of (or past)
`Content-Length`, and `verify_digest(MODEL_SHA256, …)` rejects any digest other than the pinned
`1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` — the same value Hugging
Face publishes as the file's LFS hash (`x-linked-etag`) and GitHub reports as the asset's
`digest`. Either failure returns before the rename, so `ensure_model`'s existing failure path
removes the partial file and emits `model_download_error`.

Rejected alternatives: a configurable URL (a settings surface for a third host nobody has), an
ordered GitHub-then-Hugging-Face fallback (a silent host switch hides a network problem worth
surfacing), and bundling the model in the DMG (1.6 GB per merge to main).

## Implications

- Do not point `MODEL_URL` back at huggingface.co; the whole reason for the release is that
  some networks cannot reach it.
- Changing the model means uploading the new file to the `models` release **and** updating
  `MODEL_SHA256` in lockstep; the README restates the hash so a user placing the file by hand
  can verify it, and nothing checks that the copies agree.
- The checksum covers the download only. An already-cached file is still accepted on a bare
  `exists()` check (issue #16).
- The `models` release is a prerelease on purpose; see
  [[2026-09-06-constraint-make-latest-false-cannot-hide-the-only-full-release]].
- A second model asset (for example a VAD model) can live on the same release; the helpers
  `check_length` and `verify_digest` take their expectations as parameters so a generalised
  `ensure_file(url, filename)` can reuse them per file.

## Related

- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — the bare `exists()` trust this download-time checksum narrows but does not remove
- [[2026-09-05-reference-model-rs-download-paths-have-no-tests]] — partly outdated by this unit: `model.rs` now carries an inline test block for its pure helpers, while the network paths remain untestable because `MODEL_URL` is still a `const`
- [[2026-09-06-constraint-make-latest-false-cannot-hide-the-only-full-release]] — why the release is a prerelease
- [[2026-09-06-reference-an-isolated-home-gives-a-clean-first-launch]] — how the download was exercised end to end without touching the real cache
- [[2026-09-06-bug-webpki-only-roots-rejected-zscaler-tls-inspection]] — see also
