# Scratchpad: trust-native-tls-roots

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Quick decisions 2026-09-06

- [decided] pre-flight: no collision. No open PRs, no unit branches; the two `in_flight` units (`strip-to-recorder`, `transcriber-single-flight`) are the same merged false positives as this morning. One live darric peer (`darric-3f`, idle) pinged; reply not awaited. No open issue matches (none concern TLS or the downloader).
- [decided] scope check: one unit, one PR, unphased — two files plus a README sentence.
- [decided] approach: A (add `rustls-tls-native-roots`, keep `rustls-tls`, log the error chain). Rejected B `native-tls` (swaps the TLS stack for the same benefit) and C accept-invalid / pin Zscaler CA (disables verification / bakes in one proxy).
- [decided] whole-proposal soundness: approach reaches the goal; every criterion is checkable here except the work-Mac run, which is named as an Open Question rather than claimed.
