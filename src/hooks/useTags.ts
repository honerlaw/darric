import { useCallback, useEffect, useState } from "react";
import {
  addTagToNote,
  addTagToSession,
  addTagToTask,
  listTags,
  removeTagFromNote,
  removeTagFromSession,
  removeTagFromTask,
} from "../lib/tauri";
import type { Tag } from "../types";

interface UseTagsOptions {
  onRefreshSessions: () => Promise<void>;
  onRefreshNotes: () => Promise<void>;
  onRefreshTasks: () => Promise<void>;
}

interface UseTagsReturn {
  allTags: Tag[];
  refreshTags: () => Promise<void>;
  addToSession: (sessionId: string, tagName: string) => Promise<void>;
  removeFromSession: (sessionId: string, tagId: string) => Promise<void>;
  addToNote: (noteId: string, tagName: string) => Promise<void>;
  removeFromNote: (noteId: string, tagId: string) => Promise<void>;
  addToTask: (taskId: string, tagName: string) => Promise<void>;
  removeFromTask: (taskId: string, tagId: string) => Promise<void>;
}

export function useTags({
  onRefreshSessions,
  onRefreshNotes,
  onRefreshTasks,
}: UseTagsOptions): UseTagsReturn {
  const [allTags, setAllTags] = useState<Tag[]>([]);

  const refreshTags = useCallback(async (): Promise<void> => {
    const tags = await listTags();
    setAllTags(tags);
  }, []);

  useEffect(() => {
    void refreshTags();
  }, [refreshTags]);

  const addToSession = useCallback(
    async (sessionId: string, tagName: string): Promise<void> => {
      await addTagToSession(sessionId, tagName);
      await Promise.all([refreshTags(), onRefreshSessions()]);
    },
    [refreshTags, onRefreshSessions],
  );

  const removeFromSession = useCallback(
    async (sessionId: string, tagId: string): Promise<void> => {
      await removeTagFromSession(sessionId, tagId);
      await Promise.all([refreshTags(), onRefreshSessions()]);
    },
    [refreshTags, onRefreshSessions],
  );

  const addToNote = useCallback(
    async (noteId: string, tagName: string): Promise<void> => {
      await addTagToNote(noteId, tagName);
      await Promise.all([refreshTags(), onRefreshNotes()]);
    },
    [refreshTags, onRefreshNotes],
  );

  const removeFromNote = useCallback(
    async (noteId: string, tagId: string): Promise<void> => {
      await removeTagFromNote(noteId, tagId);
      await Promise.all([refreshTags(), onRefreshNotes()]);
    },
    [refreshTags, onRefreshNotes],
  );

  const addToTask = useCallback(
    async (taskId: string, tagName: string): Promise<void> => {
      await addTagToTask(taskId, tagName);
      await Promise.all([refreshTags(), onRefreshTasks()]);
    },
    [refreshTags, onRefreshTasks],
  );

  const removeFromTask = useCallback(
    async (taskId: string, tagId: string): Promise<void> => {
      await removeTagFromTask(taskId, tagId);
      await Promise.all([refreshTags(), onRefreshTasks()]);
    },
    [refreshTags, onRefreshTasks],
  );

  return {
    allTags,
    refreshTags,
    addToSession,
    removeFromSession,
    addToNote,
    removeFromNote,
    addToTask,
    removeFromTask,
  };
}
