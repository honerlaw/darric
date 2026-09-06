import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { McpChip } from "./McpChip";
import { claudeMcpAddCommand } from "../lib/mcp";
import type { McpServerStatus } from "../types";

const LISTENING: McpServerStatus = {
  listening: true,
  port: 27842,
  url: "http://127.0.0.1:27842/mcp",
  port_busy: false,
  error: null,
};

const BUSY: McpServerStatus = {
  listening: false,
  port: 27842,
  url: null,
  port_busy: true,
  error: "bind 127.0.0.1:27842: Address already in use (os error 48)",
};

const OFF: McpServerStatus = {
  listening: false,
  port: 27842,
  url: null,
  port_busy: false,
  error: "database error: unable to open database file",
};

describe("McpChip", () => {
  const writeText = vi.fn<(text: string) => Promise<void>>();

  beforeEach(() => {
    writeText.mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
  });

  afterEach(() => {
    writeText.mockReset();
  });

  it("renders nothing until the status has resolved", () => {
    const { container } = render(<McpChip status={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the port and copies the connect command on click", async () => {
    // user-event installs its own clipboard on setup, replacing the stub from
    // beforeEach, so the spy has to be taken after setup.
    const user = userEvent.setup();
    const clipboardWrite = vi.spyOn(navigator.clipboard, "writeText");
    render(<McpChip status={LISTENING} />);

    const chip = screen.getByRole("button");
    expect(chip).toHaveTextContent("MCP · :27842");

    await user.click(chip);

    expect(clipboardWrite).toHaveBeenCalledWith(
      "claude mcp add --transport http darric http://127.0.0.1:27842/mcp",
    );
    expect(chip).toHaveTextContent("Copied");
  });

  it("returns to the port after the copied notice times out", async () => {
    vi.useFakeTimers();
    try {
      render(<McpChip status={LISTENING} />);
      const chip = screen.getByRole("button");

      await act(async () => {
        chip.click();
        await Promise.resolve();
      });
      expect(chip).toHaveTextContent("Copied");

      act(() => {
        vi.advanceTimersByTime(2000);
      });
      expect(chip).toHaveTextContent("MCP · :27842");
    } finally {
      vi.useRealTimers();
    }
  });

  it("reports a busy port with the reason as its title, and offers nothing to copy", () => {
    render(<McpChip status={BUSY} />);

    const chip = screen.getByRole("status");
    expect(chip).toHaveTextContent("MCP · port busy");
    expect(chip).toHaveAttribute("title", BUSY.error);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("does not blame the port for a failure that is not the port", () => {
    // "port busy" sends the user to free the port; a read-only open failure
    // is not that, and the README's advice would be wrong for it.
    render(<McpChip status={OFF} />);

    const chip = screen.getByRole("status");
    expect(chip).toHaveTextContent("MCP · off");
    expect(chip).toHaveAttribute("title", OFF.error);
  });

  it("builds the command around the server's own url", () => {
    expect(claudeMcpAddCommand("http://127.0.0.1:1234/mcp")).toBe(
      "claude mcp add --transport http darric http://127.0.0.1:1234/mcp",
    );
  });
});
