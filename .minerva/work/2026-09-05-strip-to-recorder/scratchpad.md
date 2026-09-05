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
- **Five pre-existing `#[allow]` violations of the `AGENTS.md` rule**, in `model.rs`,
  `audio/mod.rs` (`dead_code`, `non_send_fields_in_send_ty`, three cast lints),
  `audio/microphone.rs` and `transcription/mod.rs`. Success criterion 2 fixes them rather than
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
- The five pre-existing `#[allow]` violations are deliberately NOT fixed in this phase. All of
  them sit in `model.rs`, `audio/mod.rs`, `audio/microphone.rs` and `transcription/mod.rs` —
  files Phase 2 rewrites for the multi-device engine and the model swap. Fixing them now means
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
