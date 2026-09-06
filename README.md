# Darric

A macOS desktop app that records audio and transcribes it locally with Whisper.

Built with Tauri 2, Rust, React, and TypeScript.

## Features

- Audio capture from the microphone
- Local transcription via Whisper (runs on-device with Metal)
- Recordings persist to SQLite — all data stays on your machine

Multi-device capture (every input and output device at once, with per-device transcript
attribution) is in progress; see `.minerva/work/2026-09-05-strip-to-recorder/proposal.md`.

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

Search is case-insensitive for ASCII letters only, and transcript lines come back in the order
they were transcribed, which across two devices can differ from the order they were spoken.

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
