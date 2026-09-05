import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Session, TranscriptChunk, TranscriptLine } from "../types";

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
export const resumeSession = async (id: string): Promise<string> =>
  invoke<string>("resume_session", { id });

// Settings
export const saveSetting = async (key: string, value: string): Promise<void> => {
  await invoke("save_setting", { key, value });
};
export const getSetting = async (key: string): Promise<string | null> =>
  invoke<string | null>("get_setting", { key });

// Events
export const onTranscriptChunk = async (
  handler: (chunk: TranscriptChunk) => void,
): Promise<UnlistenFn> =>
  listen<TranscriptChunk>("transcript_chunk", (e) => {
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
