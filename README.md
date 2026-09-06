# Darric

A macOS desktop app that records audio and transcribes it locally with Whisper.

Built with Tauri 2, Rust, React, and TypeScript.

## Features

- Audio capture from every input and output device at once, with per-device transcript
  attribution
- Local transcription via Whisper (runs on-device with Metal)
- Recordings persist to SQLite — all data stays on your machine

### How transcription works

Audio from each device is resampled to 16 kHz through a band-limited filter (the previous
linear resampler folded everything above 8 kHz down into the speech band) and cut into segments
at pauses
in speech: a segment ends after 400 ms of quiet once it holds at least two seconds, and at
25 seconds regardless, so Whisper gets whole utterances rather than fixed eight-second slices.
Each segment is transcribed on its own. Before a segment reaches Whisper it goes through a voice
activity detector
([Silero VAD](https://github.com/snakers4/silero-vad), MIT, in
[ggml form](https://huggingface.co/ggml-org/whisper-vad)); only the parts it classifies as
speech are decoded, and a segment with no speech at all — an output device with nothing
playing, the quiet tail when you press Stop — produces no transcript line. Without this gate
Whisper invents words for silence: eight seconds of nothing reliably comes back as "Thank you."
The detector's model (885 KB) is bundled in the app and written to
`~/Library/Application Support/darric/` on first use, so it never needs a download.

Whisper decodes with beam search, and everything it says about one segment lands on one
transcript line, so a line is roughly one utterance. A line's `recorded_at` is the time its
audio was captured, not the time Whisper finished with it, so sorting by it puts two devices'
lines back into the order they were spoken.

A microphone that stops delivering audio mid-recording — unplugged, or a Continuity iPhone
microphone that went out of range — is rebuilt with backoff for up to a minute and then given
up on: its row reads `failed` for the rest of the recording, and the other devices carry on. An
output device is tapped once when the recording starts; if that fails its row reads `failed` too.
Either way, once the device is back it is captured again from the next recording, with no
toggling needed.

## Query darric from Claude

While darric is running it serves a read-only [MCP](https://modelcontextprotocol.io) server on
`http://127.0.0.1:27842/mcp`, so Claude Code (or any MCP client that speaks streamable HTTP) can
read your recordings — including one still in progress. Connect Claude Code with one line, or
click the `MCP` chip in darric's header to copy it:

```sh
claude mcp add --transport http darric http://127.0.0.1:27842/mcp
```

Four tools: `status` reports whether a recording is running and which devices are capturing;
`list_sessions` lists recordings newest first; `get_transcript` reads one as device-attributed
lines, paged by a cursor so a second call during a live meeting returns only what has been said
since; `search` finds where something was said across every recording.

The server binds loopback only and reads through its own read-only database connection, so
nothing an agent does can write to your data or hold up the recorder's writes. Loopback means
any account on this Mac can reach it, not just yours. If the chip reads `port busy`, another
process holds port 27842; quit it and relaunch darric. If it reads `off`, hover it for the
reason.

Search is case-insensitive for ASCII letters only. Transcript pages come back in the order
lines were transcribed, which across two devices can differ from the order they were spoken;
each line's `recorded_at` is its capture time, so sort by it to interleave devices as spoken.

## Downloads

Every merge to `main` that can change the binary publishes a prerelease on the
[Releases page](../../releases), tagged `main-<short-sha>`, with a build for each architecture:

| Download                       | For           |
| ------------------------------ | ------------- |
| `Darric_<version>_aarch64.dmg` | Apple Silicon |
| `Darric_<version>_x64.dmg`     | Intel         |

Both require **macOS 14.4 or later**.

These builds are **unsigned** — there is no Apple Developer certificate behind them. macOS
quarantines an unsigned app downloaded through a browser and reports **"Darric is damaged and
can't be opened"**, which is the quarantine flag rather than a corrupt download. Right-click →
Open does not clear it; removing the attribute does:

```sh
xattr -d com.apple.quarantine /Applications/Darric.app
```

To build one yourself instead, see [Prerequisites](#prerequisites) and run `npm run tauri:build`
— the bundles land in `src-tauri/target/release/bundle/`.

## Prerequisites

- macOS 14.4 or later — darric records with Core Audio process taps (`AudioHardwareCreateProcessTap`), which do not exist before 14.4
- [Node.js](https://nodejs.org/) 24 and npm — pinned in `.nvmrc`, which CI reads too. `nvm use`
  picks it up, and fnm reads it automatically; asdf needs `legacy_version_file = yes` in
  `~/.asdfrc`, and Volta ignores `.nvmrc` entirely (`volta pin node@24` instead)
- [Rust](https://rustup.rs/) (stable toolchain)
- Xcode Command Line Tools: `xcode-select --install`

## Getting Started

**1. Install dependencies**

```sh
npm install
```

**2. Run in development mode**

```sh
npm run tauri:dev
```

This starts the Vite dev server and the Tauri app together. On first launch, Darric will
automatically download the Whisper model (~1.6 GB) to `~/Library/Application Support/darric/`.
This only happens once.

The model comes from this repository's
[`models` release](https://github.com/honerlaw/darric/releases/tag/models) rather than from
Hugging Face, because some corporate networks block `huggingface.co` while anything that can
install the app already reaches GitHub's release assets. The file is a byte-exact mirror of
[`ggerganov/whisper.cpp`](https://huggingface.co/ggerganov/whisper.cpp)'s
`ggml-large-v3-turbo.bin` (MIT), and the app checks the download's SHA-256 against
`1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` before using it, so a
truncated or swapped download is refused rather than cached; the next Record press or launch
downloads it again. If neither host is reachable, copy the file from another machine into
`~/Library/Application Support/darric/ggml-large-v3-turbo.bin` and the app will use it as is.

The download starts on its own as soon as the app opens, and a progress bar under the header
reports it. Recording is unavailable until it finishes — the Record button reads
`Downloading <n>%` while it runs — because there is nothing to transcribe with yet. If the
download fails, the reason appears in the error bar at the bottom of the window and the Record
button becomes available again so you can retry.

## Other Commands

```sh
npm run check           # Typecheck, lint, and format all code (TS + Rust)
npm run lint:fix        # Auto-fix JS/TS lint issues
npm run format:fix      # Auto-format JS/TS with Prettier
npm run format:rust:fix # Auto-format Rust with rustfmt
npm run tauri:build     # Build the macOS app bundle into src-tauri/target/release/bundle/
```

Note that `npm run check` does not run the Rust test suite; run `cargo test --manifest-path
src-tauri/Cargo.toml` for that.

## Project Structure

```
src/                  # React frontend
  components/         # Header, recordings list, recorder pane
  hooks/              # Session and transcript state
src-tauri/            # Rust backend
  src/
    audio/            # Audio capture
    transcription/    # Whisper integration
    commands/         # Tauri commands exposed to the frontend
    db/               # SQLite setup and migrations
```

## IDE Setup

[VS Code](https://code.visualstudio.com/) with the [Tauri extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) and [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
