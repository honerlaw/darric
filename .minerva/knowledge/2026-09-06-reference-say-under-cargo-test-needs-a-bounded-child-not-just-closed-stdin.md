# `say` under cargo test needs a bounded child, not just closed stdin

**Date**: 2026-09-06
**Type**: reference
**Summary**: `say` hung a second time with stdin already null; the fixture generator now polls the child, kills it after 30 s and retries once, and synthesizes the speech once per test process
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

[[2026-09-06-reference-say-hangs-under-cargo-test-unless-stdin-is-closed]] recorded that
closing stdin made `say` return. With stdin closed it returned in about two seconds for
several runs — then sat forever once more, at 0.5 s of CPU, during a sequential run of three
model-dependent tests, while a shell invocation alongside it returned in a second.

## Finding

Closed stdin is necessary but not sufficient; whatever the synthesizer waits on is not
reproducible from a shell. `transcription::fixture::run_bounded` spawns `say` and `afconvert`
with stdin null, polls `try_wait` every 50 ms, kills the child after 30 s, and retries once
before panicking with "hung twice". `spoken()` caches the synthesized samples in a `OnceLock`
so a test process invokes `say` once however many tests need the fixture.

## Implications

- Any test that shells out to a macOS media tool should bound the child; an unbounded one
  turns a flaky tool into a hung run that the harness reports only as "running for over 60
  seconds".
- The `fixture_speaks` ignored test exercises the generator alone, so a hang is diagnosable
  apart from the model.

## Related

- [[2026-09-06-reference-say-hangs-under-cargo-test-unless-stdin-is-closed]] — supersedes
