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

### Baseline on cpal 0.15.3 (only the tests added)

`cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored --nocapture --test-threads=1 a_stopped_source enumeration_yields`:

```
input device: "Derek’s iPhone Microphone"
input device: "MacBook Pro Microphone"
capturing from "MacBook Pro Microphone"
callbacks: 2 before stop, 39 after join, 133 one second later
panicked: the stream kept delivering after its source stopped (left: 39, right: 133)
```

The leak reproduces: callbacks arrived before the stop, and the count kept rising after the supervisor
had dropped the stream and been joined. This session runs under Terminal.app, which already holds
the audio-capture grant from `npm run tauri:dev`, so the test could open the mic.

### After the upgrade to cpal 0.18.2

Same command, three consecutive runs:

```
input device: "MacBook Pro Microphone"
callbacks: 3 before stop, 20 after join, 20 one second later   -> ok
callbacks: 2 before stop, 18 after join, 18 one second later   -> ok
callbacks: 2 before stop, 19 after join, 19 one second later   -> ok
```

- **Device names:** "MacBook Pro Microphone" is byte-identical across 0.15.3 and 0.18.2. The
  iPhone was absent from the 0.18.2 runs because it had dropped off macOS itself: it was gone
  from `system_profiler SPAudioDataType` too. Continuity attaches it only on demand, so per the
  criterion it is compared only where present in both runs. The source-level equivalence covers it.
- **Callback counts:** the "before stop" counts are small (2–3) because the test stops as soon as
  the first callback lands. The counter keeps rising during the join, because the stream is
  dropped only after the supervisor's 200 ms health poll notices shutdown.
- **Device identity:** it now lives in one helper, `audio::device::device_name`, used by
  enumeration, `build_stream`'s lookup, and the leak test. That helper is where the "not `id()`"
  rationale is written down.
- **Gotcha, test logging:** tests initialise no logger. A `RUST_LOG=…` run of an ignored test
  prints nothing, so the whisper-label check used a temporary
  `env_logger::builder().is_test(true)…init()` in
  `silence_yields_nothing_and_speech_yields_the_words`, which was reverted afterwards. The output
  showed `[whisper] test: no speech…`, `test: … too short…`, `test: N of M samples are speech`
  and `test: Some("…")`.
- **Clippy:** `npm run lint:rust` (`--all-targets`) finished with zero warnings.
