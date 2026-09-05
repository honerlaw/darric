# `CLAUDE.md` is a symlink to `AGENTS.md`

**Date**: 2026-09-05
**Type**: reference
**Summary**: CLAUDE.md is a symlink, not a copy — a tool that replaces rather than follows it silently forks the two agent files
**Context**: .minerva/work/2026-09-05-knowledge-wiki-migration

## Context

At the repo root, `CLAUDE.md` is a symbolic link to `AGENTS.md`:

```
lrwxr-xr-x  CLAUDE.md -> AGENTS.md
```

One file, two names, so Claude Code and other agent harnesses read the same instructions.

## Finding

Edits must go to `AGENTS.md`, or through a tool that follows the link. An editor that _replaces_
the path — writing a whole new file at `CLAUDE.md` rather than modifying the target — breaks the
link and leaves two independent files that immediately begin to diverge. Nothing detects this:
both paths still exist and still contain plausible instructions, so the next reader of
`AGENTS.md` sees stale content with no signal that a second copy is now authoritative for Claude.

## Implications

- Prefer editing `AGENTS.md` directly; it is the real file.
- After any tool-driven write to `CLAUDE.md`, confirm the link survived with `ls -la CLAUDE.md`.
- The same applies to `GEMINI.md` if it is ever added under the same convention.

## Related

- [[2026-09-05-reference-knowledge-corpus-not-ci-gated]] — the other unguarded fact about this repo's agent-records layer
