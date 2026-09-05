import type React from "react";
import { useEffect, useRef, useState } from "react";
import { formatElapsed, formatTime } from "../lib/utils";
import type { CaptureDevice, Session, TranscriptLine } from "../types";
import { DeviceRow } from "./DeviceRow";

function emptyTranscriptMessage(isRecording: boolean, isStarting: boolean): string {
  if (isRecording) return "Listening — transcript will appear here as you speak…";
  if (isStarting) return "Starting…";
  return "No transcript for this recording.";
}

interface RecorderPaneProps {
  session: Session | null;
  transcriptLines: TranscriptLine[];
  devices: CaptureDevice[];
  isRecording: boolean;
  isStarting: boolean;
  /** False while any recording is in flight — resuming a second one would be rejected. */
  canResume: boolean;
  elapsedSeconds: number;
  downloadProgress: number | null;
  onResume: () => void;
  onRename: (topic: string) => void;
}

export function RecorderPane({
  session,
  transcriptLines,
  devices,
  isRecording,
  isStarting,
  canResume,
  elapsedSeconds,
  downloadProgress,
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
          started {startedStr} ·{" "}
          {isRecording ? `${formatElapsed(elapsedSeconds)} elapsed` : "stopped"}
        </div>

        <DeviceRow devices={devices} />
      </div>

      <div className="flex-1 overflow-y-auto border-t border-line px-10 py-5">
        {downloadProgress !== null && (
          <div className="mb-5 flex flex-col gap-2">
            <p className="text-[13px] text-ink-3">
              Downloading speech model ({String(downloadProgress)}%)…
            </p>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-paper-sunken">
              <div
                className="h-full rounded-full bg-accent transition-all duration-300"
                style={{ width: `${String(downloadProgress)}%` }}
              />
            </div>
          </div>
        )}

        {transcriptLines.length === 0 ? (
          <p className="text-[14px] text-ink-4 italic">
            {emptyTranscriptMessage(isRecording, isStarting)}
          </p>
        ) : (
          <div className="space-y-3">
            {transcriptLines.map((line, i) => (
              <div
                key={line.id}
                className="grid gap-x-[14px]"
                style={{ gridTemplateColumns: "52px 1fr" }}
              >
                <span className="pt-[3px] font-mono text-[11px] text-ink-3">
                  {formatTime(line.recorded_at)}
                </span>
                <p className="font-sans text-[15px] leading-[1.6] text-ink-2">
                  {line.content}
                  {i === transcriptLines.length - 1 && isRecording && (
                    <span className="caret-blink ml-[2px] text-accent">▍</span>
                  )}
                </p>
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>

      {!isRecording && canResume && (
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
