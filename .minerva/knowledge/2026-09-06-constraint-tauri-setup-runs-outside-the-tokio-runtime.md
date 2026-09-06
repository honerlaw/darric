# Tauri's `setup` runs outside the Tokio runtime, so adopt sockets inside the spawned task

**Date**: 2026-09-06
**Type**: constraint
**Summary**: `tokio::net::TcpListener::from_std` panics in `setup()` because no runtime is entered there; bind a std listener synchronously and convert it inside the `tauri::async_runtime::spawn`ed future
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

The MCP server needs its bind outcome known before `setup` returns, so the header chip can
read it with one command on mount instead of polling through a "starting" state. Binding
inside the spawned task would have made the outcome asynchronous; binding with Tokio in
`setup` is not possible.

## Finding

`setup` runs on the main thread with no Tokio runtime context entered, and every Tokio
primitive that registers with the reactor — `TcpListener::from_std`, `TcpListener::bind` —
requires one. `mcp_server::bind` therefore binds a `std::net::TcpListener`, sets it
non-blocking, and returns it; `mcp_server::serve` returns the handle and a future, and the
future calls `from_std` as its first step, where the runtime that spawned it is guaranteed
to be current. The caller chooses the runtime: `tauri::async_runtime::spawn` in the app,
`tokio::spawn` in a test.

## Implications

- Anything that must be decided in `setup` and needs a socket does the OS-level work with
  `std::net` and hands the handle to a task.
- Returning the future rather than spawning it inside the library function is what keeps the
  module testable on plain Tokio without a Tauri app.
- `set_nonblocking(true)` is not optional: Tokio refuses a blocking std listener.

## Related

- [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]] — the other thing `setup` cannot do, and the reason the outcome is polled rather than emitted
- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the server that hit this
