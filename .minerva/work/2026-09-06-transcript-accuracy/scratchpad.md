# Scratchpad: transcript-accuracy

> **Ephemeral working memory.** Most of what lands here is noise — small
> decisions that don't matter, dead ends, momentary confusion. At feature
> completion, run `minerva:promote`: significant items get promoted to
> `.minerva/knowledge/`, `proposal.md` gets updated to match reality, and
> the raw scratchpad is archived.

## Balanced decisions 2026-09-06
- [reviewed — folded] scope check: one unit, two phases (silence-gate, utterance-segmentation), not decomposed (Skeptic accept; item 1 load-bearing — criterion 8 spanned both phases — split into 8a/8b; items 2–3 reworded in Why; item 4 stale, tree clean; item 6 becomes a ship-report note)
- [rechecked — clean] scope check: fold-audit confirmed all six items addressed; no new concerns
- [reviewed — folded] approach: A — standalone Silero VAD in transcribe + energy cut points + bundled model (Skeptic revise; folded 1 prompt carry-over bounded by 15 s recency / 30 words / cleared on silence, 3 noise floor updates only from non-speech frames, 4 integer timestamp→sample arithmetic; also folded 6 byte-compare + loader single-flight comment, 7/8/9 wording; dismissed 5 as re-weighting (beam retained, wording softened), 10 no action; rejected B duplicates VAD per device on its own thread, C leaves every other hallucination)
- [rechecked — escalated] approach: fold-audit found item 1 (prompt carry-over bound) only partially addressed — a bad line could chain through continuous speech; asked; user chose to drop prompt carry-over from the unit entirely (escalation 1 of 3)
- [reviewed — clean] whole-proposal soundness: Skeptic accept; noted and clarified in text: ignored accuracy test is run locally before each ship (not CI), phase-1 one-line join can merge two utterances in one 8 s window (accepted until phase 2), exact_u32_from_f64 becomes pub(crate), fixture is say+afconvert as in the prototype
