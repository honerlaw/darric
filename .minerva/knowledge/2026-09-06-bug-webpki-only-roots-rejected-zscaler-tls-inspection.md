# A webpki-only root store made every download fail behind Zscaler, reported as "error sending request"

**Date**: 2026-09-06
**Type**: bug
**Summary**: reqwest's `rustls-tls` feature trusts only Mozilla's bundle, so a network that re-signs TLS with a keychain-only root fails the handshake and reqwest's Display hides the cause
**Context**: .minerva/work/2026-09-06-trust-native-tls-roots (see git history if the worktree has been cleaned up)

## Context

The model download failed on the user's work Mac over VPN with
"model download request failed: error sending request for url (https://github.com/…)". The
same error had appeared earlier against huggingface.co and was read as that host being
blocked, which motivated mirroring the model on a GitHub Release
([[2026-09-06-decision-whisper-model-served-from-the-models-github-release]]). The mirror
did not fix it.

## Finding

Diagnostics run on that machine showed no system proxy, but `openssl s_client` to github.com
returned issuer "Zscaler Intermediate Root CA (zscalertwo.net)", and curl reached both the
GitHub asset (200) and huggingface.co (302). Zscaler re-signs every TLS connection; its root
is installed in the macOS keychain, which curl and browsers consult. The app's reqwest client
was built with `default-features = false` and `rustls-tls`, which resolves to
`rustls-tls-webpki-roots`: Mozilla's bundle only. rustls rejected the Zscaler chain as
`UnknownIssuer` before any HTTP happened, and reqwest's `Display` prints only its own layer —
"error sending request for url (…)" — leaving the cause in `source()`, which nothing printed.

The fix adds `rustls-tls-native-roots` alongside `rustls-tls`. reqwest 0.12 then fills one
root store from both `webpki-roots` and `rustls_native_certs::load_native_certs()`, a union;
an empty keychain load falls through to bundle-only behaviour. A new `error_chain` helper
joins `Display` with every `source()` in both reqwest `map_err`s so the log and error bar
now name the cause.

Hugging Face was never blocked. The mirror remains worthwhile for the pinned checksum and
a host the project controls, but it was not the fix.

## Implications

- Any Rust HTTPS client in this app must trust the platform store, not just a bundled one,
  or it fails on corporate networks that inspect TLS. Check `cargo tree -e features` for
  `rustls-tls-native-roots-no-provider` after touching reqwest's features.
- `rustls-native-certs` reads `SSL_CERT_FILE` / `SSL_CERT_DIR` _instead of_ the keychain when
  either is set, so a shell that exports them can reproduce the failure even after this fix.
- "error sending request" from reqwest is never the diagnosis. Print the `source()` chain
  before reasoning about the network; the earlier "blocked host" conclusion came from
  trusting the top-level message.
- Keychain enumeration in `Client::build` is synchronous; once per download it is
  negligible, but it does not belong on a hot path.

## Related

- [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]] — the mirror built on the mistaken "blocked host" reading; still useful, not the fix
- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — the other way this download path failed silently
- [[2026-09-06-reference-an-isolated-home-gives-a-clean-first-launch]] — how the fixed download was exercised end to end here
