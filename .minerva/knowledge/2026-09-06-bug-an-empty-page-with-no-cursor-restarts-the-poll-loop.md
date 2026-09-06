# An empty page that returns no cursor restarts the poll loop from the beginning

**Date**: 2026-09-06
**Type**: bug
**Summary**: `get_transcript` returned `next_cursor: null` on an empty page while telling the agent to pass `next_cursor` back, so a quiet poll restarted the transcript from line one; an empty page now echoes the caller's cursor
**Context**: .minerva/work/2026-09-06-mcp-server-rebuild (see git history if the worktree has been cleaned up)

## Context

The live-meeting flow is: fetch a page, keep `next_cursor`, poll later with `after` set to it,
get only what landed since. The first implementation computed `next_cursor` as
`lines.last().map(|l| l.seq)` — the last row's rowid, or `None` when the page was empty — and
documented in the Rust type that "the caller's previous cursor is still the right one" in that
case. The tool description, which is all an agent reads, said only "pass `next_cursor` back as
`after`". The unit test asserted the `None`.

## Finding

On the poll with nothing new — the common case in a meeting, and the one with the least to
say — an agent following the description passes `after: null` and receives the first 500 lines
of the transcript again with a fresh cursor. Nothing is lost, but the feature's headline
behaviour is defeated exactly when it matters. Found by the fresh-context code review; the
completion Verifier had passed it because the protocol test only ever paged a non-empty
result.

Fix: `next_cursor = lines.last().map(|l| l.seq).or(after)`. An empty page echoes the cursor it
was asked from, so a poll loop can always feed the value straight back; `None` now means only
"empty transcript, read from the start". The test asserts the echo, and the tool description
says so.

## Implications

- A cursor API's "nothing new" response must be a valid input to the next call. Document the
  contract where the caller reads it — for an MCP tool that is the description string, not a
  Rust doc comment.
- A paging test that never pages an empty result has not tested the loop's steady state.

## Related

- [[2026-09-06-constraint-a-table-rebuild-renumbers-transcript-rowids-and-every-mcp-cursor]] — the other way a cursor goes wrong
- [[2026-09-06-decision-mcp-server-rebuilt-in-process-on-rmcp-3]] — the tool this is in
- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — the same shape: the verified path was correct and the unverified branch was the one in use
