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
  isRecording: false,
  isStarting: false,
  canResume: true,
  elapsedSeconds: 0,
  downloadProgress: null,
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
