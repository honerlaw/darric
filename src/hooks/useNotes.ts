import { useCallback, useEffect, useState } from "react";
import { createNote, deleteNote, listNotes, updateNote } from "../lib/tauri";
import type { Note } from "../types";

interface UseNotesReturn {
  notes: Note[];
  create: (title: string, body: string) => Promise<Note>;
  update: (id: string, title: string, body: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useNotes(): UseNotesReturn {
  const [notes, setNotes] = useState<Note[]>([]);

  const refresh = useCallback(async (): Promise<void> => {
    const list = await listNotes();
    setNotes(list);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const create = useCallback(
    async (title: string, body: string): Promise<Note> => {
      const note = await createNote(title, body);
      await refresh();
      return note;
    },
    [refresh],
  );

  const update = useCallback(
    async (id: string, title: string, body: string): Promise<void> => {
      await updateNote(id, title, body);
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (id: string): Promise<void> => {
      await deleteNote(id);
      await refresh();
    },
    [refresh],
  );

  return { notes, create, update, remove, refresh };
}
