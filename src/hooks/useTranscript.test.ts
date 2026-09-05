import { describe, expect, it } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import { useTranscript } from "./useTranscript";
import { mockCommands } from "../test/tauri-helpers";
import type { InvokeArgs } from "@tauri-apps/api/core";

function chunk(sessionId: string, content: string): Record<string, string> {
  return {
    session_id: sessionId,
    device_id: "d1",
    device_name: "MacBook Microphone",
    direction: "input",
    content,
    recorded_at: "2024-01-01T09:00:05Z",
  };
}

/**
 * Let the effect's `listen()` round-trip finish. Registration is async, so an
 * `emit` issued before it resolves reaches no handler.
 */
async function listenersReady(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

describe("useTranscript session scoping", () => {
  it("drops a late flush line for a session the user has clicked away from", async () => {
    // Whisper transcribes the final segment after the recording stops, so the
    // listener outlives the recording by FLUSH_LINGER_MS. A chunk arriving in
    // that window belongs to the session that stopped — not to whatever is on
    // screen by the time it lands.
    // The backend persists a line before it emits it, so A's flush line exists in
    // the database from the moment it is broadcast. Reselecting A must show it —
    // withholding it from B's pane loses nothing.
    let flushed = false;
    mockCommands({
      get_session_transcript: (payload?: InvokeArgs) =>
        flushed && (payload as { sessionId: string } | undefined)?.sessionId === "A"
          ? [
              {
                id: "persisted",
                session_id: "A",
                device_id: "d1",
                device_name: "MacBook Microphone",
                direction: "input",
                content: "A's trailing flush line",
                recorded_at: "2024-01-01T09:00:05Z",
              },
            ]
          : [],
    });
    const { result, rerender } = renderHook(
      ({ id, live }: { id: string; live: boolean }) => useTranscript(id, live),
      { initialProps: { id: "A", live: true } },
    );
    await listenersReady();

    await act(async () => {
      await emit("transcript_chunk", chunk("A", "spoken during A"));
    });
    await waitFor(() => {
      expect(result.current).toHaveLength(1);
    });

    // The recording stops and the user selects B while the flush is still running.
    rerender({ id: "B", live: false });
    await waitFor(() => {
      expect(result.current).toHaveLength(0);
    });
    await listenersReady();

    flushed = true;
    await act(async () => {
      await emit("transcript_chunk", chunk("A", "A's trailing flush line"));
    });

    expect(result.current).toHaveLength(0);

    // Reselecting A shows the line the filter withheld from B — it was persisted
    // before it was emitted, so nothing was lost by not displaying it there.
    rerender({ id: "A", live: false });
    await waitFor(() => {
      expect(result.current).toHaveLength(1);
    });
    expect(result.current[0]?.content).toBe("A's trailing flush line");
  });

  it("still appends a late flush line for the session being displayed", async () => {
    // The positive control: the linger window exists to catch exactly this, so a
    // filter that dropped everything would satisfy the test above for free.
    mockCommands({ get_session_transcript: () => [] });
    const { result, rerender } = renderHook(
      ({ id, live }: { id: string; live: boolean }) => useTranscript(id, live),
      { initialProps: { id: "A", live: true } },
    );
    await listenersReady();

    rerender({ id: "A", live: false });
    await listenersReady();

    await act(async () => {
      await emit("transcript_chunk", chunk("A", "A's trailing flush line"));
    });

    await waitFor(() => {
      expect(result.current).toHaveLength(1);
    });
    expect(result.current[0]?.content).toBe("A's trailing flush line");
    expect(result.current[0]?.session_id).toBe("A");
  });
});
