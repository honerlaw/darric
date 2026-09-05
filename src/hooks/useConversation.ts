import { useCallback, useEffect, useRef, useState } from "react";
import {
  getChatHistory,
  onChatDelta,
  onChatDone,
  onChatError,
  sendChatMessage,
} from "../lib/tauri";
import type { TimelineEntry } from "../types";

interface UseConversationReturn {
  agentEntries: TimelineEntry[];
  isLoading: boolean;
  submit: (prompt: string) => void;
}

function rowsToEntries(
  rows: { id: string; role: string; content: string; created_at: string }[],
): TimelineEntry[] {
  return rows.map((row) =>
    row.role === "user"
      ? {
          id: `user-${row.id}`,
          kind: "you-asked-darric" as const,
          timestamp: row.created_at,
          title: row.content,
        }
      : {
          id: `asst-${row.id}`,
          kind: "darric" as const,
          timestamp: row.created_at,
          body: row.content,
        },
  );
}

export function useConversation(): UseConversationReturn {
  const [agentEntries, setAgentEntries] = useState<TimelineEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const streamingIdRef = useRef<string | null>(null);

  // Load persisted history on mount
  useEffect(() => {
    let cancelled = false;
    getChatHistory()
      .then((rows) => {
        if (!cancelled) setAgentEntries(rowsToEntries(rows));
      })
      .catch((err: unknown) => {
        console.error("[chat] failed to load history:", err);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Register streaming event listeners once
  useEffect(() => {
    const state: { cancelled: boolean } = { cancelled: false };
    const unlisteners: (() => void)[] = [];

    const register = async (): Promise<void> => {
      const unDelta = await onChatDelta((delta) => {
        const id = streamingIdRef.current;
        if (id === null) return;
        setAgentEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, body: (e.body ?? "") + delta } : e)),
        );
      });
      const unDone = await onChatDone(() => {
        const id = streamingIdRef.current;
        if (id !== null) {
          setAgentEntries((prev) =>
            prev.map((e) => (e.id === id ? { ...e, isStreaming: false } : e)),
          );
          streamingIdRef.current = null;
        }
        setIsLoading(false);
      });
      const unError = await onChatError((msg) => {
        const id = streamingIdRef.current;
        if (id !== null) {
          setAgentEntries((prev) =>
            prev.map((e) =>
              e.id === id ? { ...e, body: `Error: ${msg}`, isStreaming: false } : e,
            ),
          );
          streamingIdRef.current = null;
        }
        setIsLoading(false);
      });

      // Push first so the cancel check below (or the cleanup closure) can call them.
      // Cleanup clears the array after calling, preventing double-calls if it ran
      // while we were awaiting and we reach this block afterwards.
      unlisteners.push(unDelta, unDone, unError);
      if (state.cancelled) {
        for (const fn of unlisteners) fn();
        unlisteners.length = 0;
      }
    };

    void register().catch((err: unknown) => {
      console.error("[chat] failed to register listeners:", err);
    });

    return () => {
      state.cancelled = true;
      for (const fn of unlisteners) fn();
      unlisteners.length = 0;
    };
  }, []);

  const submit = useCallback((prompt: string): void => {
    const now = new Date().toISOString();
    const queryId = crypto.randomUUID();
    const responseId = `streaming-${crypto.randomUUID()}`;

    streamingIdRef.current = responseId;
    setIsLoading(true);

    setAgentEntries((prev) => [
      ...prev,
      { id: queryId, kind: "you-asked-darric", timestamp: now, title: prompt },
      { id: responseId, kind: "darric", timestamp: now, body: "", isStreaming: true },
    ]);

    sendChatMessage(prompt).catch((err: unknown) => {
      console.error("[chat] send failed:", err);
      setAgentEntries((prev) =>
        prev.map((e) =>
          e.id === responseId
            ? {
                ...e,
                body: "Failed to send. Check your API key in Settings.",
                isStreaming: false,
              }
            : e,
        ),
      );
      streamingIdRef.current = null;
      setIsLoading(false);
    });
  }, []);

  return { agentEntries, isLoading, submit };
}
