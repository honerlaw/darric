import type React from "react";
import { useEffect, useState } from "react";
import { claudeMcpAddCommand } from "../lib/mcp";
import type { McpServerStatus } from "../types";

/** How long the chip reads "Copied" after a click. */
const COPIED_MS = 2000;

interface McpChipProps {
  /** Null until the status command has answered; the chip stays hidden. */
  status: McpServerStatus | null;
}

/**
 * The header's MCP indicator: the port when listening, "port busy" when the
 * bind failed. Clicking a listening chip copies the `claude mcp add` command,
 * which is the whole setup for a Claude Code user.
 */
export function McpChip({ status }: McpChipProps): React.JSX.Element | null {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => {
      setCopied(false);
    }, COPIED_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);

  if (status === null) return null;

  const label = "font-mono text-[11px] tracking-eyebrow text-ink-3 uppercase";

  if (!status.listening || status.url === null) {
    return (
      <span
        role="status"
        title={status.error ?? "The MCP server is not listening"}
        className={`flex h-[24px] items-center gap-[6px] rounded-full border border-line px-[10px] ${label}`}
      >
        <span className="h-[6px] w-[6px] rounded-full bg-danger" />
        MCP · port busy
      </span>
    );
  }

  const url = status.url;
  const copy = (): void => {
    navigator.clipboard
      .writeText(claudeMcpAddCommand(url))
      .then(() => {
        setCopied(true);
      })
      .catch((e: unknown) => {
        console.error("copying the MCP command failed:", e);
      });
  };

  return (
    <button
      type="button"
      onClick={copy}
      title={`Copy: ${claudeMcpAddCommand(url)}`}
      aria-label="Copy the Claude Code command that connects to darric"
      className={`flex h-[24px] cursor-pointer items-center gap-[6px] rounded-full border border-line px-[10px] transition-colors hover:border-line-strong hover:bg-paper-sunken ${label}`}
    >
      <span className="h-[6px] w-[6px] rounded-full bg-accent" />
      {copied ? "Copied" : `MCP · :${String(status.port)}`}
    </button>
  );
}
