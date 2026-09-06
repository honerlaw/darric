import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
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

/** The dismiss-on-press layer the dialog panel sits inside. */
function backdropOf(dialog: HTMLElement): HTMLElement {
  const backdrop = dialog.parentElement;
  if (backdrop === null) throw new Error("the dialog panel has no backdrop");
  return backdrop;
}

const BASE_PROPS = {
  selectedId: null,
  activeId: null,
  onSelect: (): void => undefined,
  onRename: (): void => undefined,
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
    // The control is hover-revealed, so without this a keyboard user tabs onto
    // an invisible button. jsdom evaluates no stylesheet, so asserting the class
    // is the strongest check available here — and this repo has already shipped
    // a rewrite that carried the markup across and dropped a guard like it.
    expect(remove).toHaveClass("focus-visible:opacity-100");
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
    expect(dialog).toHaveAccessibleName(/Standup/);
    // The consequence is the reason the dialog exists, so it has to be part of
    // what a screen reader announces — not just text that happens to be inside.
    expect(dialog).toHaveAccessibleDescription(/cannot be undone/);
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

    fireEvent.click(screen.getByRole("button", { name: "Delete Standup" }));
    rerender(<RecordingList {...BASE_PROPS} sessions={[]} onDelete={(): void => undefined} />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps a click inside the dialog from cancelling it", async () => {
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

  it("survives a text selection that is released over the backdrop", async () => {
    // A click's target is the common ancestor of its press and its release, so
    // pressing on the dialog's body text and releasing over the dim area reports
    // the *backdrop* — a click-based dismissal closes the dialog mid-selection,
    // and containment inside the panel cannot see it. Dismissal is keyed to the
    // press instead.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));
    const backdrop = backdropOf(screen.getByRole("dialog"));

    fireEvent.mouseDown(screen.getByText(/cannot be undone/));
    fireEvent.mouseUp(backdrop);
    fireEvent.click(backdrop);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("still dismisses on a press that starts on the backdrop", () => {
    // The positive control: without it the test above passes for a dialog that
    // cannot be dismissed by clicking away at all.
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Delete Standup" }));
    fireEvent.mouseDown(backdropOf(screen.getByRole("dialog")));

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps Tab inside the dialog it declares modal", async () => {
    // `aria-modal="true"` promises assistive technology the rest of the page is
    // inert, and nothing enforces that. Without containment, three Tabs from
    // Confirm reach the Record button behind the backdrop — where Enter starts a
    // recording under a modal the user cannot see past.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup"), makeSession("b", "Retro")]}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Standup" }));
    const dialog = screen.getByRole("dialog");

    for (let i = 0; i < 6; i++) {
      await user.tab();
      expect(dialog.contains(document.activeElement)).toBe(true);
    }
    await user.tab({ shift: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it("returns focus to the control that opened it", async () => {
    // A keyboard user who tabs down a long sidebar to reach a delete trigger and
    // then cancels must not be dropped on <body> and made to traverse it again.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    const trigger = screen.getByRole("button", { name: "Delete Standup" });
    await user.click(trigger);
    await user.keyboard("{Escape}");

    expect(document.activeElement).toBe(trigger);
  });

  it("names an untitled recording the same way in the row, the label and the prompt", async () => {
    // One naming answer. The trigger used to say "Delete recording" while the
    // row and the prompt said "Untitled recording" — and for a topic of "" the
    // trigger's name collapsed to a bare "Delete", colliding with the dialog's
    // own confirm button.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "")]}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete Untitled recording" }));

    expect(screen.getByRole("dialog")).toHaveTextContent("Untitled recording");
  });
});

describe("RecordingList inline rename", () => {
  it("opens an editor prefilled with the topic on a double-click, not a single click", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onSelect={onSelect}
        onDelete={(): void => undefined}
      />,
    );

    await user.click(screen.getByText("Standup"));
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(onSelect).toHaveBeenCalledWith("a");

    await user.dblClick(screen.getByText("Standup"));
    const input = screen.getByRole("textbox", { name: "Rename Standup" });
    expect(input).toHaveValue("Standup");
    expect(input).toHaveFocus();
  });

  it("starts from an empty draft for an untitled recording", async () => {
    // The row shows "Untitled recording" for a null topic. Seeding the editor
    // with that placeholder would let a stray Enter store it as a real name.
    const user = userEvent.setup();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", null)]}
        onDelete={(): void => undefined}
      />,
    );

    await user.dblClick(screen.getByText("Untitled recording"));

    expect(screen.getByRole("textbox", { name: "Rename Untitled recording" })).toHaveValue("");
  });

  it("commits a new name once on Enter and closes the editor", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup"), makeSession("b", "Retro")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Retro"));
    await user.keyboard("  Retrospective {Enter}");

    // Exactly once: Enter closes the editor by blurring it, and the blur is the
    // only commit path, so the unmount cannot commit the same draft again.
    expect(onRename).toHaveBeenCalledTimes(1);
    expect(onRename).toHaveBeenCalledWith("b", "Retrospective");
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("commits when focus leaves the editor", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("Daily standup");
    await user.tab();

    expect(onRename).toHaveBeenCalledTimes(1);
    expect(onRename).toHaveBeenCalledWith("a", "Daily standup");
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("abandons the edit on Escape", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("Something else{Escape}");

    // Escape closes by blurring, the same as Enter — the blur must know it was
    // a cancel rather than commit the draft on the way out.
    expect(onRename).not.toHaveBeenCalled();
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.getByText("Standup")).toBeInTheDocument();
  });

  it("writes nothing for an empty or unchanged name", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("   {Enter}");
    expect(screen.queryByRole("textbox")).toBeNull();

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("textbox")).toBeNull();

    expect(onRename).not.toHaveBeenCalled();
  });

  it("routes Enter through blur rather than committing on its own", async () => {
    // jsdom fires no blur when a focused element is unmounted, so the
    // "exactly once" assertion above cannot tell Enter-commits-then-blurs from
    // Enter-blurs-and-the-blur-commits. With blur stubbed out, the first
    // design commits and the second does nothing — which is what pins it.
    const user = userEvent.setup();
    const onRename = vi.fn();
    const blur = vi.spyOn(HTMLElement.prototype, "blur").mockImplementation(() => undefined);
    try {
      render(
        <RecordingList
          {...BASE_PROPS}
          sessions={[makeSession("a", "Standup")]}
          onDelete={(): void => undefined}
          onRename={onRename}
        />,
      );

      await user.dblClick(screen.getByText("Standup"));
      await user.keyboard("Daily{Enter}");

      expect(blur).toHaveBeenCalled();
      expect(onRename).not.toHaveBeenCalled();
      expect(screen.getByRole("textbox")).toBeInTheDocument();
    } finally {
      blur.mockRestore();
    }
  });

  it("commits a name for an untitled recording", async () => {
    // The unchanged-name guard compares against the stored topic, which is
    // null here — not against the "Untitled recording" the row displays.
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", null)]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Untitled recording"));
    await user.keyboard("Kickoff{Enter}");

    expect(onRename).toHaveBeenCalledWith("a", "Kickoff");
  });

  it("keeps the editor and its draft across a list refresh", async () => {
    // The list is re-read from the backend after a stop or another rename.
    // The editor is keyed by id and holds its own draft, so a refresh that
    // still contains the recording must not close it or reset the text.
    const user = userEvent.setup();
    const { rerender } = render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("Daily");
    rerender(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("b", "Retro"), makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
      />,
    );

    expect(screen.getByRole("textbox", { name: "Rename Standup" })).toHaveValue("Daily");
  });

  it("ignores Enter that confirms an IME composition", async () => {
    // A CJK user presses Enter to accept a candidate. The key reaches the
    // input with `isComposing` set, and committing there stores half a word.
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "会議" } });
    fireEvent.keyDown(input, { key: "Enter", isComposing: true });

    expect(onRename).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });

  it("does not let a cancelled edit poison the next commit", async () => {
    // The cancel flag is consumed by the blur that follows Escape. If it were
    // not reset, the very next Enter-commit on another row would be dropped.
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <RecordingList
        {...BASE_PROPS}
        sessions={[makeSession("a", "Standup"), makeSession("b", "Retro")]}
        onDelete={(): void => undefined}
        onRename={onRename}
      />,
    );

    await user.dblClick(screen.getByText("Standup"));
    await user.keyboard("x{Escape}");
    await user.dblClick(screen.getByText("Retro"));
    await user.keyboard("Retrospective{Enter}");

    expect(onRename).toHaveBeenCalledTimes(1);
    expect(onRename).toHaveBeenCalledWith("b", "Retrospective");
  });
});
