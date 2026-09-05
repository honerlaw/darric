# A feature mounted below an early return is wired end-to-end and unreachable

**Date**: 2026-09-05
**Type**: pattern
**Summary**: the model-download indicator was complete and correct but rendered below a `session === null` early return, so it could not run in the only state it existed for
**Context**: .minerva/work/2026-09-05-model-download-progress (see git history if the worktree has been cleaned up)

## Context

darric downloads a ~1.6 GB Whisper model on first launch. Users reported the app looked frozen:
the Record button sat disabled reading "Starting…" for minutes with nothing else on screen.

The obvious diagnosis — that nobody had built a progress indicator — was wrong. One existed and
was complete. `model.rs` emitted `model_download_start` / `_progress` / `_done`; `lib/tauri.ts`
bound all three; `useSession` reduced them into a `downloadProgress` state; `App` threaded it
into `RecorderPane`; and `RecorderPane` rendered a labeled percentage and a progress bar from it.
Every link in the chain was present and correct.

## Finding

`RecorderPane` began with:

```tsx
if (session === null) {
  return <p>Select a recording, or press Record to start a new one.</p>;
}
```

The download block was **below** that return. Rendering it therefore required a selected
recording — and the download runs at first launch, when no recordings exist yet. The precondition
for displaying the indicator was the exact negation of the state it was built to describe. It
could never render, in any run, and nothing about the code looked wrong at any single point.

The fix was not to repair the indicator but to move it: an app-scoped `ModelDownloadBanner`
rendered from `App`, above the pane, where the state it displays actually lives.

## Implications

- **A component's mount point is a precondition on everything inside it.** When a piece of UI
  describes app-global state, hosting it inside an entity-scoped component silently ANDs
  "an entity is selected" onto its display condition. Ask what must be true for the enclosing
  component to render _at all_ before asking whether the block inside it is correct.
- **Reviewing a data path end-to-end does not establish that the path terminates in something
  visible.** Every hop here was individually correct; the defect lived only in the composition.
- This class survives tests that render the component with a fixture entity, because supplying
  one satisfies the early return by construction. A test asserting the indicator appears
  **without** an entity is what pins it.

## Related

- [[2026-09-05-pattern-verifying-a-sequence-says-nothing-about-whether-it-runs]] — the same question, "does this code run at all?", asked of a component's mount point rather than a Rust guard
- [[2026-09-05-pattern-ui-rewrites-drop-state-guards-not-markup]] — another defect this rewrite left behind that is invisible in a screenshot
- [[2026-09-05-decision-strip-darric-to-a-recorder]] — the rewrite that produced the misplaced mount
