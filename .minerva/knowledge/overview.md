# Knowledge overview

## Hosting an MCP server inside the Tauri app

Everything darric knows so far comes from one piece of work: turning the app into a
read-only MCP server so external clients (Claude Desktop, Claude CLI) can query a user's
own notes, meetings and tasks without darric spending its own token budget.

The foundational call was to depend on the official Rust SDK rather than hand-roll the
protocol — [[2026-05-19-decision-rmcp-as-mcp-sdk]]. The deciding factor was not
convenience but a security floor: streamable HTTP before rmcp 1.4 did not validate the
`Host` header, which leaves a loopback-bound endpoint open to DNS rebinding from any web
page the user visits. Binding to `127.0.0.1` is not sufficient protection against that
class of attack, so the pin carries a hard lower bound, not just a preference.

That one dependency then propagates into the shape of everything built on it. Because
rmcp's `#[tool]` macro requires handlers to be `async fn`, and the query layer underneath
is synchronous `rusqlite`, every handler has to cross an async/blocking boundary —
[[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] is the resulting rule. It is
worth reading before "simplifying" a handler: the `spawn_blocking` wrapper looks like
ceremony and is not. An external agent in a long loop can hammer these tools far harder
than the UI ever does, and a blocking query on the runtime degrades transcription and
audio capture, not just the request that caused it.

## When the lint policy is the architecture

Two of the four MCP decisions were not really about MCP. They were forced by this repo's
rule in `AGENTS.md` that `#[allow(...)]` is forbidden outside `#[cfg(test)]` blocks — a
policy strict enough that it removes options other projects would take without noticing.

[[2026-05-19-decision-tool-handler-router-pattern]] is the clearest case. The official
rmcp example uses a bare `#[tool_handler]` and silences the resulting `dead_code` warning
with a file-level `#![allow(dead_code)]`. That escape hatch is unavailable here, so the
explicit `router = self.tool_router` form became the only viable path — and it happens to
be better anyway, since it stops rebuilding the router on every request.

[[2026-05-19-decision-inline-tests-for-mcp-queries]] runs the same way. Reaching the query
functions from `tests/` would have meant making the `mcp_server` module `pub`, widening the
crate's public API purely to accommodate tests. Inline `#[cfg(test)]` modules avoid that,
and the lint policy's one carve-out — `#[allow(clippy::unwrap_used)]` inside test modules —
exists precisely to make them practical. The cost is real and recorded: migration-loading
is now duplicated between the integration-test helper and the inline module, and both must
be updated when a migration lands.

The pattern worth carrying forward: when a decision here looks unusual, check whether the
lint policy eliminated the conventional option before assuming the author preferred this
one.

## What the records layer does not protect

Two entries describe the agent-records layer itself rather than the product, and both say
the same kind of thing — a load-bearing arrangement that nothing checks.

[[2026-09-05-reference-knowledge-corpus-not-ci-gated]] notes that `check.yml` builds and
tests TypeScript and Rust but never runs the knowledge detector, so a dropped catalog line
or a broken wikilink reaches the default branch with CI green. The corpus described by
this very overview is therefore maintained by convention, not enforcement.

[[2026-09-05-reference-claude-md-symlinks-to-agents-md]] is the smaller, sharper version:
`CLAUDE.md` is a symlink to `AGENTS.md`, and a tool that replaces the path rather than
following it forks the two files silently. Both then look plausible and drift apart.

Neither is a defect in shipped behavior, which is why both are standing facts rather than
tracker issues. They are the things most likely to be broken by accident by someone acting
reasonably.

## Limitations

A link in this overview attests **synthesis intent**, not body content: an entry can stay
linked from a narrative that no longer describes it, and nothing detects that. Re-read the
entry itself before relying on a claim made here.

Entries promoted after this synthesis will show as `unsynthesized` until the next refresh,
so a theme missing from this page is not evidence the corpus lacks it — check
[[2026-09-05-reference-knowledge-corpus-not-ci-gated]] for why nothing mechanically keeps
this page honest either. `index.md` remains the complete catalog; this page is only a way
in.
