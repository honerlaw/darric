# rmcp 3's streamable-HTTP client is implemented for reqwest 0.13, not the app's 0.12

**Date**: 2026-09-06
**Type**: reference
**Summary**: `StreamableHttpClient` is implemented only for reqwest 0.13's `Client`; the model downloader is on 0.12, so the protocol test uses a renamed `reqwest13` dev-dependency
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

The protocol round-trip test drives the server with rmcp's own client
(`transport-streamable-http-client-reqwest`). The first attempt passed
`reqwest::Client::new()` and failed with "the trait `StreamableHttpClient` is not implemented
for `reqwest::Client`", with a help line pointing at `reqwest::async_impl::client::Client` —
the same type name from a different crate version.

## Finding

rmcp 3.2 depends on `reqwest = "0.13.2"` and implements its client trait for that version's
`Client`. darric's `model.rs` downloader uses `reqwest = "0.12"` with `rustls-tls` and
`stream`. Cargo keeps both, so `Cargo.lock` already carried 0.12 and 0.13 through rmcp before
the test existed. The test takes the 0.13 one under a rename:

```toml
[dev-dependencies]
reqwest13 = { package = "reqwest", version = "0.13", default-features = false }
```

and calls `reqwest13::Client::new()`. `multiple_crate_versions` is already allowed in the
crate's lint table.

## Implications

- Bumping the app's own reqwest to 0.13 would let the rename go, but 0.13 changed its TLS
  feature names, so that is a change to the download path, not a test tidy-up.
- Any other rmcp client-side use in production code would face the same choice.

## Related

- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the dependency this rides on
