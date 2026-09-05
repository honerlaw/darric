# Strip darric to a multi-source recorder

**Status**: Draft
**Date**: 2026-09-05

## Goal

Reduce darric to one job: start a recording, capture audio from every input **and** output
device on the machine simultaneously, transcribe it locally, stop. Every transcript line is
attributed to the device it came from. Everything not serving that job is removed.

## Why

darric currently spans meeting capture, a notes app, a kanban board, a tagged search index, a
two-provider AI chat agent, an MCP client and an MCP server — roughly 10,300 lines, of which
about 1,000 are already dead (`NotesScreen.tsx` at 650 lines is never imported; five files under
`components/chat/` and `components/notes/` are literally `export {};`).

The recording path is the least developed part of all of it. It captures the default microphone
only. `src-tauri/src/audio/system_tap.rs` has been the single line `// removed — mic-only
capture` since the initial commit, and `AudioSource` has exactly one variant, so the app cannot
hear the other side of any call it is sitting in. Speaker attribution is guessed by 305 lines of
hand-rolled MFCC fingerprinting against a magic 0.82 cosine threshold.

Deleting the surrounding product surface and spending that space on real multi-device capture
makes the one thing darric is for actually work — and replaces a guess with a fact, because a
stream's originating device is known rather than inferred.

## Approach

**Per-source backends behind a uniform `CaptureSource` abstraction.**

Every device — input or output — is an independent capture source producing 16 kHz mono `f32`
chunks. Input sources use `cpal`, as today. Each output device gets its own Core Audio process
tap scoped to that device, plus an aggregate device to read it through.

Two candidates were rejected:

- **One aggregate device for everything** (all input sub-devices plus all output taps in a
  single aggregate, one IOProc, demux by channel offset). Core Audio would resolve cross-device
  clock drift internally, which is a real advantage. Rejected because it puts *all* capture
  behind the FFI path, so Phase 2 could not ship without Phase 3's unsafe code. A secondary
  concern — that one hot-unplug tears down the whole aggregate — is an **assumption, not a
  verified fact**, and is not load-bearing for the rejection.
- **A bundled helper process** that performs the tap and pipes PCM over stdout (the `audiotee`
  shape). Its genuine advantage is process-level fault isolation: a crash in the unsafe tap code
  takes down a disposable child rather than the app. That is a stronger form of blast-radius
  containment than this approach gets in-process. It is judged not worth a second executable to
  build, code-sign, notarize and bundle inside Tauri, plus one process per output device — on
  the condition that in-process containment is done properly, which is why panic safety below is
  a requirement rather than a detail.

### Output taps are scoped per device — verified

`CATapDescription` supports device scoping directly. From `CATapDescription.h` in the macOS SDK
on this machine:

```objc
- (instancetype) initExcludingProcesses:(NSArray<NSNumber*>*)processesObjectIDsToExcludeFromTap
                           andDeviceUID:(NSString*)deviceUID
                             withStream:(NSInteger)stream;
```

documented as "mix all process audio streams destined for the selected device stream except the
given processes", alongside `@property (atomic, copy, nullable) NSString* deviceUID` — "an
optional deviceUID that will have a value if this tap only taps a specific hardware device".

So one tap per output device is the supported design: pass an empty exclude-list plus that
device's UID. This is recorded because it is the assumption Phase 3's whole device-attribution
story rests on, and it is not obvious from the API's name.

### FFI mechanics

`AudioHardwareTapping.h` is `#ifdef __OBJC__`-guarded and is **not** included by `CoreAudio.h`,
so `coreaudio-sys` does not bind `AudioHardwareCreateProcessTap` / `AudioHardwareDestroyProcessTap`
(verified: zero hits in the generated bindings). It *does* bind `AudioHardwareCreateAggregateDevice`
and the `kAudioAggregateDeviceTapListKey` / `kAudioSubTapUIDKey` constants.

Before hand-transcribing those two signatures, **survey the cheaper options first**: pointing
`bindgen` at the header with `-x objective-c`, and any existing crate that already wraps process
taps. Hand-writing a C signature that bindgen skipped is itself an unsafe-correctness risk —
wrong calling convention, wrong struct layout, wrong `AudioObjectID` ownership — and is the
class of bug FFI reviewers most often miss.

