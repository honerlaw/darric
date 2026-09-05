import { marked } from "marked";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { TagInput } from "../tags/TagInput";
import type { Note, Tag } from "../../types";

marked.setOptions({ breaks: true });

function renderMarkdown(md: string): string {
  const result = marked(md);
  return typeof result === "string" ? result : "";
}

/* eslint-disable react/no-danger */
function NoteBody({ html, onEdit }: { html: string; onEdit: () => void }): React.JSX.Element {
  return (
    <div
      className="note-lede cursor-pointer font-sans text-[16px] leading-[1.7] text-ink [&_em]:italic [&_ol]:mb-[18px] [&_ol]:list-decimal [&_ol]:pl-[22px] [&_p]:mb-[18px] [&_strong]:font-[600] [&_ul]:mb-[18px] [&_ul]:list-disc [&_ul]:pl-[22px]"
      onClick={onEdit}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
/* eslint-enable react/no-danger */

interface NoteModalProps {
  note: Note;
  allTags: Tag[];
  onUpdate: (id: string, title: string, body: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onClose: () => void;
  onAddTag: (noteId: string, name: string) => void;
  onRemoveTag: (noteId: string, tagId: string) => void;
}

export function NoteModal({
  note,
  allTags,
  onUpdate,
  onDelete,
  onClose,
  onAddTag,
  onRemoveTag,
}: NoteModalProps): React.JSX.Element {
  const [editing, setEditing] = useState(note.title.length === 0 && note.body.length === 0);
  const [editTitle, setEditTitle] = useState(note.title);
  const [editBody, setEditBody] = useState(note.body);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Sync if note prop changes (e.g. after save)
  useEffect(() => {
    if (!editing) {
      setEditTitle(note.title);
      setEditBody(note.body);
    }
  }, [note.title, note.body, editing]);

  const autosave = (title: string, body: string): void => {
    if (saveTimer.current !== null) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      void onUpdate(note.id, title, body);
    }, 600);
  };

  const flushSave = (): void => {
    if (saveTimer.current !== null) {
      clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    void onUpdate(note.id, editTitle, editBody);
  };

  const handleClose = (): void => {
    flushSave();
    onClose();
  };

  const handleDelete = async (): Promise<void> => {
    if (saveTimer.current !== null) clearTimeout(saveTimer.current);
    await onDelete(note.id);
    onClose();
  };

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if (e.key === "Escape") handleClose();
    };
    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editTitle, editBody]);

  return (
    /* Backdrop */
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/20 backdrop-blur-[2px]"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      {/* Panel */}
      <div className="relative flex max-h-[82vh] w-[760px] flex-col rounded-[14px] bg-paper shadow-[0_8px_32px_rgba(10,10,10,0.14)]">
        {/* Top bar */}
        <div className="flex shrink-0 items-center justify-between border-b border-line px-8 py-4">
          <span className="font-mono text-[11px] tracking-eyebrow text-accent uppercase">Note</span>
          <div className="flex items-center gap-4">
            {confirmDelete ? (
              <div className="flex items-center gap-2">
                <span className="text-[12px] text-ink-3">Delete this note?</span>
                <button
                  type="button"
                  onClick={() => {
                    void handleDelete();
                  }}
                  className="cursor-pointer text-[12px] text-danger hover:underline"
                >
                  Yes, delete
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setConfirmDelete(false);
                  }}
                  className="cursor-pointer text-[12px] text-ink-3 hover:underline"
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                type="button"
                onClick={() => {
                  setConfirmDelete(true);
                }}
                className="cursor-pointer font-mono text-[11px] text-ink-4 transition-colors hover:text-danger"
              >
                Delete
              </button>
            )}
            <button
              type="button"
              onClick={handleClose}
              className="flex h-6 w-6 cursor-pointer items-center justify-center rounded-full text-[16px] leading-none text-ink-3 transition-colors hover:bg-paper-sunken hover:text-ink"
              aria-label="Close"
            >
              ×
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-8 py-6">
          {editing ? (
            <>
              <input
                autoFocus
                type="text"
                value={editTitle}
                onChange={(e) => {
                  setEditTitle(e.target.value);
                  autosave(e.target.value, editBody);
                }}
                onBlur={() => {
                  if (editBody.length > 0) setEditing(false);
                }}
                placeholder="Title"
                className="mb-4 w-full bg-transparent font-sans text-[36px] leading-[1.1] font-[500] tracking-[-0.024em] text-ink placeholder-ink-4 outline-none"
              />
              <textarea
                value={editBody}
                onChange={(e) => {
                  setEditBody(e.target.value);
                  autosave(editTitle, e.target.value);
                }}
                placeholder="Write in markdown…"
                className="min-h-[260px] w-full resize-none bg-transparent font-sans text-[16px] leading-[1.7] text-ink placeholder-ink-4 outline-none"
              />
            </>
          ) : (
            <>
              <h2
                className="mb-4 cursor-pointer font-sans text-[36px] leading-[1.1] font-[500] tracking-[-0.024em] text-ink"
                onClick={() => {
                  setEditing(true);
                }}
              >
                {editTitle.length > 0 ? editTitle : "Untitled"}
              </h2>
              {editBody.length > 0 ? (
                <NoteBody
                  html={renderMarkdown(editBody)}
                  onEdit={() => {
                    setEditing(true);
                  }}
                />
              ) : (
                <div
                  className="cursor-pointer py-4 text-[15px] text-ink-4 italic"
                  onClick={() => {
                    setEditing(true);
                  }}
                >
                  Click to add content…
                </div>
              )}
            </>
          )}

          {/* Tags */}
          <div className="border-t border-line pt-4">
            <TagInput
              tags={note.tags}
              allTags={allTags}
              onAdd={(name) => {
                onAddTag(note.id, name);
              }}
              onRemove={(tagId) => {
                onRemoveTag(note.id, tagId);
              }}
            />
          </div>
        </div>

        {/* Footer */}
        {editing && (
          <div className="flex shrink-0 items-center justify-end border-t border-line px-8 py-3">
            <button
              type="button"
              onClick={() => {
                flushSave();
                setEditing(false);
              }}
              className="cursor-pointer rounded-[8px] bg-accent px-4 py-2 text-[13px] font-[500] text-accent-fg transition-colors hover:bg-accent-hover"
            >
              Done
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
