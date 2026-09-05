import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ModelDownloadBanner } from "./ModelDownloadBanner";

describe("ModelDownloadBanner", () => {
  it("renders nothing when no download is in flight", () => {
    const { container } = render(<ModelDownloadBanner progress={null} />);

    expect(container).toBeEmptyDOMElement();
  });

  it("shows the percentage on a progressbar while downloading", () => {
    render(<ModelDownloadBanner progress={63} />);

    expect(screen.getByText("63%")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "63");
  });

  it("renders at 0% rather than treating it as absent", () => {
    render(<ModelDownloadBanner progress={0} />);

    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
  });

  it("clamps a percentage the backend reported out of range", () => {
    // A server under-reporting Content-Length pushes the computed percentage
    // past 100, which would otherwise reach aria-valuenow and the bar's width.
    render(<ModelDownloadBanner progress={137} />);

    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });
});
