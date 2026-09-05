import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import App from "./App";
import { mockCommands } from "./test/tauri-helpers";

function mockEmptyInstall(): void {
  mockCommands({
    list_sessions: () => [],
    list_capture_devices: () => [],
    capture_drop_count: () => 0,
  });
}

function mockOneRecording(): void {
  mockCommands({
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
});
