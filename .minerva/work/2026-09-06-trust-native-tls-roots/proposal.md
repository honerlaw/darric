# Proposal: trust-native-tls-roots

**Date**: 2026-09-06
**Status**: Draft

## Goal

Make the model download work on machines whose network inspects TLS: trust the macOS
keychain's root certificates in addition to Mozilla's bundle, and log the full error chain
so a connection-level failure names its actual cause.

## Why

On the user's work Mac over VPN, Zscaler re-signs every HTTPS connection: `openssl s_client`
to github.com shows the issuer "Zscaler Intermediate Root CA (zscalertwo.net)". The
Zscaler root is installed in the macOS keychain, so curl, Safari and the DMG download all
work there. The app's HTTP client is built with reqwest's `rustls-tls` feature, which trusts
only the `webpki-roots` Mozilla bundle, so it rejects that chain before any HTTP happens and
reports only "model download request failed: error sending request for url (…)" — reqwest's
`Display` never prints the underlying cause. There is no system proxy on that machine
(`scutil --proxy` shows everything disabled), and curl reaches both the GitHub release asset
(200) and huggingface.co (302), so Hugging Face was never blocked; the earlier failure there
was this same certificate rejection. See [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]].

## Approach

1. **`src-tauri/Cargo.toml`.** Add `rustls-tls-native-roots` to reqwest's features and keep
   `rustls-tls`. With both on, reqwest fills the root store from the platform store via
   `rustls_native_certs::load_native_certs()` **and** from `webpki-roots`, so a machine whose
   keychain load fails still trusts the public roots. This pulls `rustls-native-certs` and
   `security-framework` into the lock; no other code changes for it.
2. **`src-tauri/src/model.rs`.** Add a small `error_chain(&dyn Error) -> String` helper that
   joins an error's `Display` with every `source()` down the chain, and use it in the two
   `reqwest` `map_err`s (the request and the body stream), so the log and the UI error bar say
   e.g. "error sending request for url (…): client error (Connect): invalid peer certificate:
   UnknownIssuer" instead of stopping at the first clause. Unit-test the helper with a nested
   error.
3. **README.** One sentence: the download trusts the system keychain, so corporate TLS
   inspection works as long as its root is installed there.

### Candidate approaches considered

- **A — `rustls-tls-native-roots` alongside `rustls-tls` (chosen).** Smallest change; same
  TLS stack; union of roots.
- **B — Switch to `native-tls` (SecureTransport on macOS).** Also trusts the keychain, but
  swaps the whole TLS stack and its dependency tree for one feature's worth of benefit.
- **C — Accept invalid certificates or pin the Zscaler CA.** Rejected outright: the first
  disables TLS verification, the second bakes one company's proxy into the app.

## Success criteria

- `src-tauri/Cargo.toml` lists both `rustls-tls` and `rustls-tls-native-roots` for reqwest,
  and `Cargo.lock` contains `rustls-native-certs`.
- `error_chain` joins Display and every `source()` with `: `; covered by a unit test on a
  two-level nested error, and used by both reqwest `map_err`s in `download`.
- The download still works where it worked before: the debug binary under an isolated `HOME`
  on this machine downloads the model and logs `checksum verified`.
- README states that the download trusts the system keychain.
- `npm run check` and `cargo test` pass; no lint suppression added.

## Open Questions

- The definitive check — the rebuilt app downloading the model on the work Mac behind
  Zscaler — can only be run there, from the next `main-*` DMG. If it still fails, the new
  error chain will name why.
