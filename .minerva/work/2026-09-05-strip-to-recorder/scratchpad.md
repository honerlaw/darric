# Scratchpad — 2026-09-05-strip-to-recorder

## Balanced decisions 2026-09-05

- [reviewed — folded] scope check: one unit, three phases (Strip / Multi-input / Multi-output). Skeptic verdict `revise`; folded 7 of 9 concerns — (1) the "Phase 3 isolates every unsafe line" rationale was false, `audio/mod.rs:19` and `transcription/mod.rs:18-19` already carry `unsafe impl Send`/`Sync` and are rewritten in Phase 2, so the claim is narrowed to FFI-unsafe; (2) a delete-list was written but no rewrite-list, leaving `state.rs`, `commands/settings.rs`, `db/mod.rs`, the `generate_handler!` list, `App.tsx`, `MeetingScreen.tsx` and `Header.tsx` unaccounted for — Phase 1 would not have compiled; (3) the CHECK-constraint change is a full SQLite table rebuild, not an `ALTER`; (4) Phase 3's aggregate device re-enters Phase 2's `cpal` enumeration as a phantom input — own-device exclusion registry added; (5) Phase 1's device-row done-bar was ambiguous, fixed to one hard-coded placeholder row; (6) superseding the four MCP knowledge entries made explicit; (7) double-capture recorded as an Open Question. Dismissed as not load-bearing: a request that Phase 3 get differentiated review rigor (`minerva:review` already re-runs per phase).
- [escalated to user] data loss: the Phase 1 migration drops seven tables from the live DB and `rusqlite_migration` is forward-only. Offered JSON export / straight drop / leave tables / whole-file backup. User chose **straight drop — "I don't need that data"**. No export step is written.
- [reviewed — folded] approach: Candidate A, per-source `CaptureSource` abstraction (`cpal` for inputs, one device-scoped Core Audio tap per output). Rejected B (single aggregate device — puts all capture behind FFI, breaking the no-FFI Phase 2) and C (bundled helper process — a second binary to sign, notarize and bundle, one process per output). Skeptic verdict `revise`; folded 7 of 9 — panic safety via `catch_unwind` at every `extern "C"`/IOProc boundary (a panic there aborts the process and would silently falsify the per-device isolation this approach is picked for); C's process-level fault isolation recorded as a real advantage traded away rather than omitted; the overflow policy specified as never-block-the-producer + drop-oldest + surfaced drop counter; the pool-sizing/Metal-serialization question marked unverified with a Phase 2 measurement; B's hot-unplug claim downgraded to an assumption; a bindgen `-x objective-c` / existing-crate survey required before hand-transcribing signatures; per-source retry-with-backoff on device failure; the `source: "mic" | "speaker"` frontend event contract added to Phase 2.
- [decided] approach concern #1 (Skeptic, high severity — "taps are per-process/system-wide, no per-output-device scoping, spike required") **refuted by direct evidence**, not folded. `CATapDescription.h` in this machine's SDK provides `initExcludingProcesses:andDeviceUID:withStream:` and a `deviceUID` property documented as "will have a value if this tap only taps a specific hardware device". The Skeptic's corroborating evidence was also unsound: it read the existing `com.apple.security.screen-capture` entitlement as proof taps are system-wide, but that entitlement belongs to the never-implemented ScreenCaptureKit path. Verification recorded in the proposal so it is not re-litigated.
- [decided] whole-proposal soundness: no public interface or cross-cutting contract beyond the FFI, which is now grounded in verified headers with panic safety and a survey-first rule; blast radius bounded to a personal pre-1.0 app; the four MCP knowledge entries are supersession, not conflict. Solo gate, no escalation.

## Findings pending promote

- **`AGENTS.md` overstates Clippy enforcement.** It states `all = deny`, `pedantic`, `nursery`,
  `cargo` as "the ceiling". `src-tauri/Cargo.toml:75-81` sets only `all` and `correctness` to
  `deny`; `pedantic`, `nursery` and `cargo` are `warn`. `package.json:18` runs
  `cargo clippy --all-targets` with no `-D warnings`, and `.github/workflows/check.yml` adds no
  `RUSTFLAGS`. So a pedantic or nursery violation warns and CI stays green. Sibling of
  `2026-09-05-reference-knowledge-corpus-not-ci-gated`. Candidate `reference` entry.
