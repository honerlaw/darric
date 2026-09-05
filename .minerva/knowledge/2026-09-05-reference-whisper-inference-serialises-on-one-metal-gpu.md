# Whisper inference serialises on a single Metal GPU — the worker pool is not for throughput

**Date**: 2026-09-05
**Type**: reference
**Summary**: measured 1.14x speedup from 4x the threads against one shared `WhisperContext`, so pool size buys almost nothing and the queue's overflow policy is what protects a recording
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Capturing every input device at once means N concurrent streams feeding transcription, which
raised a design question: does a worker pool over one shared `WhisperContext` actually parallelise
on a single Metal GPU, or merely serialise?

`whisper-rs` creates per-call state from a shared context, so multiple threads _can_ call
`state.full()` at once and the model weights load only once. Whether that helps is a different
question, and it was left open in the proposal rather than assumed.

## Finding

`transcription::bench::pool_sizing_measurement` runs four 8-second segments serially, then the
same four concurrently, against one shared context. On Apple Silicon with Metal, `small.en`:

|                     | time      |
| ------------------- | --------- |
| serial, 4 segments  | 1.746 s   |
| parallel, 4 threads | 1.537 s   |
| **speedup**         | **1.14x** |

Four times the threads buys fourteen percent. Inference is effectively serialised on the GPU; the
small gain is CPU-side pre- and post-processing overlapping with someone else's inference.

Measured with `small.en` because that is what was downloaded. The serialisation is a property of
the GPU queue and should hold for `large-v3-turbo`, but the absolute timings will not — turbo is a
substantially larger model.

## Implications

- **Pool size is not a throughput lever.** Two workers recover essentially all of the available
  gain; more only multiplies per-worker state memory. Do not "scale" it with device count.
- **The overflow policy is the load-bearing part.** Since transcription cannot be made faster by
  adding threads, a machine capturing several devices at once will fall behind real time during
  sustained speech. What keeps the recording sane is the bounded queue dropping its **oldest**
  segment, counting the drops, and surfacing that count — not the pool.
- Anyone tempted to raise `WHISPER_WORKERS` should re-run the benchmark rather than reason from
  core count; the test is `#[ignore]`d and takes about fifteen seconds.
- The same reasoning applies to any GPU-bound inference in this app: measure before parallelising.

## Related

- [[2026-09-05-decision-strip-darric-to-a-recorder]] — the unit that made multi-device capture the app's job
