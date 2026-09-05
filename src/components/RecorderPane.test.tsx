import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RecorderPane } from "./RecorderPane";
import type { Session } from "../types";

function makeSession(id: string, topic: string): Session {
  return {
    id,
    topic,
    started_at: "2024-01-01T09:00:00Z",
    ended_at: "2024-01-01T09:30:00Z",
    created_at: "2024-01-01T09:00:00Z",
    recorded_minutes: 30,
  };
}

const BASE_PROPS = {
  transcriptLines: [],
  devices: [],
  onToggleDevice: (): void => undefined,
  droppedSegments: 0,
  isRecording: false,
  isStarting: false,
  isStopping: false,
  canResume: true,
  elapsedSeconds: 0,
  onResume: (): void => undefined,
};

describe("RecorderPane title editing", () => {
  it("abandons an in-progress title edit when the selection moves to another recording", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    const sessionA = makeSession("a", "Session A");
    const sessionB = makeSession("b", "Session B");

    const { rerender } = render(
      <RecorderPane {...BASE_PROPS} session={sessionA} onRename={onRename} />,
    );

    await user.click(screen.getByText("Session A"));
    const input = screen.getByRole("textbox");
    await user.clear(input);
    await user.type(input, "renamed A");

    // Select a different recording without committing the edit.
    rerender(<RecorderPane {...BASE_PROPS} session={sessionB} onRename={onRename} />);

    // B's title must render as a heading, not as an editor holding A's draft.
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.getByText("Session B")).toBeInTheDocument();
    expect(onRename).not.toHaveBeenCalledWith("renamed A");
  });

  it("commits a rename on Enter for the recording being edited", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    const sessionA = makeSession("a", "Session A");

    render(<RecorderPane {...BASE_PROPS} session={sessionA} onRename={onRename} />);

    await user.click(screen.getByText("Session A"));
    const input = screen.getByRole("textbox");
    await user.clear(input);
    await user.type(input, "renamed A{Enter}");

    expect(onRename).toHaveBeenCalledWith("renamed A");
  });
});

describe("RecorderPane stopping state", () => {
  it("reports the recording as finishing rather than still listening", () => {
    // `isRecording` stays true across the stop, so without the `isStopping`
    // check the pane keeps inviting speech into a recording that has ended.
    render(
      <RecorderPane
        {...BASE_PROPS}
        session={makeSession("a", "Standup")}
        onRename={(): void => undefined}
        isRecording
        isStopping
        elapsedSeconds={65}
      />,
    );

    expect(screen.getByText(/Finishing transcription…/)).toBeInTheDocument();
    expect(screen.queryByText(/Listening/)).toBeNull();
    expect(screen.getByText(/finishing — 01:05 recorded/)).toBeInTheDocument();
  });

  it("still invites speech while the recording is genuinely live", () => {
    // The positive control: without it the assertion above would pass for a pane
    // that had simply stopped rendering the listening message at all.
    render(
      <RecorderPane
        {...BASE_PROPS}
        session={makeSession("a", "Standup")}
        onRename={(): void => undefined}
        isRecording
        elapsedSeconds={65}
      />,
    );

    expect(screen.getByText(/Listening/)).toBeInTheDocument();
    expect(screen.getByText(/01:05 elapsed/)).toBeInTheDocument();
  });
});
