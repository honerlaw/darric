# Knowledge index

## Decisions

- [[2026-05-19-decision-inline-tests-for-mcp-queries]] — MCP query tests live inline under `#[cfg(test)] mod tests`; `tests/` cannot reach the crate-internal `mcp_server` module without making it `pub`
- [[2026-05-19-decision-rmcp-as-mcp-sdk]] — depend on rmcp 1.7 (the official Rust MCP SDK) for protocol framing and streamable HTTP; the 1.4 floor is a DNS-rebinding CVE fix
- [[2026-05-19-decision-spawn-blocking-for-rusqlite-tools]] — every MCP tool handler dispatches its rusqlite query through `tokio::task::spawn_blocking` — don't strip it, it prevents runtime stalls and satisfies `unused_async`
- [[2026-05-19-decision-tool-handler-router-pattern]] — use `#[tool_handler(router = self.tool_router)]`, never the bare form — the bare form leaves the field dead and needs an `#[allow]` this repo forbids
- [[2026-09-05-decision-strip-darric-to-a-recorder]] — darric is now only a recorder — the AI chat, MCP client, MCP server, notes, tasks, tags, search and board features were deleted outright, retiring the four MCP decisions with them

## Bugs

- [[2026-09-05-bug-arc-try-unwrap-after-sharing-fails-silently]] — reclaiming ownership with `Arc::try_unwrap(x).ok()` after clones have been handed out fails deterministically, and `.ok()` turns that failure into a plausible `None` that disables the feature with no error
- [[2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file]] — startup pre-load and `load_transcriber` both called `ensure_model`, writing one `.tmp` at independent offsets; the mixed result was renamed in and accepted by a bare `exists()` check on every later launch
- [[2026-09-05-bug-forgetting-a-block-leaked-it-and-masked-a-use-after-free]] — `mem::forget` on a block Core Audio had already `Block_copy`d leaked its whole captured environment — and because the refcount could then never reach zero, it also masked a teardown race that correcting the leak alone would have re-exposed
- [[2026-09-05-bug-phase-progress-misreads-squash-merged-phases]] — `phasing.md` feeds `phase_progress()` from `git branch --merged`, which cannot see a squash-merged branch — so a phase that has shipped reads as pending and would be re-shipped forever
- [[2026-09-05-bug-prettierignore-misses-generated-tauri-schemas]] — `.prettierignore` did not exclude the git-ignored `src-tauri/gen/` Tauri schemas, so `npm run format` failed locally for anyone who had run a build — while CI stayed green because it formats before those files exist

## Patterns

- [[2026-09-05-pattern-an-early-return-can-make-a-feature-unreachable]] — the model-download indicator was complete and correct but rendered below a `session === null` early return, so it could not run in the only state it existed for
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — rewriting a screen from its rendered shape carries the visible markup across but loses per-selection state resets and cross-entity guards — the parts with no visual counterpart
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — a completion check confirmed the flush/shutdown ordering inside `stop()` was correct, and it was — but the `if let Some(pool)` guard above it was never true, so none of the verified steps executed

## Constraints

- [[2026-09-05-constraint-phases-must-use-the-canonical-list-form]] — `read_phases` stops at the next `#` line, so writing phases as `### 1. Name` subsections parses as zero phases — the unit ships as unphased and its later phases are stranded with no error
- [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]] — `emit` only reaches webviews already holding a listener, so anything emitted during `setup()` is lost and needs a command the frontend can poll on mount

## References

- [[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]] — `AudioHardwareCreateProcessTap` succeeds without the audio-recording permission and only `AudioDeviceStart` fails, so error handling written around creation reports success on a tap that can never deliver audio
- [[2026-09-05-reference-claude-md-symlinks-to-agents-md]] — CLAUDE.md is a symlink, not a copy — a tool that replaces rather than follows it silently forks the two agent files
- [[2026-09-05-reference-clippy-ceiling-configured-not-enforced]] — `AGENTS.md` calls `all=deny` + pedantic + nursery + cargo "the ceiling", but only `all` and `correctness` are deny — pedantic and nursery are warn, and no lint command passes `-D warnings`, so those violations stay green
- [[2026-09-05-reference-knowledge-corpus-not-ci-gated]] — `check.yml` runs TypeScript/Rust builds and tests only — nothing runs `knowledge_lint.py`, so index drift and broken `[[…]]` wikilinks reach `main` green
- [[2026-09-05-reference-knowledge-wiki-is-ci-gated]] — `check.yml`'s `Knowledge Wiki` job runs `knowledge_lint.py` on every pull request and every push to main — index drift and broken wikilinks are errors, uncatalogued entries and missing reciprocals are warnings
- [[2026-09-05-reference-matchmedia-stub-pins-tests-to-light-mode]] — `src/test/setup.ts` installs a never-restored `matchMedia` returning `matches: false` with a no-op change listener, so `App`'s dark branch and its listener lifecycle go unexercised
- [[2026-09-05-reference-model-rs-download-paths-have-no-tests]] — `MODEL_URL` is a hard-coded `const`, so no Rust test can reach the status, mid-stream, rename, cleanup or serialisation paths; `model.rs` is the one module of seven with no test block
- [[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]] — measured 1.14x speedup from 4x the threads against one shared `WhisperContext`, so pool size buys almost nothing and the queue's overflow policy is what protects a recording