- **Five pre-existing `#[allow]` sites violating the `AGENTS.md` rule**, across five files:
  `model.rs`, `audio/mod.rs` (`dead_code`, `non_send_fields_in_send_ty`, three cast lints),
  `audio/microphone.rs`, `transcription/mod.rs` and `transcription/speaker_tracker.rs`. Success criterion 2 fixes them rather than
  carrying them forward. Related to the finding above — unenforced rules drift.

## Phase 1 implementation notes 2026-09-05

- Deleted more orphaned code than the proposal inventoried: `src-tauri/src/claude/` (`mod.rs`
  and `sse.rs`, both the single line `// removed`, never declared in `lib.rs`),
  `src-tauri/src/audio/system_tap.rs` (`// removed — mic-only capture`, never declared in
  `audio/mod.rs`), `src/App.css` (empty) and `src/assets/react.svg` (unreferenced). Same class
  as the dead code the proposal did name; no decision needed.
- `sessions.notes` dropped along with the notes feature. The one-screen design has no notes
  pane, and a column nothing reads is the cruft this unit exists to remove. SQLite has supported
  `ALTER TABLE ... DROP COLUMN` since 3.35 and rusqlite's bundled build is far newer, so this is
  a plain `ALTER`, unlike the Phase 2 CHECK-constraint change which still needs a table rebuild.
- `update_session_notes` removed from the command surface with it; `lib.rs` now registers 9
  commands, down from 30.
- `tests/common/mod.rs` needed migration 009 added by hand. This is the duplication
  `2026-05-19-decision-inline-tests-for-mcp-queries` warned about — the integration-test helper
  restates the migration list from `db/migrations.rs`, and both must be edited together. The
  entry's cost prediction landed exactly as written, even though the MCP code that motivated it
  is gone.
- Two nested ternaries (`Header.tsx`, `RecorderPane.tsx`) were extracted into named helper
  functions rather than suppressed, per the `AGENTS.md` linting policy.
- The five pre-existing `#[allow]` sites are deliberately NOT fixed in this phase. All of them
  sit in `model.rs`, `audio/mod.rs`, `audio/microphone.rs`, `transcription/mod.rs` and
  `transcription/speaker_tracker.rs` — files Phase 2 rewrites for the multi-device engine and
  the model swap, or (speaker_tracker) deletes outright. Fixing them now means
  doing the numeric-conversion work twice. Success criterion 2 is a unit-level bar, not a
  phase-1 bar.

## Findings pending promote (continued)

- **`.prettierignore` omits `src-tauri/gen/`, so `npm run format` fails on any machine that has
  built the app.** `src-tauri/gen/schemas` is Tauri-generated and git-ignored
  (`src-tauri/.gitignore:7`), but prettier still walks it and reports all four schema JSON files
  as unformatted. CI never sees this because the frontend job formats before anything generates
  those files — so the failure reproduces only locally, for anyone who has run `tauri dev` or
  `tauri build`. Fixed in this phase by adding `src-tauri/gen/` alongside the existing `dist/`
  and `src-tauri/target/` entries. Candidate `bug` entry: a check that passes in CI and fails on
  every developer machine is worse than one that fails in both.

## Balanced decisions 2026-09-05 (continued)

- [reviewed — clean] completion verification, phase 1: Verifier verdict `accept`. It independently re-ran `npm run check`, `npm run build`, `cargo build` and `cargo test` in the worktree and reproduced every claimed number; confirmed no lint config was weakened and no new `#[allow]`/`eslint-disable`/`@ts-ignore` was introduced; judged the criterion-2 deferral legitimate because all five sites sit in files Phase 2 rewrites or deletes; and accepted the honestly-flagged not-launched-end-to-end gap as mitigated by verified static wiring (9 registered commands matching 9 frontend invocations 1:1, capture path zero-diff from `main`). One documentation imprecision folded: the `#[allow]` write-up named four files when there are five — `transcription/speaker_tracker.rs` was omitted.

