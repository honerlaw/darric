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
- [decided] completion verification: all 5 criteria met — Cargo.toml lists both root features and Cargo.lock has rustls-native-certs 0.8.3; `error_chain` has 2 tests and is used by both reqwest `map_err`s (request, body stream); isolated-HOME e2e on this machine downloaded 1549 MB, logged `checksum verified`, file hashed 1fc70f77…; README sentence added; `npm run check` chain, clippy (0 warnings after `Option<Box<Self>>`), 88 lib tests, 118 vitest pass; diff grep for suppressions: none. Work-Mac run remains an Open Question.

## Work notes 2026-09-06

- `npm run check` stopped at `npm run format`: `.minerva/knowledge/2026-09-06-reference-say-hangs-under-cargo-test-unless-stdin-is-closed.md` on main is not Prettier-clean. It reached main through the reconciliation merge (#52), which fires no CI (`2026-09-05-reference-github-token-actions-trigger-no-workflows`), so main is red on format without anyone seeing it — issue #44's shape. Formatted it in this PR (one line).
- Nursery `use_self` fired on the test-only `Nested { cause: Option<Box<Nested>> }`; `Box<Self>` fixes it. Pedantic/nursery are warn-only here, but CLAUDE.md says fix, never suppress.

## Review triage 2026-09-06

- [decided] review triage: 4 findings from the local-diff code review — 2 FIX / 0 SUGGEST / 2 IGNORE, none contested. Minerva audit: no spec or knowledge findings.
- [FIXED] #1 low Cargo.toml / README — `SSL_CERT_FILE`/`SSL_CERT_DIR` make rustls-native-certs skip the keychain; documented in both
- [IGNORED] #2 low model.rs — keychain enumeration is blocking I/O in `Client::build`; once per multi-minute download, negligible
- [IGNORED] #3 low model.rs — explicit `as &(dyn StdError + 'static)` cast could be inferred; cosmetic
- [FIXED] #4 low model.rs — doc comment said the chain is "the whole diagnosis"; softened
- Review fix: src-tauri/Cargo.toml, README.md, src-tauri/src/model.rs — env-override caveat documented, doc comment softened
- [decided] promote partition: 1 PROMOTE (bug: webpki-only roots vs Zscaler), review fixes MERGED into proposal, decisions/triage/use_self note DISCARDED, the format-on-main note DISCARDED as already covered by #44 and the GITHUB_TOKEN reference entry, 0 TODO, no issue closed (solo gate)
