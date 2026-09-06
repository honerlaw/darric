# Scratchpad: list-inline-rename

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Quick decisions 2026-09-06
- [decided] pre-flight: no in-flight unit, branch, PR, or open issue overlaps a list rename; both live darric peers replied MINERVA-IDLE
- [decided] scope check: single unphased unit — one component gains state and a prop, one App handler generalises; no backend
- [decided] approach: RecordingList owns an id-keyed inline editor and App supplies the write via the existing `useSession.update` (dominant — a shared editor extraction rewrites working pane code; routing the double-click into the pane heading puts the input away from the click)
- [decided] whole-proposal soundness: no public interface change — `update_session` and `useSession.update` already exist; blur is the single close path so commit cannot double-fire