## Review finding 2026-09-05

Phase 1 diff reviewed on two lenses: a minerva audit (spec fidelity + knowledge compliance) and
a fresh-context code review. The code review found no high-severity defects in the rewritten
Rust — it hand-checked the riskiest edit, the scripted column-index shift in `sessions.rs` where
`row.get(6)`/`row.get(7)` had to become `(5)`/`(6)` after `s.notes` and the tags subquery left
the SELECT, and confirmed migration 009's drop order is foreign-key safe. Two frontend findings,
both triaged FIX and both fixed in this phase:

1. **`RecorderPane` leaked an in-progress title edit across recordings.** `editingTitle` and
   `titleDraft` were local state with nothing resetting them when `session` changed, and the
   component is never remounted because `RecordingList` selection only swaps the prop. Editing
   recording A's title, clicking recording B, then committing renamed **B** with the text typed
   for A — silently, with no error. This was a regression introduced by this phase: the deleted
   `MeetingScreen` reset its own per-session state on `sessionId` change, and that reset was not
   carried into the rewrite. Fixed with a `useEffect` keyed on `session?.id`, plus two tests in
   `RecorderPane.test.tsx`.
2. **"Resume recording" was offered while a different recording was already running.** The pane
   only knew whether _the viewed_ recording was live, not whether _any_ was. Clicking it hit
   `AppError::SessionActive`, which `useSession` stored in `error` — a field `App.tsx` never
   rendered, so the failure was invisible. Fixed on both halves: a `canResume` prop gates the
   button on global recording state, and `App.tsx` now surfaces `error`. The deleted
   `MeetingScreen` had the same `canResume={!isRecording}` gate; like finding 1, this was
   dropped in the rewrite rather than being a pre-existing defect.

Both regressions share a cause worth remembering: the one-screen rewrite reimplemented
`MeetingScreen`'s behavior from its rendered shape rather than from its state management, so
per-session state resets and cross-session guards were the parts that silently did not survive.

Not reproduced empirically: that `RecorderPane.test.tsx`'s first test fails without the reset
effect. The argument is that `editingTitle` would stay `true` across the rerender and leave a
textbox present, but the mutation experiment was not run.

### Minerva audit

- Spec fidelity: clean. All Phase 1 items delivered, including the seven-file rewrite list.
  Five unlisted changes landed (`sessions.notes` drop, `update_session_notes` removal, README,
  `.prettierignore`, four extra dead files); all follow from stated intent, none change the
  approach, none are a replan trigger.
- `2026-09-05-reference-knowledge-corpus-not-ci-gated` **is stale**. It asserts nothing runs
  `knowledge_lint.py` so wiki drift merges green. Commit `921fc91` added a `Knowledge Wiki` job
  to `check.yml` that runs exactly that script, unconditionally, on every pull request and every
  push to `main`. Not caused by this diff, but a false standing claim in a six-entry corpus.
  Supersede at promote.
- The other three MCP decision entries describe code this phase deletes; supersede at promote.
  `2026-05-19-decision-inline-tests-for-mcp-queries` is the exception — its predicted cost came
  due here exactly as written, so it stays live.

## Balanced decisions 2026-09-05 (promote, phase 1)