### Panic safety is a requirement, not a detail

The IOProc callback runs on a Core Audio realtime thread and is entered through a raw C function
pointer. A Rust panic crossing that boundary aborts the process. Per-source isolation does
**not** contain it: a bad index or an `unwrap` in one device's callback takes down the whole
app, which would silently falsify the "one device failing degrades only that device" property
this approach is chosen for.

Every `extern "C"` callback therefore wraps its body in `catch_unwind`, and the callback path
itself must be allocation-free and lock-free on the realtime thread.

### Transcription under N streams

One shared `Arc<Transcriber>`. `whisper-rs` already calls `ctx.create_state()` per
transcription, so a single `WhisperContext` loads the model weights once and each worker holds
its own state. A bounded worker pool drains a bounded per-source segment queue.

**Overflow policy: never block the producer.** The audio callback is a realtime thread and must
not be back-pressured. When a source's queue is full the oldest segment is dropped, a
per-session drop counter increments, and the UI surfaces that the transcript is incomplete for
that device — a silently truncated transcript is the worse failure.

Whether concurrent `state.full()` calls against one `WhisperContext` actually parallelize on a
single Metal GPU, or merely serialize, is **unverified**. Phase 2 measures it with 6+ concurrent
8-second segments and sizes the pool from the result; if it serializes, the pool is one worker
and the overflow policy above is what carries the design.

### Per-source failure handling

A `CaptureSource` failure — USB mic unplugged, a device that refuses a second concurrent open —
retries with backoff and is marked failed in the UI. It is never fatal to the session. The
current code only logs `[mic] stream error` and does nothing, which is not sufficient once
"every input device" is the target.

### Own-device exclusion

`AudioHardwareCreateAggregateDevice` creates a real `AudioObjectID` visible to ordinary Core
Audio enumeration — which is what `cpal`'s macOS backend walks. Without an exclusion filter the
app would enumerate its own tap aggregates as phantom inputs and feed its output back into its
own capture. The device enumerator holds a registry of the aggregates this process created and
filters them out.

## Success criteria

1. `npm run check` passes (typecheck, both lint passes, both format passes, both test suites).
2. No `#[allow(...)]` outside `#[cfg(test)]` blocks anywhere in `src-tauri/src/`, and no
   `eslint-disable` / `@ts-ignore` anywhere in `src/` — including the five pre-existing
   violations this work inherits, which are fixed rather than carried forward.
3. Starting a recording captures from every enabled input device and every enabled output
   device concurrently; each transcript line records which device produced it.
4. A device failing or being unplugged mid-recording does not stop the recording or lose the
   other devices' audio.
5. Elapsed-time accounting still survives stop/resume (the `recording_segments` behavior).
6. The final Whisper segment still lands after stop (the `FLUSH_LINGER_MS` behavior).
7. Nothing in the shipped tree references the AI, MCP, notes, tasks, tags, search or board
   subsystems.

## Phases

Three ordered phases, each independently shippable, each leaving the repository working.

### 1. Strip

Delete the AI providers and agent harness (`src-tauri/src/ai/`), the MCP client, the MCP server
(`src-tauri/src/mcp_server/`, including the 919-line `queries.rs`), and
`commands/{chat,notes,tasks,tags,search,mcp_server}.rs`. Drop the `rmcp`, `axum`, `tokio-util`
and `schemars` dependencies, and the `db_notes.rs` / `db_tasks.rs` / `db_search.rs` tests.

On the frontend, delete `TimelineScreen`, `NotesScreen`, `BoardScreen`, `SettingsModal`,
`SearchBar`, `NoteModal`, `Dock`, `TagInput`, everything under `components/chat/` and
`components/notes/`, the already-dead `TopBar` / `Sidebar` / `TranscriptPanel` / `useSettings`,
and the `useConversation` / `useNotes` / `useTasks` / `useTags` / `useSearch` hooks. Drop
`@dnd-kit/*`, `marked` and `@tanstack/react-virtual`.

**Rewritten, not deleted** — these reference doomed modules and will not compile untouched:

