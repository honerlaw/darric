import type React from "react";
import { useEffect, useRef, useState } from "react";
import { formatElapsed, formatTime } from "../lib/utils";
import type { CaptureDevice, Session, TranscriptLine } from "../types";
import { DeviceRow } from "./DeviceRow";

function emptyTranscriptMessage(
  isRecording: boolean,
  isStarting: boolean,
  isStopping: boolean,
): string {
  // Before `isRecording`, which stays true for the whole stop: a recording that
  // has already ended must not still claim to be listening for speech.
  if (isStopping) return "Finishing transcription…";
  if (isRecording) return "Listening — transcript will appear here as you speak…";
  if (isStarting) return "Starting…";
  return "No transcript for this recording.";
}

/** The line under the title: what this recording is doing right now. */
function statusLine(isRecording: boolean, isStopping: boolean, elapsedSeconds: number): string {
  if (isStopping) return `finishing — ${formatElapsed(elapsedSeconds)} recorded`;
  if (isRecording) return `${formatElapsed(elapsedSeconds)} elapsed`;
  return "stopped";
}

interface RecorderPaneProps {
  session: Session | null;
  transcriptLines: TranscriptLine[];
  devices: CaptureDevice[];
  onToggleDevice: (id: string, enabled: boolean) => void;
  /** Segments discarded because transcription fell behind real time. */
  droppedSegments: number;
  isRecording: boolean;
  isStarting: boolean;
  /**
   * A stop is in flight for *this* recording: capture has ended but the backend
   * is still flushing and transcribing. Carries the same viewing-is-active gate
   * as `isRecording`, so a past recording opened mid-stop is not labelled with
   * the active one's state.
   */
  isStopping: boolean;
  /**
   * False while any recording is in flight (resuming a second one would be
   * rejected) and while the speech model is still downloading (resuming would
   * block on it, with no way to tell). Already includes `!isRecording`.
   */
  canResume: boolean;
  elapsedSeconds: number;
  onResume: () => void;
  onRename: (topic: string) => void;
}

export function RecorderPane({
  session,
  transcriptLines,
  devices,
  onToggleDevice,
  droppedSegments,
  isRecording,
  isStarting,
  isStopping,
  canResume,
  elapsedSeconds,
  onResume,
  onRename,
}: RecorderPaneProps): React.JSX.Element {
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const endRef = useRef<HTMLDivElement>(null);
  const sessionId = session?.id ?? null;

  // Abandon an in-progress title edit when the selection moves to another
  // recording. Without this the draft survives the switch and the next commit
  // renames the newly-selected recording with the previous one's text.
  useEffect(() => {
    setEditingTitle(false);
    setTitleDraft("");
  }, [sessionId]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcriptLines.length]);

  if (session === null) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <p className="text-[14px] text-ink-4 italic">
          Select a recording, or press Record to start a new one.
        </p>
      </div>
    );
  }

  const startedStr = new Date(session.started_at).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="shrink-0 px-10 pt-8 pb-4">
        {editingTitle ? (
          <input
            autoFocus
            value={titleDraft}
            onChange={(e) => {
              setTitleDraft(e.target.value);
            }}
            onBlur={() => {
              if (titleDraft.trim() !== "") onRename(titleDraft);
              setEditingTitle(false);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                if (titleDraft.trim() !== "") onRename(titleDraft);
                setEditingTitle(false);
              }
              if (e.key === "Escape") setEditingTitle(false);
            }}
            className="w-full border-b border-accent bg-transparent font-sans text-[28px] leading-tight font-[500] tracking-[-0.022em] text-ink outline-none"
          />
        ) : (
          <h2
            className="cursor-text font-sans text-[28px] leading-tight font-[500] tracking-[-0.022em] text-ink"
            onClick={() => {
              setTitleDraft(session.topic ?? "");
              setEditingTitle(true);
            }}
          >
            {session.topic !== null && session.topic.length > 0
              ? session.topic
              : "Untitled recording"}
          </h2>
        )}

        <div className="mt-2 mb-4 font-mono text-[12px] text-ink-3">
          started {startedStr} · {statusLine(isRecording, isStopping, elapsedSeconds)}
        </div>

        <DeviceRow devices={devices} onToggle={onToggleDevice} />
      </div>

      <div className="flex-1 overflow-y-auto border-t border-line px-10 py-5">
        {droppedSegments > 0 && (
          <p className="mb-4 font-mono text-[11px] text-danger">
            Transcription fell behind — {String(droppedSegments)} segment
            {droppedSegments === 1 ? "" : "s"} dropped. Audio was captured; some speech is missing
            from the transcript.
          </p>
        )}

        {transcriptLines.length === 0 ? (
          <p className="text-[14px] text-ink-4 italic">
            {emptyTranscriptMessage(isRecording, isStarting, isStopping)}
          </p>
        ) : (
          <div className="space-y-3">
            {transcriptLines.map((line, i) => (
              <div
                key={line.id}
                className="grid gap-x-[14px]"
                style={{ gridTemplateColumns: "52px 120px 1fr" }}
              >
                <span className="pt-[3px] font-mono text-[11px] text-ink-3">
                  {formatTime(line.recorded_at)}
                </span>
                <span
                  className="truncate pt-[3px] font-mono text-[11px] text-accent"
                  title={`${line.device_name} (${line.direction})`}
                >
                  {line.device_name}
                </span>
                <p className="font-sans text-[15px] leading-[1.6] text-ink-2">
                  {line.content}
                  {i === transcriptLines.length - 1 && isRecording && !isStopping && (
                    <span className="caret-blink ml-[2px] text-accent">▍</span>
                  )}
                </p>
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>

      {canResume && (
        <div className="flex shrink-0 items-center gap-3 border-t border-line px-10 py-4">
          <button
            type="button"
            onClick={onResume}
            className="cursor-pointer rounded-[8px] bg-accent px-4 py-2 text-[13px] font-[500] text-accent-fg transition-colors hover:bg-accent-hover"
          >
            Resume recording
          </button>
        </div>
      )}
    </div>
  );
}
