# Darric

A macOS desktop app that records audio and transcribes it locally with Whisper.

Built with Tauri 2, Rust, React, and TypeScript.

## Features

- Audio capture from the microphone
- Local transcription via Whisper (runs on-device with Metal)
- Recordings persist to SQLite — all data stays on your machine

Multi-device capture (every input and output device at once, with per-device transcript
attribution) is in progress; see `.minerva/work/2026-09-05-strip-to-recorder/proposal.md`.

## Downloads

Every merge to `main` that can change the binary publishes a prerelease on the
[Releases page](../../releases), tagged `main-<short-sha>`, with a build for each architecture:

| Download                       | For           |
| ------------------------------ | ------------- |
| `Darric_<version>_aarch64.dmg` | Apple Silicon |
| `Darric_<version>_x64.dmg`     | Intel         |

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

- [Node.js](https://nodejs.org/) (v18+) and npm
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
