# Leaking the IOProc block hid a use-after-free that fixing the leak would have armed

**Date**: 2026-09-05
**Type**: bug
**Summary**: `mem::forget` on a block Core Audio had already `Block_copy`d leaked its whole captured environment — and because the refcount could then never reach zero, it also masked a teardown race that correcting the leak alone would have re-exposed
**Context**: .minerva/work/2026-09-05-strip-to-recorder

## Context

Each output tap installs an IOProc via `AudioDeviceCreateIOProcIDWithBlock`. The first
implementation ended with:

```rust
// Deliberately leaked into Core Audio's ownership: the block must outlive
// this function and is released when the IOProc is destroyed.
std::mem::forget(block);
```

The comment asserts Core Audio adopts the caller's reference. The binding's own documentation,
carried verbatim from Apple's header, says otherwise:

> Note that this block will be `Block_copy`'d and the reference maintained until a matching call
> to `AudioDeviceDestroyIOProcID` is made.

`Block_copy` takes a **new** reference. The caller's `RcBlock` +1 was still ours to release.

## Finding

Two defects, and the relationship between them is the point.

**The leak.** Every successful tap left the block at a refcount that never returned to zero, so
its captured environment was never freed: an `Arc<Mutex<Segmenter>>` holding up to eight seconds
of buffered audio, plus pool and status handles, per recording session, accumulating across every
start/stop cycle.

**The race the leak was hiding.** `Drop` calls `AudioDeviceStop` and then immediately
`AudioDeviceDestroyIOProcID`, which relies on `Stop` having quiesced the IO thread — universally
assumed, but not promised anywhere in the binding's documentation. If an invocation were still in
flight, `AudioDeviceDestroyIOProcID`'s `Block_release` would free the captured environment while
that invocation was still reading it.

That was **unreachable while the leak existed**, because the refcount could never fall to zero and
`Block_release` therefore freed nothing. Deleting the `mem::forget` in isolation would have
restored correct refcounting and, in the same stroke, armed the use-after-free.

The fix is both at once: let the block drop normally, _and_ clear an
`Arc<AtomicBool>` before `AudioDeviceStop` so an in-flight callback returns without touching
anything.

## Implications

- When an FFI function documents that it copies or retains what you pass, the caller still owns
  its original reference. Read the ownership sentence rather than inferring it from the fact that
  the callee "needs" the object to live on.
- A resource leak can be load-bearing. Before removing one, ask what its presence was preventing —
  a refcount that never reaches zero suppresses every free-related bug behind it.
- `mem::forget` in FFI deserves the same scrutiny as `unsafe`: it is an ownership assertion with
  no compiler check and, here, no runtime symptom short of unbounded memory growth.
- Neither defect was reachable by any test in this repo. Both were found by reading the binding's
  documentation against the code.

## Related

- [[2026-09-05-reference-a-core-audio-tap-starts-not-creates-under-permission]] — the other phase-3 finding, and why neither could be caught by `cargo test`
