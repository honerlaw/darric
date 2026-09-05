import type React from "react";
import { groupSessionsByDate } from "../lib/utils";
import type { Session } from "../types";

interface RecordingListProps {
  sessions: Session[];
  selectedId: string | null;
  activeId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

export function RecordingList({
  sessions,
  selectedId,
  activeId,
  onSelect,
  onDelete,
}: RecordingListProps): React.JSX.Element {
  const groups = groupSessionsByDate(sessions);

  return (
    <aside className="flex w-[260px] shrink-0 flex-col overflow-y-auto border-r border-line">
      <div className="shrink-0 px-5 pt-6 pb-3 font-mono text-[11px] tracking-eyebrow text-accent uppercase">
        Recordings
      </div>

      {sessions.length === 0 && (
        <p className="px-5 text-[13px] text-ink-4 italic">
          Nothing recorded yet. Press Record to start.
        </p>
      )}

      {Object.entries(groups).map(([label, group]) => (
        <div key={label} className="mb-2">
          <div className="px-5 py-1 font-mono text-[11px] text-ink-4">{label}</div>
          {group.map((s) => (
            <div
              key={s.id}
              className={`group flex items-center gap-2 px-5 py-2 transition-colors ${
                s.id === selectedId ? "bg-accent-tint" : "hover:bg-paper-sunken"
              }`}
            >
              <button
                type="button"
                onClick={() => {
                  onSelect(s.id);
                }}
                className="flex-1 cursor-pointer truncate text-left text-[13px] text-ink"
              >
                {s.topic !== null && s.topic.length > 0 ? s.topic : "Untitled recording"}
                <span className="ml-2 font-mono text-[11px] text-ink-4">{s.recorded_minutes}m</span>
                {s.id === activeId && (
                  <span className="pulse-dot ml-1.5 inline-block h-1.5 w-1.5 rounded-full bg-danger align-middle" />
                )}
              </button>
              <button
                type="button"
                onClick={() => {
                  onDelete(s.id);
                }}
                aria-label={`Delete ${s.topic ?? "recording"}`}
                className="cursor-pointer text-[13px] text-ink-4 opacity-0 transition-opacity group-hover:opacity-100 hover:text-danger"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      ))}
    </aside>
  );
}
