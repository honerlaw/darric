import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RecordingList } from "./RecordingList";
import type { Session } from "../types";

function makeSession(id: string, topic: string | null): Session {
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
  selectedId: null,
  activeId: null,
  onSelect: (): void => undefined,
};

describe("RecordingList delete confirmation", () => {
  it("labels the row control with an icon rather than a dismiss glyph", () => {
    // `×` reads as "close" or "dismiss". The control destroys the recording and
    // its transcript, so it has to look like it does.
    const { container } = render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    const remove = screen.getByRole("button", { name: "Delete Standup" });
    expect(remove.querySelector("svg")).not.toBeNull();
    expect(remove).toHaveTextContent("");
    expect(container).not.toHaveTextContent("×");
  });

  it("deletes nothing on the first click and names the recording it is about to destroy", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={onDelete}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveTextContent("Standup");
    expect(onDelete).not.toHaveBeenCalled();
  });

  it("deletes the recording once the dialog is confirmed", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup"), makeSession("b", "Retro")]}
        onDelete={onDelete}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Retro" }));
    // Exact match: the row triggers are named "Delete <topic>".
    await user.click(screen.getByRole("button", { name: "Delete" }));

    // The id has to survive the trip through the dialog — confirming must not
    // delete whichever row happens to be first.
    expect(onDelete).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledWith("b");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("deletes nothing when the dialog is cancelled", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={onDelete}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("deletes nothing when the dialog is dismissed with Escape", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={onDelete}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));
    await user.keyboard("{Escape}");

    expect(onDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("closes the dialog if the recording it names disappears", () => {
    // The list is refreshed from the backend. A prompt left standing over a row
    // that no longer exists would confirm a delete against a stale id.
    const { rerender } = render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    screen.getByRole("button", { name: "Delete Standup" }).click();
    rerender(<RecordingList {...BASE_PROPS} sessions={[]} onDelete={(): void => undefined} />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps a click inside the dialog from cancelling it", async () => {
    // The panel sits inside the click-to-dismiss backdrop, so without a stopped
    // propagation, selecting the dialog's own text closes it.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));
    await user.click(screen.getByText(/cannot be undone/));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("falls back to a generic name for an untitled recording", async () => {
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", null)]}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete recording" }));

    expect(screen.getByRole("dialog")).toHaveTextContent("Untitled recording");
  });
});
