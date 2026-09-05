import type React from "react";
import { formatElapsed } from "../../lib/utils";

function recordLabel(
  isRecording: boolean,
  isStarting: boolean,
  isStopping: boolean,
  downloadProgress: number | null,
): string {
  // Checked before `isRecording`: the recording stays live for the whole stop,
  // and leaving the label on "Stop" is what made a multi-second flush read as a
  // click that did nothing.
  if (isStopping) return "Stopping…";
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
  /**
   * A stop is in flight: capture has ended but the backend is still flushing and
   * transcribing. `isRecording` stays true throughout, so "actively capturing"
   * is `isRecording && !isStopping`.
   */
  isStopping: boolean;
  /** Percentage of the speech model downloaded, or null when no download is in flight. */
  downloadProgress: number | null;
  elapsedSeconds: number;
  onRecord: () => void;
  onStop: () => void;
}

export function Header({
  isRecording,
  isStarting,
  isStopping,
  downloadProgress,
  elapsedSeconds,
  onRecord,
  onStop,
}: HeaderProps): React.JSX.Element {
  // Disabled in exactly two cases: a start that cannot proceed, and a stop that
  // is already under way. Never while a recording is live and stoppable — this is
  // one button serving both roles, and disabling it on an unrelated download
  // would strand the user unable to stop an active recording.
  const disabled = isStopping || (!isRecording && (isStarting || downloadProgress !== null));

  return (
    <header className="flex h-14 shrink-0 items-center gap-4 px-6" data-tauri-drag-region="true">
      <div className="flex items-center gap-2">
        <div className="h-3 w-3 rounded-[2px] bg-ink" />
        <span className="font-sans text-[15px] font-[500] tracking-[-0.02em] text-ink">Darric</span>
      </div>

      <div className="flex-1" />

      {isRecording && (
        <div className="flex items-center gap-2">
          <span
            className={`h-2 w-2 rounded-full ${isStopping ? "bg-ink-4" : "pulse-dot bg-danger"}`}
          />
          <span className="font-mono text-[11px] tracking-eyebrow text-ink-3 uppercase">
            {isStopping ? "finishing" : "recording"} · {formatElapsed(elapsedSeconds)}
          </span>
        </div>
      )}

      <button
        type="button"
        onClick={isRecording ? onStop : onRecord}
        disabled={disabled}
        className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken disabled:cursor-default disabled:opacity-40"
      >
        <span
          className={`h-[7px] w-[7px] rounded-full bg-accent ${
            isRecording && !isStopping ? "pulse-dot" : ""
          }`}
        />
        <span>{recordLabel(isRecording, isStarting, isStopping, downloadProgress)}</span>
      </button>
    </header>
  );
}
