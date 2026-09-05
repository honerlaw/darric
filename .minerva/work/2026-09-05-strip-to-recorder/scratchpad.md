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
