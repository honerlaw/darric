export interface Session {
  id: string;
  topic: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
  recorded_minutes: number;
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

/** A capture device the recorder can draw audio from. */
export interface CaptureDevice {
  id: string;
  name: string;
  direction: "input" | "output";
  enabled: boolean;
}