- [decided] promote partition, Mode B (per `phasing.md`, the full Mode A pass belongs before the FINAL phase's ship, not phase 1's). PROMOTE ×5: `2026-09-05-decision-strip-darric-to-a-recorder`, `2026-09-05-reference-knowledge-wiki-is-ci-gated`, `2026-09-05-reference-clippy-ceiling-configured-not-enforced`, `2026-09-05-bug-prettierignore-misses-generated-tauri-schemas`, `2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup`. Four supersession banners written: the three MCP decisions retired by the strip, and the stale CI-gating reference replaced by the entry stating current truth. DISCARD: the routine gate log. No TODOs — nothing deferred that lacks a trigger, because phases 2 and 3 are the trigger and they are declared in the proposal.
- [decided] promoted knowledge in phase 1's PR rather than holding it to the end. A unit abandoned after phase 1 would otherwise strand all five entries, and three of them describe facts about the repo that are true regardless of whether phases 2-3 ever land.
- `knowledge_lint.py`: 0 errors, 9 warnings, all `pending reconciliation` — the expected add-only shape. The supersession banners satisfied the reciprocal check for the four superseded entries, so only the five genuinely new forward links are unreciprocated.

- [decided] proposal `## Phases` was written as `### N. Name` subsections, which `read_phases` parses as ZERO phases because its scan breaks at the next `#` line. Ship would have treated a 3-phase unit as unphased and cleanup would have torn down the worktree after phase 1, stranding phases 2-3 with no error anywhere. Rewritten to the canonical numbered-list form (verified: 3 phases now parse) and promoted as `2026-09-05-constraint-phases-must-use-the-canonical-list-form`. Formatting fix to match a documented contract, not a plan change — not a replan trigger.

## Phase 2 implementation notes 2026-09-05

### The pool-sizing question is answered — and the answer changes the design's emphasis

The proposal left open whether concurrent `state.full()` calls against one shared
`WhisperContext` parallelise on a single Metal GPU. `transcription::bench::pool_sizing_measurement`
now measures it: four 8-second segments, serial then concurrent, on Apple Silicon with `small.en`.

- serial: **1.746 s**
- parallel (4 threads): **1.537 s**
- **speedup: 1.14x from 4x the threads**

Inference is effectively serialised on the GPU; the ~14% is CPU-side pre/post-processing
overlapping. So the worker pool is **not** a throughput device, and `WHISPER_WORKERS = 2` recovers
essentially all of the available gain. What actually protects a recording when transcription falls
behind is the queue's drop-oldest policy — exactly what the proposal predicted would carry the
design if this measurement came back flat. Measured with `small.en` because that is what was
downloaded; the serialisation conclusion is a property of the GPU queue and should hold for
`large-v3-turbo`, but the absolute timings will not.

### Two `unsafe` blocks removed rather than carried forward

`SendableStream` existed only because the old code built a `cpal::Stream` on one thread and moved
it to another, which needed `unsafe impl Send` plus an `#[allow(clippy::non_send_fields_in_send_ty)]`.
Each source's supervisor thread now builds, watches and drops its own stream, so the stream never
crosses a thread boundary and the unsafety is gone — not asserted past. `unsafe impl Send/Sync for
Transcriber` remains; it is whisper-rs's own soundness claim, not ours.

### The lint ceiling is now actually met

Clippy reports **0 warnings** across `--all-targets` with `pedantic` + `nursery` + `cargo` enabled —
`main` had 4. All five inherited `#[allow]` sites are gone: the resampler carries its position as
fixed-point integers instead of casting a float cursor back to an index (which also removes
accumulating drift, so it is a correctness improvement rather than lint appeasement), `rms`
accumulates its sample count as `f32`, and the model-download progress logs in whole megabytes.

`cargo clippy --fix` rewrote `wait - slept` as `wait.checked_sub(slept).unwrap()` — trading a
possible panic for a certain one, and putting an `unwrap` in production code. Replaced with
`saturating_sub`. Worth remembering that `--fix` optimises for silencing the lint, not for the
better program.

### Dead code was deleted rather than allowed

The first cut included an `ExclusionRegistry` for phase 3's own-device filtering, plus
`default_input_name`, `state_label` and `queued`. None had a caller, so all of them tripped
`dead_code` — and the policy forbids `#[allow]`. They were removed. Phase 3 adds the exclusion
filter when it has a first caller; the requirement is recorded in the proposal and in a module-level
comment in `audio/device.rs`, which is where someone building the tap will actually be reading.

### Two tests hung and had to be reclassified

