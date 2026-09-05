import { afterEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import App from "./App";
import { mockCommands } from "./test/tauri-helpers";

function mockEmptyInstall(downloadState: number | null = null): void {
  mockCommands({
    list_sessions: () => [],
    list_capture_devices: () => [],
    capture_drop_count: () => 0,
    model_download_state: () => downloadState,
  });
}

function mockOneRecording(): void {
  mockCommands({
    model_download_state: () => null,
    list_sessions: () => [
      {
        id: "a",
        topic: "Standup",
        started_at: "2024-01-01T09:00:00Z",
        ended_at: "2024-01-01T09:30:00Z",
        created_at: "2024-01-01T09:00:00Z",
        recorded_minutes: 30,
      },
    ],
    list_capture_devices: () => [],
    capture_drop_count: () => 0,
    get_session_transcript: () => [],
  });
}

describe("App model download visibility", () => {
  it("surfaces the download with no recording selected", async () => {
    mockEmptyInstall();
    render(<App />);

    // The fresh-install case: nothing recorded yet, so nothing is selected and
    // RecorderPane renders its placeholder instead of its body.
    await screen.findByText(/Select a recording/);

    await act(async () => {
      await emit("model_download_start");
      await emit("model_download_progress", 42);
    });

    await waitFor(() => {
      expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "42");
    });
    expect(screen.getByRole("button", { name: /Downloading 42%/ })).toBeDisabled();
    // Still no recording selected — the indicator does not depend on one.
    expect(screen.getByText(/Select a recording/)).toBeInTheDocument();
  });

  it("clears the indicator and re-enables recording once the download finishes", async () => {
    mockEmptyInstall();
    render(<App />);
    await screen.findByText(/Select a recording/);

    await act(async () => {
      await emit("model_download_start");
      await emit("model_download_done");
    });

    await waitFor(() => {
      expect(screen.queryByRole("progressbar")).toBeNull();
    });
    expect(screen.getByRole("button", { name: /Record/ })).toBeEnabled();
  });

  it("releases the UI and reports the reason when the download fails", async () => {
    mockEmptyInstall();
    render(<App />);
    await screen.findByText(/Select a recording/);

    await act(async () => {
      await emit("model_download_start");
      await emit("model_download_progress", 12);
      await emit("model_download_error", "connection reset");
    });

    await waitFor(() => {
      expect(screen.getByText(/connection reset/)).toBeInTheDocument();
    });
    // A failed download must not leave a stalled bar and a dead Record button.
    expect(screen.queryByRole("progressbar")).toBeNull();
    expect(screen.getByRole("button", { name: /Record/ })).toBeEnabled();
  });

  it("withholds Resume while a download is in flight", async () => {
    // Resume reaches the same load_transcriber path as Record. Leaving it live
    // during a download is the second way to start a duplicate download of the
    // same file, and it has no visual counterpart in the pane to notice.
    mockOneRecording();
    render(<App />);

    await act(async () => {
      await emit("model_download_start");
      await emit("model_download_progress", 30);
    });

    const recording = await screen.findByText("Standup");
    fireEvent.click(recording);

    await waitFor(() => {
      expect(screen.getByRole("progressbar")).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: /Resume recording/ })).toBeNull();
  });

  it("offers Resume on a selected recording once no download is in flight", async () => {
    // The positive control for the test above: without it, a change that broke
    // selection would leave that assertion passing for the wrong reason.
    mockOneRecording();
    render(<App />);

    const recording = await screen.findByText("Standup");
    fireEvent.click(recording);

    expect(await screen.findByRole("button", { name: /Resume recording/ })).toBeInTheDocument();
  });

  it("seeds the indicator from backend state when no event was ever received", async () => {
    // The fresh-install path: `ensure_model` runs in Tauri's `setup()` and emits
    // `model_download_start` before the webview holds a listener, so that event
    // and every tick until the next one are lost. Nothing is emitted in this
    // test at all — a purely event-driven UI would show nothing here.
    mockEmptyInstall(37);
    render(<App />);

    expect(await screen.findByRole("progressbar")).toHaveAttribute("aria-valuenow", "37");
    expect(screen.getByRole("button", { name: /Downloading 37%/ })).toBeDisabled();
  });

  it("lets a live event override the seeded value", async () => {
    mockEmptyInstall(37);
    render(<App />);
    await screen.findByRole("progressbar");

    await act(async () => {
      await emit("model_download_progress", 58);
    });

    await waitFor(() => {
      expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "58");
    });
  });
});

const LIVE_SESSION = {
  id: "live",
  topic: "Standup",
  started_at: "2024-01-01T09:00:00Z",
  ended_at: null,
  created_at: "2024-01-01T09:00:00Z",
  recorded_minutes: 0,
};

/**
 * Mock a backend whose `stop_session` hangs until the returned release is
 * called — the real one spends several seconds joining capture threads,
 * flushing segmenters and draining the whisper queue.
 */
