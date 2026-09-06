# Scratchpad: whisper-model-github-release

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06

- [decided] pre-flight: no collision. Two units read `in_flight` true (`strip-to-recorder` still says Status Draft, `transcriber-single-flight` lacks a marker) but all their PRs are MERGED — false positives. Live peers: `darric-72` (list-inline-rename, PR #45) and `darric-3f` (transcript-accuracy; will also touch model.rs to add `ensure_file(url, filename)`) — both adjacent, not collisions. Open issue #16 (validate cached model) is adjacent, not adopted.
- [reviewed — clean] scope check: one unit, one PR, unphased (Skeptic accept). Noted: merge-order precondition (asset live before PR), sibling-branch sequencing, license provenance — all carried into the proposal text as criteria/notes; verified the HF model card declares `license: mit`.
- [reviewed — clean] approach: A (GitHub Release asset + pinned SHA-256) over B configurable URL / C ordered fallback / D bundle in DMG (Skeptic accept). Folded as text corrections: release assets redirect to `release-assets.githubusercontent.com` (not objects.\*); digest check placed before rename; truncation check guarded on `total > 0`.
- [reviewed — clean] whole-proposal soundness: accept. Folded as text: success-path log line naming the digest; the not-latest criterion now checks `isLatest` directly instead of list order.

## Work notes 2026-09-06

- `gh release create … --latest=false` did not keep `models` off the Latest badge: every other
  release in this repo is a prerelease, and GitHub's "latest" is the newest non-prerelease,
  so the only full release is latest by definition regardless of `make_latest`. `gh release
edit models --prerelease` fixed it; `releases/latest` is 404 again as before. Candidate
  constraint entry.
- Upload of the 1.55 GB asset via `gh release upload` took 77 s. The asset redirect goes to
  `release-assets.githubusercontent.com` (not `objects.githubusercontent.com`).
- E2E ran the debug binary under an isolated `HOME` (`default_model_path` and the DB both hang
  off `$HOME`), so the real app's cache and the sibling session's whisper tests were untouched.
  The MCP bind failed with port-busy (real app running) and the download proceeded anyway.
