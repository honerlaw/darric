import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Header } from "./Header";

const BASE_PROPS = {
  isRecording: false,
  isStarting: false,
  isStopping: false,
  downloadProgress: null,
  elapsedSeconds: 0,
  resumeTarget: null,
  mcpStatus: null,
  onRecord: (): void => undefined,
  onStop: (): void => undefined,
  onResume: (): void => undefined,
};

describe("Header record button", () => {
  it("reports the model download instead of showing a bare Starting…", () => {
    render(<Header {...BASE_PROPS} downloadProgress={42} />);

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Downloading 42%");
    expect(button).toBeDisabled();
  });

  it("keeps reporting the download even after Record was pressed", () => {
    // Pressing Record mid-download sets isStarting as well. "Starting…" is the
    // label that made a multi-minute download look like a frozen app, so the
    // download has to win the tie.
    render(<Header {...BASE_PROPS} isStarting downloadProgress={7} />);

    expect(screen.getByRole("button")).toHaveTextContent("Downloading 7%");
  });

  it("offers Record once no download is in flight", () => {
    render(<Header {...BASE_PROPS} />);

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Record");
    expect(button).toBeEnabled();
  });

  it("falls back to Starting… when a start is in flight with no download", () => {
    render(<Header {...BASE_PROPS} isStarting />);

    expect(screen.getByRole("button")).toHaveTextContent("Starting…");
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("reports a stop in flight instead of leaving the button on Stop", () => {
    // The recording stays live for the whole flush, so `isStopping` has to win
    // the tie with `isRecording` — otherwise the several seconds the backend
    // spends draining the whisper queue look like a click that did nothing.
    render(<Header {...BASE_PROPS} isRecording isStopping elapsedSeconds={65} />);

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Stopping…");
    // Marked unavailable without taking focus away — `useSession.stop` is what
    // actually rejects the re-entrant call.
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByText(/finishing · 01:05/)).toBeInTheDocument();
    expect(screen.queryByText(/recording ·/)).toBeNull();
  });

  it("stops every pulse while the stop is in flight", () => {
    // The pulsing red dot is the strongest "still recording" affordance in the
    // chrome. Leaving it running restores most of the original complaint even
    // with the label corrected.
    const { container } = render(<Header {...BASE_PROPS} isRecording isStopping />);

    expect(container.querySelectorAll(".pulse-dot")).toHaveLength(0);
  });

  it("pulses while the recording is genuinely live", () => {
    // The positive control for the assertion above.
    const { container } = render(<Header {...BASE_PROPS} isRecording />);

    expect(container.querySelectorAll(".pulse-dot").length).toBeGreaterThan(0);
  });

  it("keeps focus on the button it disables mid-interaction", () => {
    // The user has just pressed this button. A native `disabled` would move
    // their focus to the body and announce nothing — the original confusion,
    // relocated to assistive tech.
    render(<Header {...BASE_PROPS} isRecording isStopping />);

    const button = screen.getByRole("button");
    expect(button).toBeEnabled();
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("status")).toHaveTextContent("Stopping — finishing transcription");
  });

  it("never disables Stop, even while a download is still streaming", () => {
    // One button serves both roles. The startup download can still be running
    // when a recording begins (the recording only needed the model loaded), and
    // disabling the button then leaves the user unable to stop it.
    render(<Header {...BASE_PROPS} isRecording downloadProgress={64} />);

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Stop");
    expect(button).toBeEnabled();
  });
});

describe("Header resume button", () => {
  it("offers Resume beside Record when a stopped recording is selected", () => {
    render(<Header {...BASE_PROPS} resumeTarget="Standup" />);

    // The button names what it acts on: in the pane it sat under the recording
    // it would continue, and in global chrome nothing else says which one.
    const resume = screen.getByRole("button", { name: "Resume recording “Standup”" });
    const record = screen.getByRole("button", { name: /Record/ });
    expect(resume).toBeInTheDocument();
    // Immediately beside Record, not merely somewhere else in the chrome — the
    // whole point of the move is that both ways to begin capturing sit together.
    expect(record.previousElementSibling).toBe(resume);
  });

  it("withholds Resume when there is nothing resumable", () => {
    // `canResume` is false for four distinct reasons (nothing selected, a
    // recording in flight, a start in flight, a model download). The header only
    // has to honour the answer.
    render(<Header {...BASE_PROPS} />);

    expect(screen.queryByRole("button", { name: /Resume recording/ })).toBeNull();
  });

  it("resumes rather than starting a new recording when pressed", async () => {
    const user = userEvent.setup();
    const onResume = vi.fn();
    const onRecord = vi.fn();
    render(
      <Header {...BASE_PROPS} resumeTarget="Standup" onResume={onResume} onRecord={onRecord} />,
    );

    await user.click(screen.getByRole("button", { name: /Resume recording/ }));

    expect(onResume).toHaveBeenCalledTimes(1);
    expect(onRecord).not.toHaveBeenCalled();
  });
});
