# Use `#[tool_handler(router = self.tool_router)]`, not bare `#[tool_handler]`

**Date**: 2026-05-18
**Type**: decision
**Summary**: use `#[tool_handler(router = self.tool_router)]`, never the bare form — the bare form leaves the field dead and needs an `#[allow]` this repo forbids
**Context**: .minerva/work/2026-05-19-mcp-server

## Context

rmcp's `#[tool_router]` macro generates a `Self::tool_router()` static method that builds a `ToolRouter<Self>` from `#[tool(...)]`-annotated impl methods. The official rmcp examples (see `examples/servers/src/common/counter.rs`) follow this pattern:

```rust
pub struct Counter {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Counter>,
    ...
}

#[tool_router]
impl Counter {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),  // builds the router once
            ...
        }
    }
    #[tool(description = "...")]
    async fn increment(&self) -> ... { ... }
}

#[tool_handler]
impl ServerHandler for Counter { ... }
```

But the bare `#[tool_handler]` form defaults to calling `Self::tool_router()` (the static method) at request time, **not** reading the stored `self.tool_router` field. So the field is unused, and the Rust compiler emits a `dead_code` warning. The rmcp example masks this with a file-level `#![allow(dead_code)]`.

CLAUDE.md (in this repo) forbids any `#[allow(...)]` outside `#[cfg(test)] mod tests` blocks. We cannot silence the warning that way.

## Decision

Use the explicit attribute form on the `ServerHandler` impl:

```rust
#[tool_handler(router = self.tool_router)]
impl ServerHandler for DarricService { ... }
```

This binds the macro-generated routing logic to the instance field, so the field is actually read at request time, eliminating the dead-code warning without an `#[allow]`. It also avoids rebuilding the router per request.

## Consequences

- Anyone adding a new `#[tool_router]`-decorated service in this repo must use the `router = self.tool_router` attribute form. The bare form will trip the dead-code lint and CI policy.
- If we ever upgrade rmcp and the macro changes, this is the one spot to verify still compiles cleanly without an `#[allow]`. The rmcp 1.7 form is documented at the `tool_handler` macro doc in `~/.cargo/registry/src/.../rmcp-macros-*/src/lib.rs`.
- The stored `tool_router` field is now load-bearing — don't remove it on the assumption that the macro creates one for you.

## Related

- [[2026-05-19-decision-rmcp-as-mcp-sdk]] — depends on the rmcp pin chosen there
- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — also decided by this repo's no-`#[allow]`-outside-tests policy