| File | Why |
|---|---|
| `src-tauri/src/state.rs` | `AppState` holds `ai_harness`, `chat_history`, `mcp`, `mcp_server` |
| `src-tauri/src/commands/settings.rs` | `save_setting` constructs providers on `ai.*` keys; `list_mcp_servers` reads `state.mcp` |
| `src-tauri/src/db/mod.rs` | `load_chat_history` imports `ChatMessage`/`ContentBlock`/`Role` from `crate::ai` |
| `src-tauri/src/lib.rs` | the `generate_handler!` list registers 20 commands that no longer exist |
| `src/App.tsx` | imports and wires nine deleted components and hooks |
| `src/screens/MeetingScreen.tsx` | imports `TagInput`; colors by `speaker_label` |
| `src/components/layout/Header.tsx` | hard-codes nav tabs for screens that no longer exist |

Migration dropping `notes`, `tasks`, `tags`, `session_tags`, `note_tags`, `task_tags` and
`chat_messages`. This permanently destroys the contents of those tables in the live database at
`~/Library/Application Support/darric/darric.db`; the user has explicitly confirmed that data is
disposable, so no export step is written.

UI collapses to the one-screen shape — recordings list plus live pane — driven by the existing
mic-only capture, with a **single hard-coded device row**. Level meters and per-device toggles
are Phase 2; Phase 1's row is a placeholder so the layout is settled before the engine lands.

Promote also marks the four MCP knowledge entries superseded: they document a subsystem this
phase deletes, and left alone they would assert live architecture that no longer exists.

**Done when**: the app builds, records from the default mic, transcribes, saves and lists
recordings, with no reference to any deleted subsystem.

### 2. Multi-input capture and device attribution

Enumerate and capture every input device concurrently through the `CaptureSource` abstraction.
Per-source segmenting, the bounded worker pool and the overflow policy, the pool-sizing
measurement, and per-source failure retry.

Replace `transcript_lines.source CHECK(source IN ('mic','speaker'))` with device-attribution
columns. SQLite cannot drop a CHECK constraint, so this is a full table rebuild — create the new
table, copy rows mapping existing `mic`/`speaker` values onto the new columns, drop the old,
rename, and recreate `idx_transcript_session` and the `sessions` foreign key.

The event contract changes with it: `TranscriptChunk` and `TranscriptLine` in
`src/types/index.ts` and the payload handled in `src/hooks/useTranscript.ts` both hard-code
`source: "mic" | "speaker"` today.

Delete `transcription/speaker_tracker.rs` and the `rustfft` dependency. Swap the Whisper model
from `ggml-small.en-tdrz` to `large-v3-turbo`. Real device rows with level meters and per-device
toggles.

This phase contains no FFI. It does carry forward the pre-existing `unsafe impl Send for
SendableStream` (`audio/mod.rs:19`) and `unsafe impl Send/Sync for Transcriber`
(`transcription/mod.rs:18-19`) — marker impls asserting soundness, a different and much smaller
thing than FFI.

**Done when**: a recording captures every connected input device at once, each line names its
device, and unplugging one mid-recording loses only that device.

### 3. Output capture via Core Audio taps

The FFI: `CATapDescription` through `objc2`, the two hand-declared (or bindgen-generated) tap
functions, the aggregate device, the IOProc, `catch_unwind` at every callback boundary, and the
own-device exclusion registry.

Replace `NSScreenCaptureUsageDescription` and the `com.apple.security.screen-capture`
entitlement with `NSAudioCaptureUsageDescription` — taps do not use ScreenCaptureKit, and the
audio-only permission prompt is markedly less alarming than screen recording.

**Done when**: a recording captures what the machine is playing, attributed to the output device
it played through, alongside the microphones.

## Open questions

1. **Double capture.** Capturing a microphone and an output tap at once transcribes the same
   speech twice when not on headphones — once from the tap, once attenuated from the mic. The
   per-device toggles are the intended mitigation. Whether that is enough in practice, or
   whether the transcript needs actual dedup, is deferred until there is real usage to judge it
   against. Not solved speculatively.
2. **Pool sizing.** Left open until Phase 2's measurement (see Approach).
