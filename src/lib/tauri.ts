import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ChatMessageRow,
  McpServerStatus,
  Note,
  SearchResults,
  Session,
  Tag,
  Task,
  TaskColumn,
  TranscriptChunk,
  TranscriptLine,
} from "../types";

// Sessions
export const startSession = async (topic?: string): Promise<string> =>
  invoke<string>("start_session", { topic });
export const stopSession = async (): Promise<void> => {
  await invoke("stop_session");
};
export const listSessions = async (): Promise<Session[]> => invoke<Session[]>("list_sessions");
export const getSessionTranscript = async (sessionId: string): Promise<TranscriptLine[]> =>
  invoke<TranscriptLine[]>("get_session_transcript", { sessionId });
export const deleteSession = async (id: string): Promise<void> => {
  await invoke("delete_session", { id });
};
export const updateSession = async (id: string, topic?: string): Promise<Session> =>
  invoke<Session>("update_session", { id, topic });
export const updateSessionNotes = async (id: string, notes: string): Promise<void> => {
  await invoke("update_session_notes", { id, notes });
};
export const resumeSession = async (id: string): Promise<string> =>
  invoke<string>("resume_session", { id });

// Settings
export const saveSetting = async (key: string, value: string): Promise<void> => {
  await invoke("save_setting", { key, value });
};
export const getSetting = async (key: string): Promise<string | null> =>
  invoke<string | null>("get_setting", { key });
export const listMcpServers = async (): Promise<string[]> => invoke<string[]>("list_mcp_servers");
export const mcpServerStatus = async (): Promise<McpServerStatus> =>
  invoke<McpServerStatus>("mcp_server_status");

// Events
export const onTranscriptChunk = async (
  handler: (chunk: TranscriptChunk) => void,
): Promise<UnlistenFn> =>
  listen<TranscriptChunk>("transcript_chunk", (e) => {
    handler(e.payload);
  });

export const onWhisperUnavailable = async (
  handler: (reason: string) => void,
): Promise<UnlistenFn> =>
  listen<string>("whisper_unavailable", (e) => {
    handler(e.payload);
  });

export const onModelDownloadStart = async (handler: () => void): Promise<UnlistenFn> =>
  listen("model_download_start", () => {
    handler();
  });

export const onModelDownloadProgress = async (
  handler: (pct: number) => void,
): Promise<UnlistenFn> =>
  listen<number>("model_download_progress", (e) => {
    handler(e.payload);
  });

export const onModelDownloadDone = async (handler: () => void): Promise<UnlistenFn> =>
  listen("model_download_done", () => {
    handler();
  });

export const onModelReady = async (handler: () => void): Promise<UnlistenFn> =>
  listen("model_ready", () => {
    handler();
  });

// Notes
export const listNotes = async (): Promise<Note[]> => invoke<Note[]>("list_notes");
export const getNote = async (id: string): Promise<Note> => invoke<Note>("get_note", { id });
export const createNote = async (title: string, body: string): Promise<Note> =>
  invoke<Note>("create_note", { title, body });
export const updateNote = async (id: string, title: string, body: string): Promise<Note> =>
  invoke<Note>("update_note", { id, title, body });
export const deleteNote = async (id: string): Promise<void> => {
  await invoke("delete_note", { id });
};

// Tasks
export const listTasks = async (): Promise<Task[]> => invoke<Task[]>("list_tasks");
export const createTask = async (title: string, col: TaskColumn): Promise<Task> =>
  invoke<Task>("create_task", { title, col });
export const updateTask = async (
  id: string,
  updates: { title?: string; col?: TaskColumn; position?: number },
): Promise<Task> => invoke<Task>("update_task", { id, ...updates });
export const deleteTask = async (id: string): Promise<void> => {
  await invoke("delete_task", { id });
};

// Chat / AI
export const sendChatMessage = async (prompt: string): Promise<void> => {
  await invoke("send_chat_message", { prompt });
};
export const getChatHistory = async (): Promise<ChatMessageRow[]> =>
  invoke<ChatMessageRow[]>("get_chat_history");
export const clearChatHistory = async (): Promise<void> => {
  await invoke("clear_chat_history");
};

export const onChatDelta = async (handler: (delta: string) => void): Promise<UnlistenFn> =>
  listen<string>("chat_delta", (e) => {
    handler(e.payload);
  });

export const onChatDone = async (handler: () => void): Promise<UnlistenFn> =>
  listen("chat_done", () => {
    handler();
  });

export const onChatError = async (handler: (msg: string) => void): Promise<UnlistenFn> =>
  listen<string>("chat_error", (e) => {
    handler(e.payload);
  });

// Search
export const searchAll = async (query: string): Promise<SearchResults> =>
  invoke<SearchResults>("search_all", { query });

// Tags
export const listTags = async (): Promise<Tag[]> => invoke<Tag[]>("list_tags");
export const addTagToSession = async (sessionId: string, tagName: string): Promise<Tag[]> =>
  invoke<Tag[]>("add_tag_to_session", { sessionId, tagName });
export const removeTagFromSession = async (sessionId: string, tagId: string): Promise<Tag[]> =>
  invoke<Tag[]>("remove_tag_from_session", { sessionId, tagId });
export const addTagToNote = async (noteId: string, tagName: string): Promise<Tag[]> =>
  invoke<Tag[]>("add_tag_to_note", { noteId, tagName });
export const removeTagFromNote = async (noteId: string, tagId: string): Promise<Tag[]> =>
  invoke<Tag[]>("remove_tag_from_note", { noteId, tagId });
export const addTagToTask = async (taskId: string, tagName: string): Promise<Tag[]> =>
  invoke<Tag[]>("add_tag_to_task", { taskId, tagName });
export const removeTagFromTask = async (taskId: string, tagId: string): Promise<Tag[]> =>
  invoke<Tag[]>("remove_tag_from_task", { taskId, tagId });
