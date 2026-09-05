# Use rmcp (official Rust MCP SDK), pinned at 1.7

**Date**: 2026-05-18
**Type**: decision
**Summary**: depend on rmcp 1.7 (the official Rust MCP SDK) for protocol framing and streamable HTTP; the 1.4 floor is a DNS-rebinding CVE fix
**Context**: .minerva/work/2026-05-19-mcp-server

<!-- superseded-by: 2026-09-05-decision-strip-darric-to-a-recorder -->

> **Superseded by [[2026-09-05-decision-strip-darric-to-a-recorder]]** (2026-09-05)

## Context

darric needs to host an MCP server inside the running Tauri app so external AI tools (Claude Desktop, Claude CLI) can read its data over a localhost HTTP endpoint. Two viable shapes:

1. Use `rmcp` (the official `modelcontextprotocol/rust-sdk`) — macro-driven, includes streamable HTTP transport.
2. Hand-roll JSON-RPC over HTTP using axum directly. The existing MCP **client** code in `src-tauri/src/ai/mcp/mod.rs` hand-rolls the stdio client side in ~250 lines, so the precedent is there.

The streamable HTTP transport has nontrivial spec surface (single endpoint at `/mcp`, POST for requests, SSE for notifications, session-id negotiation). Prior to rmcp 1.4.0, the streamable HTTP server transport had a DNS rebinding CVE — the server didn't validate the incoming Host header, allowing a malicious public website to send authenticated requests to the loopback MCP endpoint. Even bound to 127.0.0.1, this class of attack is real.

## Decision

Depend on `rmcp = "1.7"` with features `server, macros, transport-streamable-http-server, schemars`. Pair it with `axum = "0.8"` for the HTTP listener (rmcp's own dev-deps target axum 0.8, so this is the supported pairing). Do not hand-roll the protocol.

## Consequences

- The MCP server's wire-level correctness, Host-header validation, and session management ride on rmcp. Future upgrades must stay at or above 1.4 for the CVE fix; current floor of 1.7 gives headroom.
- All MCP tool definitions go through rmcp's macro system (`#[tool_router]`, `#[tool(description = "...")]`, `Parameters<T>`). Tool input types must derive `serde::Deserialize` + `schemars::JsonSchema`.
- The dependency footprint is significant (axum, tower, hyper, sse-stream, schemars), but Tauri already pulls in much of this transitively.
- If rmcp ever materially breaks API in a major release, we re-evaluate hand-rolling — the existing MCP client code is the precedent and could be extended to a server. The trigger would be either a CVE without a fix or unmaintained-crate status.

## Related

- [[2026-05-19-decision-tool-handler-router-pattern]] — the macro system this pins is what forces the explicit router form
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — rmcp's `#[tool]` macro is what makes every handler an `async fn`
