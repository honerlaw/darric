import { marked } from "marked";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { getSessionTranscript } from "../lib/tauri";
import type { Note, Session, TranscriptLine } from "../types";

marked.setOptions({ breaks: true });

/* ── Helpers ──────────────────────────────────────────────────────────── */
function fmtWhen(iso: string): string {
  const d = new Date(iso);
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return "Today";
  if (d.toDateString() === yesterday.toDateString()) return "Yesterday";
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

function fmtTimestamp(iso: string): string {
  return new Date(iso).toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function fmtTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function wordCount(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function sessionDurationMin(s: Session): number | null {
  if (s.ended_at === null) return null;
  return Math.round(
    (new Date(s.ended_at).getTime() - new Date(s.started_at).getTime()) / 60000,
  );
}

function renderMarkdown(md: string): string {
  const result = marked(md);
  return typeof result === "string" ? result : "";
}

/* ── Rail item types ─────────────────────────────────────────────────── */
type RailItem =
  | { kind: "note"; item: Note; sortKey: string }
  | { kind: "session"; item: Session; sortKey: string };

function buildRailItems(notes: Note[], sessions: Session[]): RailItem[] {
  const noteItems: RailItem[] = notes.map((n) => ({
    kind: "note",
    item: n,
    sortKey: n.updated_at,
  }));
  const sessionItems: RailItem[] = sessions.map((s) => ({
    kind: "session",
    item: s,
    sortKey: s.started_at,
  }));
  return [...noteItems, ...sessionItems].sort(
    (a, b) => new Date(b.sortKey).getTime() - new Date(a.sortKey).getTime(),
  );
}

function groupRailItems(
  items: RailItem[],
): { label: string; items: RailItem[] }[] {
  const today = new Date().toDateString();
  const yesterday = new Date();
  yesterday.setDate(yesterday.getDate() - 1);
  const yStr = yesterday.toDateString();

  const todayItems: RailItem[] = [];
  const yesterdayItems: RailItem[] = [];
  const earlierItems: RailItem[] = [];

  for (const ri of items) {
    const d = new Date(ri.sortKey).toDateString();
    if (d === today) todayItems.push(ri);
    else if (d === yStr) yesterdayItems.push(ri);
    else earlierItems.push(ri);
  }
  return [
    { label: "Today", items: todayItems },
    { label: "Yesterday", items: yesterdayItems },
    { label: "Earlier", items: earlierItems },
  ].filter(({ items: is }) => is.length > 0);
}

/* ── Active selection ────────────────────────────────────────────────── */
type ActiveSelection =
  | { kind: "note"; id: string }
  | { kind: "session"; id: string }
  | null;

/* ── Note components ─────────────────────────────────────────────────── */
interface NoteEditorProps {
  note: Note;
  editTitle: string;
  editBody: string;
  onChangeTitle: (v: string) => void;
  onChangeBody: (v: string) => void;
  onDone: () => void;
  onCancel: () => void;
}

function NoteEditor({
  editTitle,
  editBody,
  onChangeTitle,
  onChangeBody,
  onDone,
  onCancel,
}: NoteEditorProps): React.JSX.Element {
  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[680px] px-10 py-8">
          <div className="font-mono text-[11px] uppercase tracking-eyebrow text-accent mb-4">
            Note
          </div>
          <input
            autoFocus
            type="text"
            value={editTitle}
            onChange={(e) => { onChangeTitle(e.target.value); }}
            placeholder="Title"
            className="mb-4 w-full bg-transparent font-sans text-[44px] font-[500] tracking-[-0.028em] leading-[1.05] text-ink outline-none placeholder-ink-4"
          />
          <textarea
            value={editBody}
            onChange={(e) => { onChangeBody(e.target.value); }}
            placeholder="Write in markdown…"
            className="min-h-[400px] w-full resize-none bg-transparent font-sans text-[16px] leading-[1.7] text-ink outline-none placeholder-ink-4"
          />
        </div>
      </div>
      <div className="shrink-0 flex items-center gap-3 border-t border-line px-10 py-4">
        <button
          type="button"
          onClick={onDone}
          className="cursor-pointer rounded-[8px] bg-accent px-4 py-2 text-[13px] font-[500] text-accent-fg hover:bg-accent-hover transition-colors"
        >
          Done
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="cursor-pointer rounded-[8px] border border-line px-4 py-2 text-[13px] text-ink-2 hover:bg-paper-sunken transition-colors"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

/* eslint-disable react/no-danger */
interface NoteBodyProps { html: string; onEdit: () => void }
function NoteBody({ html, onEdit }: NoteBodyProps): React.JSX.Element {
  return (
    <div
      className="note-lede cursor-pointer font-sans text-[16px] leading-[1.7] text-ink [&_p]:mb-[18px] [&_ul]:mb-[18px] [&_ul]:list-disc [&_ul]:pl-[22px] [&_ol]:mb-[18px] [&_ol]:list-decimal [&_ol]:pl-[22px] [&_strong]:font-[600] [&_em]:italic"
      onClick={onEdit}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
/* eslint-enable react/no-danger */

interface NoteViewerProps {
  note: Note;
  confirmDelete: boolean;
  onEdit: () => void;
  onDeleteRequest: () => void;
  onDeleteCancel: () => void;
  onDeleteConfirm: () => void;
}

function NoteViewer({
  note,
  confirmDelete,
  onEdit,
  onDeleteRequest,
  onDeleteCancel,
  onDeleteConfirm,
}: NoteViewerProps): React.JSX.Element {
  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[680px] px-10 pt-8 pb-16">
          <div className="font-mono text-[11px] uppercase tracking-eyebrow text-accent mb-4">
            Note
          </div>
          <h1
            className="font-sans text-[44px] font-[500] tracking-[-0.028em] leading-[1.05] text-ink mb-4 cursor-pointer"
            onClick={onEdit}
          >
            {note.title.length > 0 ? note.title : "Untitled"}
          </h1>
          <div className="flex items-center border-b border-line pb-3 mb-6">
            <span className="font-mono text-[11px] text-ink-3">
              {fmtTimestamp(note.updated_at)} · {String(wordCount(note.body))} words
            </span>
            <div className="flex-1" />
            <DeleteControl
              confirmDelete={confirmDelete}
              onRequest={onDeleteRequest}
              onCancel={onDeleteCancel}
              onConfirm={onDeleteConfirm}
            />
          </div>
          {note.body.length > 0 ? (
            <NoteBody html={renderMarkdown(note.body)} onEdit={onEdit} />
          ) : (
            <div className="cursor-pointer py-8 text-[15px] italic text-ink-4" onClick={onEdit}>
              Click to add content…
            </div>
          )}
        </div>
      </div>
      <div className="shrink-0 border-t border-line px-10 py-4">
        <button
          type="button"
          onClick={onEdit}
          className="cursor-pointer rounded-[8px] border border-line px-4 py-2 text-[13px] text-ink-2 hover:bg-paper-sunken transition-colors"
        >
          Edit
        </button>
      </div>
    </div>
  );
}

/* ── Meeting detail ──────────────────────────────────────────────────── */
interface MeetingDetailProps {
  session: Session;
  confirmDelete: boolean;
  onDeleteRequest: () => void;
  onDeleteCancel: () => void;
  onDeleteConfirm: () => void;
}

function MeetingDetail({
  session,
  confirmDelete,
  onDeleteRequest,
  onDeleteCancel,
  onDeleteConfirm,
}: MeetingDetailProps): React.JSX.Element {
  const [lines, setLines] = useState<TranscriptLine[]>([]);

  useEffect(() => {
    void getSessionTranscript(session.id).then(setLines);
  }, [session.id]);

  const dur = sessionDurationMin(session);
  const topic = session.topic !== null && session.topic.length > 0 ? session.topic : "Untitled meeting";

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[680px] px-10 pt-8 pb-16">
          <div className="font-mono text-[11px] uppercase tracking-eyebrow text-accent mb-4">
            Meeting
          </div>
          <h1 className="font-sans text-[44px] font-[500] tracking-[-0.028em] leading-[1.05] text-ink mb-4">
            {topic}
          </h1>
          <div className="flex items-center border-b border-line pb-3 mb-6">
            <span className="font-mono text-[11px] text-ink-3">
              {fmtTimestamp(session.started_at)}
              {dur !== null ? ` · ${String(dur)} min` : ""}
              {" · "}
              {String(lines.length)} transcript lines
            </span>
            <div className="flex-1" />
            <DeleteControl
              confirmDelete={confirmDelete}
              onRequest={onDeleteRequest}
              onCancel={onDeleteCancel}
              onConfirm={onDeleteConfirm}
            />
          </div>

          {lines.length === 0 ? (
            <div className="py-8 text-[15px] italic text-ink-4">
              {session.ended_at !== null ? "No transcript recorded." : "Recording in progress…"}
            </div>
          ) : (
            <div className="space-y-4">
              {lines.map((line) => (
                <div
                  key={line.id}
                  className="grid gap-x-[14px]"
                  style={{ gridTemplateColumns: "52px 1fr" }}
                >
                  <span className="font-mono text-[11px] text-ink-3 pt-[2px]">
                    {fmtTime(line.recorded_at)}
                  </span>
                  <p className="font-sans text-[15px] italic leading-[1.6] text-ink-2">
                    {line.content}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/* ── Shared delete control ───────────────────────────────────────────── */
interface DeleteControlProps {
  confirmDelete: boolean;
  onRequest: () => void;
  onCancel: () => void;
  onConfirm: () => void;
}

function DeleteControl({
  confirmDelete,
  onRequest,
  onCancel,
  onConfirm,
}: DeleteControlProps): React.JSX.Element {
  if (confirmDelete) {
    return (
      <div className="flex items-center gap-2">
        <span className="text-[12px] text-ink-3">Delete?</span>
        <button
          type="button"
          onClick={onConfirm}
          className="cursor-pointer text-[12px] text-danger hover:underline"
        >
          Yes, delete
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="cursor-pointer text-[12px] text-ink-3 hover:underline"
        >
          Cancel
        </button>
      </div>
    );
  }
  return (
    <button
      type="button"
      onClick={onRequest}
      className="cursor-pointer font-mono text-[11px] text-ink-4 hover:text-danger transition-colors"
    >
      Delete
    </button>
  );
}

/* ── Main screen ─────────────────────────────────────────────────────── */
interface NotesScreenProps {
  notes: Note[];
  sessions: Session[];
  initialNoteId?: string | undefined;
  onCreate: (title: string, body: string) => Promise<Note>;
  onUpdate: (id: string, title: string, body: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onDeleteSession: (id: string) => Promise<void>;
}

export function NotesScreen({
  notes,
  sessions,
  initialNoteId,
  onCreate,
  onUpdate,
  onDelete,
  onDeleteSession,
}: NotesScreenProps): React.JSX.Element {
  const [active, setActive] = useState<ActiveSelection>(
    initialNoteId !== undefined ? { kind: "note", id: initialNoteId } : null,
  );
  const [editing, setEditing] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editBody, setEditBody] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const activeNote =
    active?.kind === "note" ? (notes.find((n) => n.id === active.id) ?? null) : null;
  const activeSession =
    active?.kind === "session"
      ? (sessions.find((s) => s.id === active.id) ?? null)
      : null;

  useEffect(() => {
    if (activeNote !== null && !editing) {
      setEditTitle(activeNote.title);
      setEditBody(activeNote.body);
    }
  }, [activeNote, editing]);

  useEffect(() => {
    if (initialNoteId !== undefined) {
      setActive({ kind: "note", id: initialNoteId });
    }
  }, [initialNoteId]);

  const startEditing = (): void => {
    if (activeNote === null) return;
    setEditTitle(activeNote.title);
    setEditBody(activeNote.body);
    setEditing(true);
  };

  const autosave = (id: string, title: string, body: string): void => {
    if (saveTimer.current !== null) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => { void onUpdate(id, title, body); }, 800);
  };

  const handleCreate = async (): Promise<void> => {
    const note = await onCreate("", "");
    setActive({ kind: "note", id: note.id });
    setEditTitle("");
    setEditBody("");
    setEditing(true);
  };

  const handleDeleteNote = async (): Promise<void> => {
    if (activeNote === null) return;
    await onDelete(activeNote.id);
    setActive(null);
    setEditing(false);
    setConfirmDelete(false);
  };

  const handleDeleteSession = async (): Promise<void> => {
    if (activeSession === null) return;
    await onDeleteSession(activeSession.id);
    setActive(null);
    setConfirmDelete(false);
  };

  const allItems = buildRailItems(notes, sessions);
  const grouped = groupRailItems(allItems);

  const renderDetail = (): React.JSX.Element => {
    if (active === null) {
      return (
        <div className="flex flex-1 items-center justify-center text-[15px] italic text-ink-4">
          Select a note or meeting
        </div>
      );
    }

    if (active.kind === "session" && activeSession !== null) {
      return (
        <MeetingDetail
          session={activeSession}
          confirmDelete={confirmDelete}
          onDeleteRequest={() => { setConfirmDelete(true); }}
          onDeleteCancel={() => { setConfirmDelete(false); }}
          onDeleteConfirm={() => { void handleDeleteSession(); }}
        />
      );
    }

    if (activeNote === null) {
      return (
        <div className="flex flex-1 items-center justify-center text-[15px] italic text-ink-4">
          Select a note or meeting
        </div>
      );
    }

    if (editing) {
      return (
        <NoteEditor
          note={activeNote}
          editTitle={editTitle}
          editBody={editBody}
          onChangeTitle={(t) => {
            setEditTitle(t);
            autosave(activeNote.id, t, editBody);
          }}
          onChangeBody={(b) => {
            setEditBody(b);
            autosave(activeNote.id, editTitle, b);
          }}
          onDone={() => {
            if (saveTimer.current !== null) clearTimeout(saveTimer.current);
            void onUpdate(activeNote.id, editTitle, editBody).then(() => { setEditing(false); });
          }}
          onCancel={() => { setEditing(false); }}
        />
      );
    }

    return (
      <NoteViewer
        note={activeNote}
        confirmDelete={confirmDelete}
        onEdit={startEditing}
        onDeleteRequest={() => { setConfirmDelete(true); }}
        onDeleteCancel={() => { setConfirmDelete(false); }}
        onDeleteConfirm={() => { void handleDeleteNote(); }}
      />
    );
  };

  return (
    <div className="flex flex-1 overflow-hidden">
      {/* ── Rail ─────────────────────────────────────────────────────── */}
      <div className="flex w-[280px] shrink-0 flex-col border-r border-line overflow-hidden">
        <div className="flex h-14 items-center px-[14px] shrink-0">
          <span className="font-sans text-[18px] font-[500] text-ink">All notes</span>
          <div className="flex-1" />
          <button
            type="button"
            onClick={() => { void handleCreate(); }}
            className="cursor-pointer rounded-[8px] border border-line bg-paper px-3 py-[5px] text-[12px] text-ink-2 hover:bg-paper-sunken transition-colors"
          >
            ＋ New
          </button>
        </div>

        <div className="border-t border-line" />

        <div className="flex-1 overflow-y-auto">
          {allItems.length === 0 && (
            <div className="px-4 py-8 text-center text-[13px] italic text-ink-4">
              No notes yet
            </div>
          )}
          {grouped.map(({ label, items }) => (
            <div key={label}>
              <div className="px-[14px] py-[18px] pb-[6px] font-mono text-[10px] uppercase tracking-[0.14em] text-ink-4">
                {label}
              </div>
              {items.map((ri) => {
                if (ri.kind === "note") {
                  const note = ri.item;
                  const isActive = active?.kind === "note" && active.id === note.id;
                  return (
                    <button
                      key={`note-${note.id}`}
                      type="button"
                      onClick={() => {
                        setActive({ kind: "note", id: note.id });
                        setEditing(false);
                        setConfirmDelete(false);
                      }}
                      className={`w-full cursor-pointer px-[14px] py-[10px] text-left transition-colors hover:bg-paper-sunken ${
                        isActive ? "bg-paper-sunken" : ""
                      }`}
                    >
                      <div className="flex items-baseline gap-2">
                        <span className="flex-1 truncate text-[14px] font-[500] text-ink">
                          {note.title.length > 0 ? note.title : "Untitled"}
                        </span>
                        <span className="shrink-0 font-mono text-[11px] text-ink-4">
                          {fmtWhen(note.updated_at)}
                        </span>
                      </div>
                      <div className="mt-[2px] truncate text-[12px] italic text-ink-3">
                        {note.body.length > 0 ? note.body.slice(0, 80) : "No content"}
                      </div>
                    </button>
                  );
                }

                // session
                const session = ri.item;
                const isActive = active?.kind === "session" && active.id === session.id;
                const dur = sessionDurationMin(session);
                const topic =
                  session.topic !== null && session.topic.length > 0
                    ? session.topic
                    : "Untitled meeting";
                return (
                  <button
                    key={`session-${session.id}`}
                    type="button"
                    onClick={() => {
                      setActive({ kind: "session", id: session.id });
                      setEditing(false);
                      setConfirmDelete(false);
                    }}
                    className={`w-full cursor-pointer px-[14px] py-[10px] text-left transition-colors hover:bg-paper-sunken ${
                      isActive ? "bg-paper-sunken" : ""
                    }`}
                  >
                    <div className="flex items-baseline gap-2">
                      <span className="flex-1 truncate text-[14px] font-[500] text-ink">
                        {topic}
                      </span>
                      <span className="shrink-0 font-mono text-[11px] text-ink-4">
                        {fmtWhen(session.started_at)}
                      </span>
                    </div>
                    <div className="mt-[2px] flex items-center gap-[6px]">
                      <span className="font-mono text-[10px] uppercase tracking-[0.08em] text-accent">
                        meeting
                      </span>
                      {dur !== null && (
                        <span className="font-mono text-[11px] text-ink-3">
                          · {String(dur)} min
                        </span>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </div>

      {/* ── Detail pane ──────────────────────────────────────────────── */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {renderDetail()}
      </div>
    </div>
  );
}
