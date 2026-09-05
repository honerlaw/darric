# Knowledge index

## Decisions

- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — MCP query tests live inline under `#[cfg(test)] mod tests`; `tests/` cannot reach the crate-internal `mcp_server` module without making it `pub`
- [[2026-05-19-decision-rmcp-as-mcp-sdk]] — depend on rmcp 1.7 (the official Rust MCP SDK) for protocol framing and streamable HTTP; the 1.4 floor is a DNS-rebinding CVE fix
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — every MCP tool handler dispatches its rusqlite query through `tokio::task::spawn_blocking` — don't strip it, it prevents runtime stalls and satisfies `unused_async`
- [[2026-05-19-decision-tool-handler-router-pattern]] — use `#[tool_handler(router = self.tool_router)]`, never the bare form — the bare form leaves the field dead and needs an `#[allow]` this repo forbids

## Bugs

## Patterns

## Constraints

## References

- [[2026-09-05-reference-claude-md-symlinks-to-agents-md]] — CLAUDE.md is a symlink, not a copy — a tool that replaces rather than follows it silently forks the two agent files
- [[2026-09-05-reference-knowledge-corpus-not-ci-gated]] — `check.yml` runs TypeScript/Rust builds and tests only — nothing runs `knowledge_lint.py`, so index drift and broken `[[…]]` wikilinks reach `main` green
