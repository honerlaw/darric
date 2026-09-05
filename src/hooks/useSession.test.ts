import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
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

const ONE_SESSION = [
  {
    id: "a",
    topic: "Standup",
    started_at: "2024-01-01T09:00:00Z",
    ended_at: "2024-01-01T09:30:00Z",
    created_at: "2024-01-01T09:00:00Z",
    recorded_minutes: 30,
  },
];

describe("useSession remove", () => {
  it("surfaces a failed delete instead of swallowing it", async () => {
    // `remove` was the only command besides `update` with no catch, so a
    // rejection reached nothing that displays it — and the confirmation modal
    // has just promised the user the recording is gone.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => ONE_SESSION,
      delete_session: () => {
        throw new Error("database is locked");
      },
    });
    const { result } = renderHook(() => useSession());

    let deleted: boolean | undefined;
    await act(async () => {
      deleted = await result.current.remove("a");
    });

    expect(deleted).toBe(false);
    expect(result.current.error).toMatch(/database is locked/);
  });

  it("reports success so a caller can gate its optimistic update", async () => {
    // The positive control: without it, a `remove` that always returned false
    // would satisfy the assertion above.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      delete_session: () => undefined,
    });
    const { result } = renderHook(() => useSession());

    let deleted: boolean | undefined;
    await act(async () => {
      deleted = await result.current.remove("a");
    });

    expect(deleted).toBe(true);
    expect(result.current.error).toBeNull();
  });

  it("clears the active recording only when it is the one deleted", async () => {
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      start_session: () => "live",
      delete_session: () => undefined,
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    expect(result.current.activeSessionId).toBe("live");

    await act(async () => {
      await result.current.remove("some-other-recording");
    });
    expect(result.current.activeSessionId).toBe("live");

    await act(async () => {
      await result.current.remove("live");
    });
    expect(result.current.activeSessionId).toBeNull();
  });

  it("leaves another subsystem's error alone when a delete succeeds", async () => {
    // `error` is one shared slot with no provenance. The model-download failure
    // is the only notice the user gets — App never renders `modelReady`, and the
    // Record button stays enabled — so an unrelated successful delete erasing it
    // is permanent.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      delete_session: () => undefined,
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await emit("model_download_error", "disk full");
    });
    expect(result.current.error).toMatch(/disk full/);

    await act(async () => {
      await result.current.remove("a");
    });

    expect(result.current.error).toMatch(/disk full/);
  });

  it("clears its own failure but not one that landed on top of it", async () => {
    // The narrow case the ref exists for: a delete fails, something else then
    // writes the shared slot, and the delete's retry must not take that with it.
    let fail = true;
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      delete_session: () => {
        if (fail) throw new Error("database is locked");
        return undefined;
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.remove("a");
    });
    await act(async () => {
      await emit("model_download_error", "disk full");
    });

    fail = false;
    await act(async () => {
      await result.current.remove("a");
    });

    expect(result.current.error).toMatch(/disk full/);
  });

  it("does not clear the active recording set by a start that overtook the delete", async () => {
    // `remove` used to read `activeSessionId` out of its closure. Stop A (the id
    // deliberately outlives the recording), start deleting A, and press Record
    // again before it resolves: the stale closure compares its captured "A"
    // against the id being deleted, matches, and clears the id of the recording
    // that is now running — killing the sidebar's live dot and every
    // `viewingSessionId === activeSessionId` gate in RecorderPane.
    let release = (): void => undefined;
    const pending = new Promise<void>((resolve) => {
      release = (): void => {
        resolve();
      };
    });
    let started = 0;
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [],
      start_session: () => {
        started += 1;
        return started === 1 ? "old" : "new";
      },
      stop_session: () => undefined,
      delete_session: async () => {
        await pending;
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      await result.current.stop();
    });
    expect(result.current.activeSessionId).toBe("old");

    await act(async () => {
      const removing = result.current.remove("old");
      await result.current.start();
      release();
      await removing;
    });

    expect(result.current.activeSessionId).toBe("new");
  });

  it("applies nothing when the delete lands but the refresh fails", async () => {
    // `remove` answers "the recording may still be there" on this path, and the
    // list it would have refreshed still holds the row. Clearing the active id
    // anyway half-applies the delete behind that answer: the sidebar still shows
    // a resumable-looking row whose session is gone.
    let listFails = false;
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => {
        if (listFails) throw new Error("cannot read sessions");
        return [];
      },
      start_session: () => "live",
      stop_session: () => undefined,
      delete_session: () => {
        listFails = true;
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      await result.current.stop();
    });
    expect(result.current.activeSessionId).toBe("live");

    let deleted: boolean | undefined;
    await act(async () => {
      deleted = await result.current.remove("live");
    });

    expect(deleted).toBe(false);
    expect(result.current.error).toMatch(/cannot read sessions/);
    // The two halves must agree: false means nothing was applied.
    expect(result.current.activeSessionId).toBe("live");
  });

  it("does not leave a previous failure's message on a later success", async () => {
    let fail = true;
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => ONE_SESSION,
      delete_session: () => {
        if (fail) throw new Error("database is locked");
        return undefined;
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.remove("a");
    });
    expect(result.current.error).toMatch(/database is locked/);

    fail = false;
    await act(async () => {
      await result.current.remove("a");
    });
    expect(result.current.error).toBeNull();
  });
});

describe("useSession update", () => {
  it("surfaces a failed rename instead of rejecting unhandled", async () => {
    // Lower stakes than a delete — an unchanged title is its own feedback — but
    // it is the other half of the same gap, and fixing one path while leaving
    // the other open is a shape this project has already recorded.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => ONE_SESSION,
      update_session: () => {
        throw new Error("no such session");
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.update("a", "Retro");
    });

    expect(result.current.error).toMatch(/no such session/);
  });

  it("clears a failed rename's message once the rename succeeds", async () => {
    let fail = true;
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => ONE_SESSION,
      update_session: () => {
        if (fail) throw new Error("no such session");
        return undefined;
      },
    });
    const { result } = renderHook(() => useSession());

    await act(async () => {
      await result.current.update("a", "Retro");
    });
    expect(result.current.error).toMatch(/no such session/);

    fail = false;
    await act(async () => {
      await result.current.update("a", "Retro");
    });

    expect(result.current.error).toBeNull();
  });
});

describe("useSession initial load", () => {
  it("reports an unreadable session list instead of showing an empty sidebar", async () => {
    // The mount effect was the one `refresh` call site with nobody above it to
    // catch, so a failed `list_sessions` at startup was an unhandled rejection
    // and the user got an empty sidebar with no explanation.
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => {
        throw new Error("cannot read sessions");
      },
    });

    const { result } = renderHook(() => useSession());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.error).toMatch(/cannot read sessions/);
    expect(result.current.sessions).toEqual([]);
  });
});
