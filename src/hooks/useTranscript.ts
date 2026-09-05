import { useEffect, useRef, useState } from "react";
import { getSessionTranscript, onTranscriptChunk } from "../lib/tauri";
import type { TranscriptChunk, TranscriptLine } from "../types";

// Whisper processes the flush segment asynchronously after stop. Keep the
// listener alive this long after recording ends so the chunk still lands.
const FLUSH_LINGER_MS = 20_000;

export function useTranscript(sessionId: string | null, isLive: boolean): TranscriptLine[] {
  const [lines, setLines] = useState<TranscriptLine[]>([]);
  const sessionRef = useRef(sessionId);
  const prevIsLiveRef = useRef(false);

  useEffect(() => {
    sessionRef.current = sessionId;
    setLines([]);
    if (sessionId === null) return;
    void getSessionTranscript(sessionId).then(setLines);
  }, [sessionId]);

  useEffect(() => {
    const wasLive = prevIsLiveRef.current;
    prevIsLiveRef.current = isLive;

    const appendChunk = (chunk: TranscriptChunk): void => {
      // The chunk says which session it belongs to, so a late flush line for a
      // recording the user has clicked away from is dropped rather than appended
      // under whatever is now selected. Capturing the id when the listener is
      // attached would not work: the `[sessionId]` effect above is declared
      // first, so on the render where the selection changes `sessionRef.current`
      // is already the new session by the time this effect re-runs.
      if (chunk.session_id !== sessionRef.current) return;
      setLines((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          session_id: chunk.session_id,
          device_id: chunk.device_id,
          device_name: chunk.device_name,
          direction: chunk.direction,
          content: chunk.content,
          recorded_at: chunk.recorded_at,
        },
      ]);
    };

    if (isLive) {
      const unsub = onTranscriptChunk(appendChunk);
      return () => {
        void unsub.then((fn) => {
          fn();
        });
      };
    }

    if (wasLive) {
      // Recording just stopped. Keep listening for FLUSH_LINGER_MS so the
      // final partial segment (transcribed async by whisper) still appears.
      const unsub = onTranscriptChunk(appendChunk);
      const timer = setTimeout(() => {
        void unsub.then((fn) => {
          fn();
        });
      }, FLUSH_LINGER_MS);
      return () => {
        clearTimeout(timer);
        void unsub.then((fn) => {
          fn();
        });
      };
    }
  }, [isLive]);

  return lines;
}