`enumeration_yields_unique_stable_ids` and `an_absent_device_gives_up_without_hanging_the_caller`
both drive real Core Audio enumeration, and under `cargo test`'s parallel execution they hung for
over 60 seconds and held the target lock. Both are now `#[ignore]`d with the reason stated. They
test `cpal` and the machine's hardware, not this crate's logic, and the pure state transitions are
covered without hardware. A hanging test is worse than a failing one: it produces no verdict.

Also of note: `timeout` does not exist on macOS (it is `gtimeout`, from coreutils), so two
verification runs silently produced nothing at all rather than failing loudly.

### Findings pending promote (phase 2)

- **`phasing.md`'s phase-progress snippet is wrong for a squash-merging repo.** It feeds
  `phase_progress()` from `git branch --merged <default>`. PR #7 was squash-merged, so phase 1's
  commits never landed on `main` as themselves and `--merged` cannot see that branch at all —
  `phase_progress` reported `merged: 0` for a phase that had shipped. `merge-detection.md` already
  knows about squash merges and checks `gh pr list --state merged` first; the phasing snippet does
  not. A unit shipping its phases on a squash-merging repo would re-ship phase 1 forever. Candidate
  `bug` entry against the minerva tooling.
- **A freshly cut phase branch with zero commits reads as "merged"**, because it is identical to
  the default branch. Combined with the above, phase resolution is only trustworthy once the phase
  has at least one commit. Worth folding into the same entry.

## Review finding 2026-09-05 (phase 2)

Reviewed on two lenses again. The minerva audit caught one spec-fidelity miss: the proposal said
phase 2 drops `speaker_tracker.rs` **and** `rustfft`, and only the file had gone — an unused Rust
dependency compiles fine, so nothing but reading the spec back against the diff would have found
it. Removed; Rust dependencies are now 13, down from 18 before the strip.

The audit also re-checked whether phase 2's second rewrite of `RecorderPane`/`App` repeated the
mistake `2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup` documents. It did not: the
`[sessionId]` reset, the `canResume` gate and the error banner all survived. The entry earned its
keep on the very next change to those files.

The code review found four defects, all triaged FIX and all fixed.

### 1. HIGH — `Arc::try_unwrap` silently disabled the entire transcription pool

`CaptureEngine::start` wrapped the pool in an `Arc`, handed a clone to each device's capture
thread, and then called `Arc::try_unwrap(pool)` to reclaim ownership for the struct field. With N
devices the strong count is `1 + N`, and `begin_capture` refuses to start with zero devices — so
`try_unwrap` failed **on every real recording**, `.ok().flatten()` turned that into `None`, and the
engine stored no pool at all while the real one stayed alive inside the thread closures.

Three consequences, all silent and all deterministic:

- `stop()`'s flush block is guarded by `if let Some(pool)`, so **every device's trailing partial
  segment was discarded on every stop** — up to 8 seconds of real speech per device per recording.
  The frontend's `FLUSH_LINGER_MS` exists precisely to wait for that chunk, so the two halves of
  the design disagreed with each other and nothing failed loudly.
- `dropped_segments()` always returned 0, so the drop warning could never appear — the honesty
  mechanism built specifically to avoid a silently incomplete transcript was itself silently
  disabled.
- `pool.shutdown()` was never called, so the whisper workers stayed parked in `Condvar::wait`
  forever and their `JoinHandle`s were dropped without joining: **two leaked threads per
  recording session**.

Fixed by changing `TranscriptionPool::shutdown` to take `&self` (joining through a
`Mutex<Vec<JoinHandle>>`, drained so it is idempotent) and having the engine hold
`Option<Arc<TranscriptionPool>>` rather than trying to reclaim sole ownership. The type now makes
the bug unexpressible instead of relying on a runtime unwrap that always fails.

Worth noting that the completion Verifier explicitly checked the _ordering_ inside `stop()` and
pronounced it correct — which it was. The ordering was right and the guard above it was never
true. Verifying a sequence of steps says nothing about whether the block containing them runs.

### 2. MEDIUM — `stop_session` blocked a Tokio worker

