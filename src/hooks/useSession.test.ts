import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useSession } from "./useSession";
import { mockCommands } from "../test/tauri-helpers";

/** A `stop_session` that hangs until released, as the real one effectively does. */
function pendingStop(): { release: () => void; calls: () => number } {
  let release = (): void => undefined;
  const pending = new Promise<void>((resolve) => {
    release = (): void => {
      resolve();
    };
  });
  let calls = 0;
  mockCommands({
    model_download_state: () => null,
    list_sessions: () => [],
    start_session: () => "live",
    stop_session: async () => {
      calls += 1;
      await pending;
    },
  });
  return { release, calls: () => calls };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("useSession stop", () => {
  it("rejects a re-entrant stop rather than invoking the command twice", async () => {
    // `stop_session` takes the engine out of app state on entry, so a second
    // concurrent call finds none and fails with NoSession. The Stop button's
    // `disabled` covers the UI, but the invariant belongs to the hook — this is
    // what holds if any other caller is ever added.
    const { release, calls } = pendingStop();
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    expect(result.current.isRecording).toBe(true);

    await act(async () => {
      const first = result.current.stop();
      const second = result.current.stop();
      release();
      await Promise.all([first, second]);
    });

    expect(calls()).toBe(1);
    expect(result.current.isRecording).toBe(false);
    expect(result.current.isStopping).toBe(false);
  });

  it("stops a second recording after the first one finished stopping", async () => {
    // The re-entrancy guard is a ref, so a missed reset is invisible until the
    // *next* stop: it early-returns forever and the Stop button silently does
    // nothing for the rest of the app's life, with no error anywhere.
    const { release, calls } = pendingStop();
    const { result } = renderHook(() => useSession());

    for (const round of [1, 2]) {
      await act(async () => {
        await result.current.start();
      });
      await act(async () => {
        const stopping = result.current.stop();
        release();
        await stopping;
      });
      expect(calls()).toBe(round);
    }

    expect(result.current.isRecording).toBe(false);
  });

  it("clears the stopping state when the backend fails", async () => {
    // Without clearing on the failure path the button stays a disabled
    // "Stopping…" for the rest of the session, with no way to stop or record.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      start_session: () => "live",
      stop_session: () => {
        throw new Error("no active session");
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      await result.current.stop();
    });

    expect(result.current.isStopping).toBe(false);
    expect(result.current.isRecording).toBe(false);
    expect(result.current.error).toMatch(/no active session/);
  });

  it("freezes the elapsed counter on the click, not when the backend returns", async () => {
    // Capture has already ended by the time the command resolves — the seconds
    // it spends there are flush and transcription. A clock still climbing
    // through them claims audio is being recorded that is not.
    vi.useFakeTimers();
    const { release } = pendingStop();
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(result.current.elapsedSeconds).toBe(3);

    act(() => {
      void result.current.stop();
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(result.current.isStopping).toBe(true);
    expect(result.current.elapsedSeconds).toBe(3);

    // `waitFor` polls on real timers, which never advance here — release the
    // command and flush its continuations through the fake clock instead.
    await act(async () => {
      release();
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(result.current.isStopping).toBe(false);
    expect(result.current.elapsedSeconds).toBe(3);
  });
});
