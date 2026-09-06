# Scratchpad: whisper-model-github-release

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06

- [decided] pre-flight: no collision. Two units read `in_flight` true (`strip-to-recorder` still says Status Draft, `transcriber-single-flight` lacks a marker) but all their PRs are MERGED — false positives. Live peers: `darric-72` (list-inline-rename, PR #45) and `darric-3f` (transcript-accuracy; will also touch model.rs to add `ensure_file(url, filename)`) — both adjacent, not collisions. Open issue #16 (validate cached model) is adjacent, not adopted.
- [reviewed — clean] scope check: one unit, one PR, unphased (Skeptic accept). Noted: merge-order precondition (asset live before PR), sibling-branch sequencing, license provenance — all carried into the proposal text as criteria/notes; verified the HF model card declares `license: mit`.
- [reviewed — clean] approach: A (GitHub Release asset + pinned SHA-256) over B configurable URL / C ordered fallback / D bundle in DMG (Skeptic accept). Folded as text corrections: release assets redirect to `release-assets.githubusercontent.com` (not objects.*); digest check placed before rename; truncation check guarded on `total > 0`.
- [reviewed — clean] whole-proposal soundness: accept. Folded as text: success-path log line naming the digest; the not-latest criterion now checks `isLatest` directly instead of list order.
