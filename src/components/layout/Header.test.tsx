import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Header } from "./Header";

const BASE_PROPS = {
  isRecording: false,
  isStarting: false,
  downloadProgress: null,
  elapsedSeconds: 0,
  onRecord: (): void => undefined,
  onStop: (): void => undefined,
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
});
