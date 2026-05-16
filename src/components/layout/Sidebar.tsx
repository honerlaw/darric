import type React from "react";
import { groupSessionsByDate } from "../../lib/utils";
import type { Session } from "../../types";

interface SidebarProps {
  sessions: Session[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
}

export function Sidebar({
  sessions,
  activeSessionId,
  onSelectSession,
}: SidebarProps): React.JSX.Element {
  const groups = groupSessionsByDate(sessions);

  return (
    <div className="flex h-full flex-col overflow-y-auto border-r border-neutral-800 bg-neutral-900">
      <div className="px-3 py-3 text-xs font-semibold tracking-wider text-neutral-500 uppercase">
        Sessions
      </div>
      {Object.entries(groups).map(([label, group]) => (
        <div key={label}>
          <div className="px-3 py-1 text-xs text-neutral-600">{label}</div>
          {group.map((s) => (
            <button
              key={s.id}
              onClick={() => {
                onSelectSession(s.id);
              }}
              className={`w-full truncate px-3 py-2 text-left text-sm transition-colors ${
                s.id === activeSessionId
                  ? "bg-neutral-700 text-neutral-100"
                  : "text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
              }`}
            >
              {s.topic ?? "Untitled session"}
              {s.ended_at === null && (
                <span className="ml-1.5 inline-block h-1.5 w-1.5 rounded-full bg-red-500 align-middle" />
              )}
            </button>
          ))}
        </div>
      ))}
      {sessions.length === 0 && (
        <p className="px-3 py-2 text-xs text-neutral-600">
          No sessions yet. Press Record to start.
        </p>
      )}
    </div>
  );
}