An `async fn` command called the synchronous, thread-joining `engine.stop()` directly. Masked
while bug 1 was live (there was nothing to join); the moment 1 was fixed it would block a runtime
worker for as long as the queue took to drain — seconds, with several devices, since inference
serialises on the GPU. Now wrapped in `spawn_blocking`.

### 3. LOW — a spawn failure mid-loop leaked already-started capture threads

If `thread::Builder::spawn` failed for device N, the function returned `Err` before constructing
the engine, so the N−1 threads already running had no handle able to stop them. Now sets the
shutdown flag, joins what started, and shuts the pool down before returning.

### 4. LOW — an optimistic device toggle could desynchronise

`useDevices.toggle` awaited the IPC call without catching, and its only caller discards the
promise with `void`, so a failed toggle became an unhandled rejection and could leave the switch
showing a state the backend rejected. Now caught and logged; the `finally` refresh re-reads truth.

### Findings pending promote (phase 2 review)

- The `Arc::try_unwrap`-after-sharing pattern is worth an entry: it fails silently, it is
  guaranteed rather than racy once anything else holds a clone, and `.ok()` converts the failure
  into a plausible-looking `None`.

## Balanced decisions 2026-09-05 (phase 2)

- [reviewed — clean] completion verification, phase 2: Verifier verdict `accept`. It independently re-ran clippy, the full suite, the ignored hardware tests and the migration-list agreement check, and confirmed the pool-sizing comment matches the benchmark rather than being fabricated. Two non-blocking fix-ups folded: a stale download log still naming `ggml-small.en-tdrz.bin (~466MB)` after the model swap, and the last `#[allow(clippy::unwrap_used)]` sitting in `mod bench` rather than literally `mod tests` — resolved by replacing the unwraps with `expect`, leaving **zero** `#[allow]` anywhere.
- [decided] review triage, phase 2: 4 findings, all FIX, all fixed. One HIGH (`Arc::try_unwrap` disabling the pool) with a writable failure scenario — trailing segment lost on every stop, drop counter pinned at 0, two leaked threads per session. Two LOW with concrete scenarios (spawn-failure thread orphaning, unhandled toggle rejection) fixed rather than deferred because both were cheap and both leave the app in a state a user cannot recover from. One MEDIUM (blocking a Tokio worker in `stop_session`) fixed in the same pass because bug 1 was masking it.
- [decided] promote partition, Mode B (phase 3 outstanding, so no Mode A pass yet). PROMOTE x4: the `Arc::try_unwrap` bug, the verification-shape pattern it exposed, the Metal serialisation measurement, and the `phase_progress` squash-merge bug found while shipping phase 1. DISCARD: routine gate logs.
- [decided] the `phase_progress` finding is promoted even though it is a defect in minerva's own tooling rather than in darric. It was found here, it will bite the next phased unit in any squash-merging repo, and `.minerva/knowledge/` is where this project records what it learned — including about its own process.

## Phase 3 implementation notes 2026-09-05

### The survey the approach required paid for itself

The proposal said to survey `bindgen -x objective-c` and existing crates **before** hand-writing
the two tap signatures, because transcribing a C prototype that bindgen skipped is exactly the
error a reviewer cannot see and a runtime cannot survive. The survey found
`objc2-core-audio` 0.3.2, which already generates `AudioHardwareCreateProcessTap`,
`AudioHardwareDestroyProcessTap`, `CATapDescription`, `AudioHardwareCreateAggregateDevice`,
`AudioDeviceCreateIOProcIDWithBlock` and every constant needed, straight from the SDK headers.

So **no C signature is declared by hand anywhere in this phase**. That removes the single largest
risk the approach Skeptic identified. It also produced a better callback: the block-based
`AudioDeviceCreateIOProcIDWithBlock` carries its captured state directly, instead of the raw
function pointer plus `*mut c_void` userdata the older API forces.

### What actually got built

`audio/coreaudio.rs` — checked property reads (ask for the size, allocate exactly that, ask for
the data) plus output-device enumeration and stream-format reading. `audio/tap.rs` — an
`OutputTap` that owns a process tap, a private aggregate device and an IOProc, and unwinds all
three in reverse on drop, including on partial-construction failure.

