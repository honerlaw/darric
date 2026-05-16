import type React from "react";
import { useEffect, useRef } from "react";
import { formatTime } from "../../lib/utils";
import type { TranscriptLine } from "../../types";

const PANEL_SPEAKER_COLORS: Record<number, string> = {
  0: "bg-blue-900/50 text-blue-300",
  1: "bg-emerald-900/50 text-emerald-300",
  2: "bg-amber-900/50 text-amber-300",
  3: "bg-purple-900/50 text-purple-300",
  4: "bg-rose-900/50 text-rose-300",
};

function SpeakerBadge({
  label,
  source,
}: {
  label: string | undefined;
  source: string;
}): React.JSX.Element {
  if (label !== undefined && label !== "") {
    const num = parseInt(label.replace("Speaker ", ""), 10);
    const colorClass = PANEL_SPEAKER_COLORS[(num - 1) % 5] ?? "bg-blue-900/50 text-blue-300";
    return (
      <span className={`mt-0.5 shrink-0 rounded px-1.5 py-0.5 text-xs font-medium uppercase ${colorClass}`}>
        S{num}
      </span>
    );
  }
  return (
    <span
      className={`mt-0.5 shrink-0 rounded px-1.5 py-0.5 text-xs font-medium uppercase ${
        source === "mic" ? "bg-blue-900/50 text-blue-300" : "bg-green-900/50 text-green-300"
      }`}
    >
      {source === "mic" ? "MIC" : "SPK"}
    </span>
  );
}

interface TranscriptPanelProps {
  lines: TranscriptLine[];
  isLive: boolean;
  isStarting: boolean;
  modelReady: boolean;
  downloadProgress: number | null;
}

export function TranscriptPanel({
  lines,
  isLive,
  isStarting,
  modelReady,
  downloadProgress,
}: TranscriptPanelProps): React.JSX.Element {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isLive) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [lines, isLive]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center gap-2 border-b border-neutral-800 px-4 py-2">
        <span className="text-xs font-semibold tracking-wider text-neutral-500 uppercase">
          Transcript
        </span>
        {isLive && (
          <span className="flex items-center gap-1 text-xs text-emerald-500">
            <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500" />
            Live
          </span>
        )}
      </div>
      <div className="flex-1 space-y-2 overflow-y-auto p-4">
        {downloadProgress !== null && (
          <div className="mt-4 flex flex-col gap-2">
            <p className="text-sm text-neutral-400">
              Downloading speech model ({String(downloadProgress)}%)…
            </p>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-neutral-800">
              <div
                className="h-full rounded-full bg-indigo-500 transition-all duration-300"
                style={{ width: `${String(downloadProgress)}%` }}
              />
            </div>
            <p className="text-xs text-neutral-600">~466 MB, one-time download</p>
          </div>
        )}
        {downloadProgress === null && !modelReady && !isLive && lines.length === 0 && (
          <p className="mt-4 text-sm text-neutral-500 italic">Loading speech model…</p>
        )}
        {isStarting && downloadProgress === null && modelReady && (
          <p className="mt-4 text-sm text-neutral-500 italic">Starting session…</p>
        )}
        {!isStarting && lines.length === 0 && (modelReady || isLive) && (
          <p className="text-sm text-neutral-600 italic">
            {isLive
              ? "Listening... transcript will appear here."
              : "No transcript for this session."}
          </p>
        )}
        {lines.map((line) => (
          <div key={line.id} className="flex items-start gap-3">
            <SpeakerBadge label={line.speaker_label} source={line.source} />
            <p className="flex-1 text-sm leading-relaxed text-neutral-200">{line.content}</p>
            <span className="mt-0.5 shrink-0 text-xs text-neutral-600 tabular-nums">
              {formatTime(line.recorded_at)}
            </span>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
