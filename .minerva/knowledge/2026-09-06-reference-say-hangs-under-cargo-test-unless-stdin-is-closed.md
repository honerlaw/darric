# macOS `say` spawned from a cargo test never returns unless stdin is closed

**Date**: 2026-09-06
**Type**: reference
**Summary**: `Command::new("say")` with inherited stdin hangs forever under the test harness; `.stdin(Stdio::null())` makes it return in a second
**Context**: .minerva/work/2026-09-06-transcript-accuracy (see git history if the worktree has been cleaned up)

## Context

The transcription accuracy test needs real speech and the repo commits no audio, so the fixture
is synthesized at test time with `say -o speech.aiff "<sentence>"` and converted with
`afconvert -f WAVE -d LEF32@16000 -c 1`.

## Finding

From an interactive shell `say -o file "text"` returns in about a second with any voice. Spawned
from the test binary with `std::process::Command` and inherited stdin, it never returned: the
test sat at "has been running for over 60 seconds" with a `say` process idle at 0.5 s of CPU.
Setting `.stdin(std::process::Stdio::null())` on the command fixed it; the fixture then builds in
2.5 s. `afconvert` gets the same treatment for symmetry.

## Implications

- Close stdin on any `say` invocation from Rust; there is no error, only a hang.
- The `fixture_speaks` ignored test exercises the generator alone, so a future hang is diagnosable
  apart from the model.
- Both tools ship with macOS, the only platform darric builds for; the tests that use them stay
  `#[ignore]`d because they also need the downloaded whisper model.

## Related

- [[2026-09-06-bug-whisper-transcribes-silence-as-thank-you]] — the accuracy test this fixture serves
