import type React from "react";
import { formatElapsed } from "../../lib/utils";

function recordLabel(isRecording: boolean, isStarting: boolean): string {
  if (isRecording) return "Stop";
  if (isStarting) return "Starting…";
  return "Record";
}

interface HeaderProps {
  isRecording: boolean;
  isStarting: boolean;
  elapsedSeconds: number;
  onRecord: () => void;
  onStop: () => void;
}

export function Header({
  isRecording,
  isStarting,
  elapsedSeconds,
  onRecord,
  onStop,
}: HeaderProps): React.JSX.Element {
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
        disabled={isStarting}
        className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken disabled:cursor-default disabled:opacity-40"
      >
        <span
          className={`h-[7px] w-[7px] rounded-full bg-accent ${isRecording ? "pulse-dot" : ""}`}
        />
        <span>{recordLabel(isRecording, isStarting)}</span>
      </button>
    </header>
  );
}
