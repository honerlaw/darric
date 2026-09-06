import type React from "react";
import { useRef, useState } from "react";
import { groupSessionsByDate, sessionLabel } from "../lib/utils";
import type { Session } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";

/** The row's delete affordance. A can, not an `×` — the action destroys the recording. */
function TrashIcon(): React.JSX.Element {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="h-[14px] w-[14px]"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.3"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M2.5 4h11" />
      <path d="M6.5 4V2.8h3V4" />
      <path d="M4 4l.6 8.4a1 1 0 0 0 1 .9h4.8a1 1 0 0 0 1-.9L12 4" />
      <path d="M6.7 6.5v4.4M9.3 6.5v4.4" />
    </svg>
  );
}

interface RecordingListProps {
  sessions: Session[];
  selectedId: string | null;
  activeId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  /** Called with a trimmed, non-empty topic that differs from the current one. */
  onRename: (id: string, topic: string) => void;
}

interface TitleEdit {
  id: string;
  draft: string;
}

export function RecordingList({
  sessions,
  selectedId,
  activeId,
  onSelect,
  onDelete,
  onRename,
}: RecordingListProps): React.JSX.Element {
  const groups = groupSessionsByDate(sessions);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [editing, setEditing] = useState<TitleEdit | null>(null);
  // Escape sets this and then blurs the input, so the blur handler below is
  // the one place an edit ends. Enter blurs too. One close path means a commit
  // cannot fire twice (Enter, then the blur its own unmount would produce) and
  // a cancel cannot be undone by a trailing blur that commits the draft anyway.
  const cancelEditRef = useRef(false);

  const finishEdit = (session: Session): void => {
    const cancelled = cancelEditRef.current;
    cancelEditRef.current = false;
    if (editing?.id !== session.id) return;
    setEditing(null);
    if (cancelled) return;
    const topic = editing.draft.trim();
    // An empty draft is an abandoned edit, not a request to clear the name —
    // the pane's title editor makes the same call.
    if (topic === "" || topic === session.topic) return;
    onRename(session.id, topic);
  };

  // Resolved from the live list rather than stored alongside the id: a session
  // deleted from elsewhere while the dialog is open must close it, not leave a
  // prompt naming a recording that no longer exists.
  const pendingDelete = sessions.find((s) => s.id === pendingDeleteId) ?? null;

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
              {editing?.id === s.id ? (
                <input
                  autoFocus
                  value={editing.draft}
                  aria-label={`Rename ${sessionLabel(s)}`}
                  onFocus={(e) => {
                    e.currentTarget.select();
                  }}
                  onChange={(e) => {
                    setEditing({ id: s.id, draft: e.target.value });
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") cancelEditRef.current = true;
                    if (e.key === "Enter" || e.key === "Escape") e.currentTarget.blur();
                  }}
                  onBlur={() => {
                    finishEdit(s);
                  }}
                  className="min-w-0 flex-1 border-b border-accent bg-transparent text-[13px] text-ink outline-none"
                />
              ) : (
                <button
                  type="button"
                  onClick={() => {
                    onSelect(s.id);
                  }}
                  onDoubleClick={() => {
                    // The draft is the stored topic, never the "Untitled
                    // recording" placeholder — committing that verbatim would
                    // turn a null topic into a literal one.
                    setEditing({ id: s.id, draft: s.topic ?? "" });
                  }}
                  className="flex-1 cursor-pointer truncate text-left text-[13px] text-ink"
                >
                  {sessionLabel(s)}
                  <span className="ml-2 font-mono text-[11px] text-ink-4">
                    {s.recorded_minutes}m
                  </span>
                  {s.id === activeId && (
                    <span className="pulse-dot ml-1.5 inline-block h-1.5 w-1.5 rounded-full bg-danger align-middle" />
                  )}
                </button>
              )}
              <button
                type="button"
                onClick={() => {
                  setPendingDeleteId(s.id);
                }}
                aria-label={`Delete ${sessionLabel(s)}`}
                className="cursor-pointer text-ink-4 opacity-0 transition-opacity group-hover:opacity-100 hover:text-danger focus-visible:opacity-100"
              >
                <TrashIcon />
              </button>
            </div>
          ))}
        </div>
      ))}

      {pendingDelete !== null && (
        <ConfirmDialog
          title={`Delete “${sessionLabel(pendingDelete)}”?`}
          body="This removes the recording and its transcript. It cannot be undone."
          confirmLabel="Delete"
          onConfirm={() => {
            onDelete(pendingDelete.id);
            setPendingDeleteId(null);
          }}
          onCancel={() => {
            setPendingDeleteId(null);
          }}
        />
      )}
    </aside>
  );
}
