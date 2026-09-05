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

/** What a screen reader hears when the recording phase changes. */
function phaseAnnouncement(isRecording: boolean, isStopping: boolean): string {
  if (isStopping) return "Stopping — finishing transcription";
  if (isRecording) return "Recording";
  return "";
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
  /**
   * The name of the recording Resume would continue, or null when there is
   * nothing to continue: no recording is selected, one is already in flight, a
   * start is already in flight, or the speech model is still downloading. Null
   * hides the button rather than disabling it — there is nothing for it to act
   * on. Carrying the name rather than a bare boolean is what lets the button say
   * which recording it appends to, now that it no longer sits beside one.
   */
  resumeTarget: string | null;
  onRecord: () => void;
  onStop: () => void;
  onResume: () => void;
}

export function Header({
  isRecording,
  isStarting,
  isStopping,
  downloadProgress,
  elapsedSeconds,
  resumeTarget,
  onRecord,
  onStop,
  onResume,
}: HeaderProps): React.JSX.Element {
  // A start that cannot proceed uses the native `disabled`: nothing was in
  // progress on the button and there is no interaction to preserve. A stop in
  // flight does not — the user has just pressed this button, and `disabled`
  // moves their focus to the body mid-interaction while announcing nothing.
  // `aria-disabled` keeps focus, and `useSession.stop` rejects the re-entrant
  // call, so the click it still delivers is inert.
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
          <span
            className={`h-2 w-2 rounded-full ${isStopping ? "bg-ink-4" : "pulse-dot bg-danger"}`}
          />
          <span className="font-mono text-[11px] tracking-eyebrow text-ink-3 uppercase">
            {isStopping ? "finishing" : "recording"} · {formatElapsed(elapsedSeconds)}
          </span>
        </div>
      )}

      {/* The button's own label change is not announced, and the elapsed time in
          the indicator above re-renders every second — a live region on that
          would narrate the clock. This carries the phase alone, so it speaks
          once per transition. */}
      <span role="status" aria-live="polite" className="sr-only">
        {phaseAnnouncement(isRecording, isStopping)}
      </span>

      {resumeTarget !== null && (
        // Sits beside Record because it is the same action against an existing
        // recording: both ways to begin capturing are in one place.
        <button
          type="button"
          onClick={onResume}
          aria-label={`Resume recording “${resumeTarget}”`}
          className="flex h-[30px] cursor-pointer items-center rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken"
        >
          Resume
        </button>
      )}

      <button
        type="button"
        onClick={isRecording ? onStop : onRecord}
        disabled={cannotStart}
        aria-disabled={isStopping}
        className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken disabled:cursor-default disabled:opacity-40 aria-disabled:cursor-default aria-disabled:opacity-40"
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
