export interface Tag {
  id: string;
  name: string;
}

export interface Session {
  id: string;
  topic: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
  notes: string;
  recorded_minutes: number;
  tags: Tag[];
}

export interface TranscriptLine {
  id: string;
  session_id: string;
  source: "mic" | "speaker";
  speaker_label?: string;
  content: string;
  recorded_at: string;
}

export interface TranscriptChunk {
  source: "mic" | "speaker";
  speaker_label?: string;
  content: string;
  recorded_at: string;
}

export interface Note {
  id: string;
  title: string;
  body: string;
  created_at: string;
  updated_at: string;
  tags: Tag[];
}

export type TaskColumn = "todo" | "doing" | "done";

export interface Task {
  id: string;
  title: string;
  col: TaskColumn;
  position: number;
  created_at: string;
  updated_at: string;
  tags: Tag[];
}

export type TimelineEntryKind = "note" | "meeting" | "task" | "you-asked-darric" | "darric";

export interface TimelineEntry {
  id: string;
  kind: TimelineEntryKind;
  timestamp: string;
  title?: string | undefined;
  body?: string | undefined;
  refId?: string | undefined;
  durationMin?: number | undefined;
  isStreaming?: boolean | undefined;
  tags?: Tag[] | undefined;
}

export interface ChatMessageRow {
  id: string;
  role: "user" | "assistant";
  content: string;
  created_at: string;
}

export interface McpServerStatus {
  enabled: boolean;
  configured_port: number;
  listening: boolean;
  bound_port: number | null;
  url: string | null;
}

export type Screen = "timeline" | "meeting" | "board";

export interface SearchResultSession {
  id: string;
  topic: string | null;
  started_at: string;
  snippet: string | null;
  tags: Tag[];
}

export interface SearchResultNote {
  id: string;
  title: string;
  snippet: string | null;
  tags: Tag[];
}

export interface SearchResultTask {
  id: string;
  title: string;
  col: string;
  tags: Tag[];
}

export interface SearchResults {
  sessions: SearchResultSession[];
  notes: SearchResultNote[];
  tasks: SearchResultTask[];
}
