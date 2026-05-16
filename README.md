# Darric

A macOS desktop app for meeting capture and personal work management. Records audio, transcribes speech locally using Whisper, and lets you chat with Claude or Gemini about your notes and sessions.

Built with Tauri 2, Rust, React, and TypeScript.

## Features

- Audio capture from microphone and system audio
- Local transcription via Whisper (runs on-device with Metal)
- AI chat (Claude or Gemini) with MCP server support
- Meeting sessions with notes, tasks, and timeline views
- SQLite-backed persistence — all data stays on your machine

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

This starts the Vite dev server and the Tauri app together. On first launch, Darric will automatically download the Whisper model (~466 MB) to `~/Library/Application Support/darric/`. This only happens once.

**3. Configure an AI provider**

Open Settings in the app and enter an API key for Claude (Anthropic) or Gemini (Google). The key is stored locally in the app's SQLite database.

## Other Commands

```sh
npm run check          # Typecheck, lint, and format all code (TS + Rust)
npm run lint:fix       # Auto-fix JS/TS lint issues
npm run format:fix     # Auto-format JS/TS with Prettier
npm run format:rust:fix # Auto-format Rust with rustfmt
```

## Project Structure

```
src/                  # React frontend
  screens/            # Top-level views (Meeting, Notes, Board, Timeline)
  components/         # Shared UI components
src-tauri/            # Rust backend
  src/
    audio/            # Microphone and system audio capture
    transcription/    # Whisper integration
    ai/               # Claude and Gemini providers, MCP client
    commands/         # Tauri commands exposed to the frontend
    db/               # SQLite setup and migrations
```

## IDE Setup

[VS Code](https://code.visualstudio.com/) with the [Tauri extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) and [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
