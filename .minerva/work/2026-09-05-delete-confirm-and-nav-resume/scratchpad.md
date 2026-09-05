# Scratchpad: delete-confirm-and-nav-resume

## Quick decisions 2026-09-05
- [decided] pre-flight: no in-flight collision — all work-unit branches have MERGED PRs, no open PRs, no same-project live peers
- [decided] open-issue match: none — the three open `minerva:followup` issues (#16 #17 #18) are backend/lint, unrelated to the seed
- [decided] scope check: one work unit, one PR — two independent small UI edits, no phases needed
- [decided] approach: local `pendingDeleteId` in RecordingList + extracted `ConfirmDialog` (dominant — `window.confirm` is not the app's design language and is unassertable; lifting state to App is prop drilling for one caller)
- [decided] approach: Header gains `canResume`/`onResume`; RecorderPane's footer is deleted rather than kept in parallel (two Resume controls would be the bug being fixed)
- [decided] soundness: no public interface, no backend, no `.minerva/knowledge/` constraint in tension
