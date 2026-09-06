export interface Session {
  id: string;
  topic: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
  recorded_minutes: number;
}

export type Direction = "input" | "output";

export interface TranscriptLine {
  /**
   * The backend's SQLite rowid, the MCP server's paging cursor. Null on a line
   * appended live from a `transcript_chunk`, which is not read back from the
   * database until the transcript reloads.
   */
  seq: number | null;
  id: string;
  session_id: string;
  device_id: string;
  device_name: string;
  direction: Direction;
  content: string;
  recorded_at: string;
}

export interface TranscriptChunk {
  session_id: string;
  device_id: string;
  device_name: string;
  direction: Direction;
  content: string;
  recorded_at: string;
}

/** Live capture state of one source. Mirrors `SourceState` in the backend. */
export type DeviceState = "idle" | "starting" | "active" | "retrying" | "failed";

export interface CaptureDevice {
  id: string;
  name: string;
  direction: Direction;
  enabled: boolean;
  state: DeviceState;
  /** RMS level in [0, 1], for the meter. */
  level: number;
}

/** What the in-app MCP server reports about itself. */
export interface McpServerStatus {
  listening: boolean;
  /** The bound port, or the port that could not be bound. */
  port: number;
  url: string | null;
  /** Why the server is not listening, when it is not. */
  error: string | null;
}
