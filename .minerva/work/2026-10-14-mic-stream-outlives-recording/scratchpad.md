# Scratchpad: mic-stream-outlives-recording

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Decisions 2026-10-14
- [solo] pre-flight in-flight check: no collision; strip-to-recorder and transcriber-single-flight units read in_flight but are 2026-09-05 and unrelated (stale/adjacent); no open issue matches; #17 adjacent (tier: hardcoded check — no trigger fired)
- [reviewed — clean] scope check: one unit, one PR, no phases; speaker-tap dropout excluded as a follow-up (tier: reviewer — not provably small, no panel clause; parallel wave). Skeptic noted hard-gate the test, capture-correctness and dependency fallout — carried to whole-proposal, not load-bearing for scope
- [reviewed — clean] approach: A, upgrade cpal to 0.18.2; rejected B (pause guard on 0.15.3 — still leaks AudioUnit, keeps upstream bug) and C (0.16.0 — old, migration still pending) (tier: reviewer — interface-clause doubt on persisted device-name keys passed to the Skeptic, who confirmed at source level that 0.18.2 description().name() reads the same kAudioDevicePropertyDeviceNameCFString/Output/Master as 0.15.3 name(); parallel wave)
- [reviewed — folded] whole-proposal: success criteria lacked a permission fallback for the hardware test and named no command for the device-name and log criteria; folded timeout+panic, stop-and-ask, counts recorded, named enumeration test, four named log lines, single check command (tier: reviewer; parallel wave; no restart — scope/approach unchanged)
- [rechecked — residual folded] whole-proposal: items 2 and 5 partially addressed — folded default_input_device()/name→description edit between runs, shared Arc<AtomicUsize> counter, lint:rust --all-targets coverage

## Work notes