const PAST_SESSION = {
  id: "past",
  topic: "Retro",
  started_at: "2024-01-01T08:00:00Z",
  ended_at: "2024-01-01T08:30:00Z",
  created_at: "2024-01-01T08:00:00Z",
  recorded_minutes: 30,
};

function mockPendingStop(): { release: () => void; stopCalls: () => number } {
  let release = (): void => undefined;
  const pending = new Promise<void>((resolve) => {
    release = (): void => {
      resolve();
    };
  });
  let stopCalls = 0;
  mockCommands({
    model_download_state: () => null,
    list_sessions: () => [LIVE_SESSION, PAST_SESSION],
    list_capture_devices: () => [],
    capture_drop_count: () => 0,
    get_session_transcript: () => [],
    start_session: () => "live",
    stop_session: async () => {
      stopCalls += 1;
      await pending;
    },
  });
  return { release, stopCalls: () => stopCalls };
}

describe("App stop feedback", () => {
  afterEach(() => {
    // A test that fails before its own restore would otherwise leak fake timers
    // into every test after it.
    vi.useRealTimers();
  });

  it("reports the stop from the click until the backend finishes", async () => {
    // `App` is the only consumer of Header and RecorderPane, so a prop wired to
    // the wrong expression there is invisible to either component's own tests.
    // This drives the composed tree.
    const { release, stopCalls } = mockPendingStop();
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: /Record/ }));
    const button = await screen.findByRole("button", { name: /Stop$/ });
    expect(await screen.findByText(/Listening/)).toBeInTheDocument();

    fireEvent.click(button);

    // The whole point: this holds while `stop_session` is still running.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Stopping…/ })).toHaveAttribute(
        "aria-disabled",
        "true",
      );
    });
    expect(screen.getByText(/finishing ·/)).toBeInTheDocument();
    expect(screen.getByText(/Finishing transcription…/)).toBeInTheDocument();
    expect(screen.queryByText(/Listening/)).toBeNull();

    // A second press cannot reach the backend. The button keeps focus and still
    // delivers the click, so this genuinely exercises `useSession.stop`'s
    // re-entrancy guard: `stop_session` takes the engine on entry, and a second
    // invocation would return NoSession into the error bar.
    fireEvent.click(screen.getByRole("button", { name: /Stopping…/ }));
    expect(stopCalls()).toBe(1);

    release();

    // `waitFor` re-renders inside act, so the released command's continuations
    // land before these assertions run.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Record/ })).toBeEnabled();
    });
    expect(screen.getByText(/No transcript for this recording/)).toBeInTheDocument();
    expect(screen.queryByText(/finishing ·/)).toBeNull();
  });

  it("keeps the dropped-segment warning when the stop clears the backend count", async () => {
    // `capture_drop_count` reads the engine, and `stop_session` takes it on
    // entry — so a poll landing during the stop reports 0 and would erase the
    // warning. The effect is gated on `isStopping` precisely to stop polling at
    // the click; this is the only chance the user has to read it.
    vi.useFakeTimers();
    let stopped = false;
    let release = (): void => undefined;
    const pending = new Promise<void>((resolve) => {
      release = (): void => {
        resolve();
      };
    });
    mockCommands({
      model_download_state: () => null,
      list_sessions: () => [LIVE_SESSION],
      list_capture_devices: () => [],
      get_session_transcript: () => [],
      start_session: () => "live",
      capture_drop_count: () => (stopped ? 0 : 12),
      stop_session: async () => {
        stopped = true;
        await pending;
      },
    });
    render(<App />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    fireEvent.click(screen.getByRole("button", { name: /Record/ }));
    // The poll's interval is only registered once `start_session` resolves and
    // `isRecording` flips, so let that settle before advancing onto a tick.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2100);
    });
    expect(screen.getByText(/Transcription fell behind/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Stop$/ }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(screen.getByText(/Transcription fell behind/)).toBeInTheDocument();

    await act(async () => {
      release();
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText(/Transcription fell behind/)).toBeInTheDocument();
  });

  it("does not label a past recording with the stopping one's state", async () => {
    // The pane's props carry a viewing-is-active gate for exactly this: clicking
    // away mid-stop must show that recording's own state, not the active one's.
    // Without the gate a finished recording reads "finishing — 00:00 recorded".
    const { release } = mockPendingStop();
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: /Record/ }));
    fireEvent.click(await screen.findByRole("button", { name: /Stop$/ }));
    await screen.findByText(/Finishing transcription…/);

    fireEvent.click(screen.getByText("Retro"));

    await screen.findByText(/No transcript for this recording/);
    expect(screen.queryByText(/Finishing transcription…/)).toBeNull();
    expect(screen.queryByText(/finishing — /)).toBeNull();
    // The chrome still reports the stop — it is app-scoped, not pane-scoped.
    expect(screen.getByText(/finishing ·/)).toBeInTheDocument();

    release();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Record/ })).toBeEnabled();
    });
  });
});
