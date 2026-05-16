import type React from "react";
import { useEffect, useRef, useState } from "react";
import { TagInput } from "../components/tags/TagInput";
import type { Tag, TranscriptLine } from "../types";

const SPEAKER_COLORS: Record<number, string> = {
  0: "bg-blue-500/20 text-blue-300 ring-blue-500/30",
  1: "bg-emerald-500/20 text-emerald-300 ring-emerald-500/30",
  2: "bg-amber-500/20 text-amber-300 ring-amber-500/30",
  3: "bg-purple-500/20 text-purple-300 ring-purple-500/30",
  4: "bg-rose-500/20 text-rose-300 ring-rose-500/30",
};

function speakerColor(label: string | undefined): string {
  if (label === undefined || label === "") return "bg-neutral-700/40 text-neutral-400 ring-neutral-600/30";
  const num = parseInt(label.replace("Speaker ", ""), 10);
  return SPEAKER_COLORS[(num - 1) % 5] ?? "bg-blue-500/20 text-blue-300 ring-blue-500/30";
}

function speakerShort(label: string | undefined): string {
  if (label === undefined || label === "") return "MIC";
  const num = label.replace("Speaker ", "");
  return `S${num}`;
}

function fmtTimestamp(iso: string): string {
  return new Date(iso).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

interface MeetingScreenProps {
  sessionId: string | null;
  sessionTopic: string | null;
  sessionStartedAt: string | null;
  sessionNotes: string;
  sessionTags: Tag[];
  allTags: Tag[];
  transcriptLines: TranscriptLine[];
  isRecording: boolean;
  elapsedSeconds: number;
  canResume: boolean;
  onStop: () => void;
  onResume: () => void;
  onUpdateTitle: (title: string) => void;
  onSaveNotes: (notes: string) => void;
  onAddTag: (name: string) => void;
  onRemoveTag: (tagId: string) => void;
}

function formatElapsed(sec: number): string {
  const m = Math.floor(sec / 60)
    .toString()
    .padStart(2, "0");
  const s = (sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function todayShort(): string {
  const d = new Date();
  const days = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
  const months = [
    "jan", "feb", "mar", "apr", "may", "jun",
    "jul", "aug", "sep", "oct", "nov", "dec",
  ];
  const day = days[d.getDay()] ?? "?";
  const month = months[d.getMonth()] ?? "?";
  return `${day} ${String(d.getDate())} ${month}`;
}

export function MeetingScreen({
  sessionId,
  sessionTopic,
  sessionStartedAt,
  sessionNotes,
  sessionTags,
  allTags,
  transcriptLines,
  isRecording,
  elapsedSeconds,
  canResume,
  onStop,
  onResume,
  onUpdateTitle,
  onSaveNotes,
  onAddTag,
  onRemoveTag,
}: MeetingScreenProps): React.JSX.Element {
  const [notes, setNotes] = useState(sessionNotes);
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const transcriptEndRef = useRef<HTMLDivElement>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset notes only when the session changes, not on every autosave ACK.
  // Omitting sessionNotes from deps is intentional: including it would overwrite
  // local edits whenever the server reflects a saved value back through the prop.
  useEffect(() => {
    if (saveTimerRef.current !== null) clearTimeout(saveTimerRef.current);
    setNotes(sessionNotes);
  }, [sessionId]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    return () => {
      if (saveTimerRef.current !== null) clearTimeout(saveTimerRef.current);
    };
  }, []);

  useEffect(() => {
    transcriptEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcriptLines.length]);

  const startedStr =
    sessionStartedAt !== null
      ? new Date(sessionStartedAt).toLocaleTimeString("en-US", {
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
        })
      : "—";

  return (
    <div className="flex flex-1 overflow-hidden">
      {/* ── Left: Notes pane ─────────────────────────────────────────── */}
      <div className="flex flex-[1.3] flex-col overflow-hidden border-r border-line">
        <div className="px-10 pt-8 pb-0 shrink-0">
          <div className="font-mono text-[11px] uppercase tracking-eyebrow text-accent mb-2">
            Meeting
          </div>
          {editingTitle ? (
            <input
              autoFocus
              value={titleDraft}
              onChange={(e) => { setTitleDraft(e.target.value); }}
              onBlur={() => { if (titleDraft.trim() !== "") onUpdateTitle(titleDraft); setEditingTitle(false); }}
              onKeyDown={(e) => {
                if (e.key === "Enter") { if (titleDraft.trim() !== "") onUpdateTitle(titleDraft); setEditingTitle(false); }
                if (e.key === "Escape") { setEditingTitle(false); }
              }}
              className="font-sans text-[32px] font-[500] tracking-[-0.022em] leading-tight text-ink bg-transparent border-b border-accent outline-none w-full"
            />
          ) : (
            <h2
              className="font-sans text-[32px] font-[500] tracking-[-0.022em] leading-tight text-ink cursor-text"
              onClick={() => { setTitleDraft(sessionTopic ?? ""); setEditingTitle(true); }}
            >
              {sessionTopic !== null && sessionTopic.length > 0 ? sessionTopic : "Untitled meeting"}
            </h2>
          )}
          <div className="font-mono text-[12px] text-ink-3 mt-2 mb-3">
            {todayShort()} · started {startedStr}
          </div>

          <div className="mb-5 border-b border-line pb-4">
            <TagInput
              tags={sessionTags}
              allTags={allTags}
              onAdd={onAddTag}
              onRemove={onRemoveTag}
            />
          </div>
        </div>

        <div className="flex-1 overflow-hidden px-10 pb-6">
          <textarea
            value={notes}
            onChange={(e) => {
              const value = e.target.value;
              setNotes(value);
              if (saveTimerRef.current !== null) clearTimeout(saveTimerRef.current);
              saveTimerRef.current = setTimeout(() => { onSaveNotes(value); }, 500);
            }}
            placeholder="Type your notes here…"
            className="h-full w-full resize-none bg-transparent font-sans text-[16px] leading-[1.7] text-ink placeholder-ink-4 outline-none"
          />
        </div>

        <div className="shrink-0 flex items-center gap-3 px-10 py-5 border-t border-line">
          {isRecording && (
            <button
              type="button"
              onClick={onStop}
              className="rounded-[8px] bg-accent px-4 py-2 text-[13px] font-[500] text-accent-fg hover:bg-accent-hover transition-colors cursor-pointer"
            >
              End meeting
            </button>
          )}
          {!isRecording && canResume && (
            <button
              type="button"
              onClick={onResume}
              className="rounded-[8px] bg-accent px-4 py-2 text-[13px] font-[500] text-accent-fg hover:bg-accent-hover transition-colors cursor-pointer"
            >
              Resume recording
            </button>
          )}
          <span className="font-mono text-[11px] text-ink-4">
            {isRecording ? `${formatElapsed(elapsedSeconds)} elapsed` : "Recording stopped"}
          </span>
        </div>
      </div>

      {/* ── Right: Transcript pane ───────────────────────────────────── */}
      <div className="flex flex-1 flex-col overflow-hidden bg-accent-tint">
        <div className="px-8 pt-8 pb-1.5 shrink-0">
          <div className="font-mono text-[11px] uppercase tracking-eyebrow text-accent mb-2">
            Transcript
          </div>
          <div className="font-sans text-[22px] font-[500] text-ink">
            {isRecording ? "Live" : "Ended"}
          </div>
          <div className="font-mono text-[12px] text-ink-3 mt-1">
            recorded by Darric · {formatElapsed(elapsedSeconds)} elapsed
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-8 py-4">
          {transcriptLines.length === 0 ? (
            <div className="py-8 text-[14px] italic text-ink-4">
              Transcript will appear here as you speak…
            </div>
          ) : (
            <div className="space-y-4">
              {transcriptLines.map((line, i) => {
                const isLast = i === transcriptLines.length - 1;
                return (
                  <div
                    key={line.id}
                    className="grid gap-x-[14px]"
                    style={{ gridTemplateColumns: "52px 32px 1fr" }}
                  >
                    <span className="font-mono text-[11px] text-ink-3 pt-[3px]">
                      {fmtTimestamp(line.recorded_at)}
                    </span>
                    <span
                      className={`mt-[1px] h-fit rounded px-1 py-[1px] text-[10px] font-semibold ring-1 ${speakerColor(line.speaker_label)}`}
                    >
                      {speakerShort(line.speaker_label)}
                    </span>
                    <p className="font-sans text-[15px] italic leading-[1.6] text-ink-2">
                      {line.content}
                      {isLast && isRecording && (
                        <span className="caret-blink ml-[2px] text-accent">▍</span>
                      )}
                    </p>
                  </div>
                );
              })}
              <div ref={transcriptEndRef} />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
