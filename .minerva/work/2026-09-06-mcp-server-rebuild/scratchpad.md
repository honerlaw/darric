# Scratchpad: mcp-server-rebuild

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06

- [decided] scope check: single unit, one PR, no phases — decided under human gates in `minerva:explore` → `minerva:propose` earlier this session (user chose in-app server over standalone binary; always-on fixed port; status tool included)
- [decided] approach: A, shared query layer + server's own read-only SQLite connection — decided under human gates in `minerva:propose` (rejected B: self-contained SQL duplicating the transcript SELECT; rejected C: FTS5 from day one, no measured need)
- [reviewed — folded] whole-proposal soundness: Skeptic accept with 6 concerns — folded 1 (rowid renumbering trigger is a rebuild migration like 010, not just VACUUM; cursors now scoped to the app process), 2 (protocol test must insert through a concurrent writer after the first page and assert the cursor sees it), 3 (rowid order is transcription-completion order; description points at recorded_at), 4 (LiveStatus returns engine snapshot only; status fills topic/started_at from the read-only connection); proceeded past 5 (seq on the UI type is disclosed scope) and 6 (no port retry is the chosen design)
- [rechecked — residual folded] whole-proposal soundness: fold-audit accept, items 1–4 addressed, 5–6 left as proceed-past; residual was the sentence implying every startup rebuilds the table — reworded to the one launch that applies a new migration
- [reviewed — clean] completion verification: Verifier accept on all five criteria (reproduced cargo test 57/57, vitest 100/100, clippy/tsc/eslint clean, no suppressions outside cfg(test)); live parts of C1/C2/C4 unverified by it but covered by this session's debug-binary run
- [decided] review triage: 7 FIX / 3 SUGGEST / 0 IGNORE, none contested (solo gate); the medium finding (empty page returned no cursor, restarting an agent's poll) was the one worth the pass

## Work notes 2026-09-06

- **rmcp 3.2 pins reqwest 0.13; the app's model downloader is on reqwest 0.12.** The protocol
  round-trip test needs a `reqwest::Client` that implements rmcp's `StreamableHttpClient`, which
  is only the 0.13 one, so it comes in as a renamed dev-dependency (`reqwest13`). The lockfile
  already carried both versions through rmcp, so this adds no new duplicate.
- **Bind synchronously in `setup`, adopt into Tokio inside the serve future.** `setup` is not
  inside a runtime, so `tokio::net::TcpListener::from_std` would panic there; a std listener set
  non-blocking and converted inside the spawned future avoids a "starting" state the UI would
  otherwise have to poll through. `serve()` returns the handle and the future separately so a
  test spawns on plain Tokio and the app on `tauri::async_runtime`.
- **`seq` on the UI `TranscriptLine` is `number | null`, not `number`.** Lines the transcript
  hook appends live from a `transcript_chunk` are never read back from the database until the
  transcript reloads, so they have no rowid. Making the chunk carry one would need
  `last_insert_rowid` on the insert path for a value the UI does not use. Deviation from the
  proposal's `seq: number`.
- **`search` returns `{ sessions, lines }`, not one `hits[]`.** A topic match returning every
  line of that session is not what "search over content and topic" means; the session is the
  hit. Deviation from the proposal's table; the tool description states the shape.
- **`@testing-library/user-event` replaces `navigator.clipboard` on `setup()`.** A `writeText`
  stub installed in `beforeEach` is silently swapped out, so a clipboard assertion in a
  user-event test must `vi.spyOn(navigator.clipboard, "writeText")` after `userEvent.setup()`.
- **`db::test_db()` replaces the per-file `include_str!` migration chains.** Built from
  `migrations::migrations()`, so a new migration cannot leave a test schema behind — the
  refactor the 2026-05-19 inline-tests decision said to do if the duplication bit.
- **Live verification (debug binary, real database):** raw streamable-HTTP handshake and
  `tools/list` return exactly `get_transcript`, `list_sessions`, `search`, `status`; `status`
  and `list_sessions` answered from real recordings; `Host: evil.example` got 403 from rmcp's
  default allowlist; `claude mcp add --transport http … ` then `claude mcp list` showed
  `✔ Connected` (entry removed again); a second instance launched while the first held 27842
  logged `not started: … Address already in use` and stayed up.

## Review triage 2026-09-06

Mode: local-diff (fresh-context subagent); minerva audit inline.

- [SUGGESTED] #1 low proposal table — `search` returns `{sessions, lines}` not `hits[]` (merge into proposal)
- [SUGGESTED] #2 low src/types/index.ts — UI `TranscriptLine.seq` is `number | null` (merge into proposal)
- [FIXED] #3 med src-tauri/src/db/sessions.rs — empty page returns `next_cursor: null`; agent told to pass it back restarts the walk from line 1
- [FIXED] #4 low src-tauri/src/db/sessions.rs — query folded with Unicode `to_lowercase`, columns with SQLite ASCII `lower()`; "über" misses "ÜBER"
- [FIXED] #5 low src/components/McpChip.tsx — every non-listening state reads "port busy", including a read-only open failure
- [FIXED] #6 low src-tauri/src/mcp_server/mod.rs — bind-failure path and `status` with no engine have no test
- [FIXED] #7 low src/components/layout/Header.test.tsx — Header→McpChip wiring only tested hidden
- [FIXED] #8 low README.md — "cannot slow the recorder" overstates; "cannot block" is what the design gives
- [FIXED] #9 low src-tauri/src/db/mod.rs — `SQLITE_OPEN_URI` on a plain path is dead configuration
- [SUGGESTED] #10 low design — the loopback endpoint is reachable by any local account, a broader exposure than the 0700 database file; accepted tradeoff, belongs in the proposal

- Review fix: src-tauri/src/db/sessions.rs — an empty page echoes the caller's cursor instead of `None`
- Review fix: src-tauri/src/db/sessions.rs — query folded with `to_ascii_lowercase` to match SQLite's ASCII `lower()`
- Review fix: src-tauri/src/mcp_server/mod.rs, lib.rs, commands/mcp_server.rs, McpChip.tsx — `PortBusy` split from `Failed`; chip reads "off" for non-port failures
- Review fix: tests for bind refusal, `status` with no engine, and Header→McpChip wiring
- Review fix: README — "hold up the recorder's writes" not "slow the recorder"; local-account reach, ASCII search, transcription order stated
- Review fix: src-tauri/src/db/mod.rs — dropped `SQLITE_OPEN_URI`

## Review finding 2026-09-06

- `search` returns `{ sessions, lines }` rather than the proposal table's single `hits[]`: a topic match is a session, not every line of it.
- The UI `TranscriptLine.seq` is `number | null`, not `number`: lines appended live from a `transcript_chunk` have no rowid until the transcript reloads.
- The loopback endpoint is reachable by any local account on the machine, a broader exposure than the user-owned database file; the design accepted that, and the README says so.
