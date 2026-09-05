import type React from "react";
import { formatElapsed } from "../../lib/utils";

function recordLabel(
  isRecording: boolean,
  isStarting: boolean,
  downloadProgress: number | null,
): string {
  if (isRecording) return "Stop";
  // Checked before `isStarting`: pressing Record mid-download sets `isStarting`
  // too, and "Starting…" is the label that made the download look like a freeze.
  if (downloadProgress !== null) return `Downloading ${String(downloadProgress)}%`;
  if (isStarting) return "Starting…";
  return "Record";
}

interface HeaderProps {
  isRecording: boolean;
  isStarting: boolean;
  /** Percentage of the speech model downloaded, or null when no download is in flight. */
  downloadProgress: number | null;
  elapsedSeconds: number;
  onRecord: () => void;
  onStop: () => void;
}

export function Header({
  isRecording,
  isStarting,
  downloadProgress,
  elapsedSeconds,
  onRecord,
  onStop,
}: HeaderProps): React.JSX.Element {
  // Gates starting a recording, never stopping one: this is one button serving
  // both roles, and disabling it while `isRecording` would strand the user
  // unable to stop an active recording until an unrelated download finished.
  const cannotStart = !isRecording && (isStarting || downloadProgress !== null);

  return (
    <header className="flex h-14 shrink-0 items-center gap-4 px-6" data-tauri-drag-region="true">
      <div className="flex items-center gap-2">
        <div className="h-3 w-3 rounded-[2px] bg-ink" />
        <span className="font-sans text-[15px] font-[500] tracking-[-0.02em] text-ink">Darric</span>
      </div>

      <div className="flex-1" />

      {isRecording && (
        <div className="flex items-center gap-2">
          <span className="pulse-dot h-2 w-2 rounded-full bg-danger" />
          <span className="font-mono text-[11px] tracking-eyebrow text-ink-3 uppercase">
            recording · {formatElapsed(elapsedSeconds)}
          </span>
        </div>
      )}

      <button
        type="button"
        onClick={isRecording ? onStop : onRecord}
        disabled={cannotStart}
        className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken disabled:cursor-default disabled:opacity-40"
      >
        <span
          className={`h-[7px] w-[7px] rounded-full bg-accent ${isRecording ? "pulse-dot" : ""}`}
        />
        <span>{recordLabel(isRecording, isStarting, downloadProgress)}</span>
      </button>
    </header>
  );
}
