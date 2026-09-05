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
  id: string;
  session_id: string;
  device_id: string;
  device_name: string;
  direction: Direction;
  content: string;
  recorded_at: string;
}

export interface TranscriptChunk {
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