Verified live on this machine before building anything on top of it: enumeration returns
`uid="BuiltInSpeakerDevice" name="MacBook Pro Speakers"`. That is the first runtime-verified
piece of phase 3 rather than a compile-time one.

### The exclusion registry earns its place now

Aggregates are created **private**, which hides them from other processes — but not from this
one, and this is the process running `cpal`'s enumeration. So a private aggregate is _not_ on its
own sufficient, and each tap registers its aggregate UID with `ExclusionRegistry`, which input
enumeration filters against. Phase 2 deliberately deleted this type for having no caller; it is
back because it now has two.

### Two real defects clippy found in the unsafe code

- **Alignment UB.** `*bytes.as_ptr().cast::<u32>()` dereferences a `*const u32` derived from a
  `Vec<u8>` buffer, which carries only 1-byte alignment. `cast_ptr_alignment` caught it; fixed
  with `read_unaligned`. This is the kind of thing that works on x86 and faults elsewhere, and it
  would not have been caught by any test on this machine.
- **A lossy `f64` -> `u32` cast** on the sample rate. Rather than suppress
  `cast_possible_truncation`, the conversion is now an exact binary search over the `u32` range
  using only `u32 -> f64` (which is exact), settling in 32 iterations — once per tap.

Both were found by the lint policy this repo enforces, in code where the failure mode is silent.

### A test of mine caught its own ambiguity

`exact_u32_from_f64(f64::INFINITY)` returns 0, not `u32::MAX`: the non-finite guard fires before
the saturating clamp. Rejecting garbage is the safer semantic — 0 is refused by every caller,
whereas saturating would hand the resampler a 4-billion-hertz rate and produce convincing noise —
so the behaviour stands and the doc comment now says so explicitly.

### Permissions

`NSScreenCaptureUsageDescription` and the `com.apple.security.screen-capture` entitlement are
gone, replaced by `NSAudioCaptureUsageDescription`. They belonged to a ScreenCaptureKit path that
was never implemented; taps are gated on audio recording, and that prompt is markedly less
alarming to grant than screen recording.

### The live tap test: how far it got, and the wall it hit

Running `OutputTap::start` against the real built-in speakers from `cargo test`:

```
tapping "MacBook Pro Speakers" (BuiltInSpeakerDevice)
!! TAP DID NOT START: AudioDeviceStart failed: OSStatus 268451843 (0x10004003)
```

Read carefully, this is more informative than a flat failure. `AudioHardwareCreateProcessTap`
**succeeded**, `AudioHardwareCreateAggregateDevice` **succeeded**, and
`AudioDeviceCreateIOProcIDWithBlock` **succeeded** — the construction sequence, the toll-free
bridged `CFDictionary`, the tap-list structure and the UUID plumbing are all correct enough for
Core Audio to accept them. Only the final `AudioDeviceStart` was refused, and the failure path
destroyed the aggregate and the tap cleanly with no leaked system-wide audio objects.

The cause is structural rather than a code defect: a bare `cargo test` binary has **no Info.plist
and no bundle identifier**, so macOS TCC cannot associate it with an audio-recording grant or
prompt for one. The `NSAudioCaptureUsageDescription` this needs lives in the _app bundle's_
Info.plist. A tap can be created without the permission but cannot be started.

**Consequence for verification: phase 3's capture path cannot be proven by `cargo test` at all.**
It requires launching the built app and granting the permission. That is a real limit on what
automated verification can establish here, and it should not be papered over — everything in
phase 3 except device enumeration and teardown remains compile-verified rather than
runtime-verified.

### Findings pending promote (phase 3)

- A Core Audio process tap can be **created** without the audio-recording permission and only
  fails at `AudioDeviceStart`. The natural assumption is that creation would fail, so error
  handling written around creation alone would report success on a tap that can never deliver
  audio. Worth an entry together with the `cargo test` / TCC limitation.
