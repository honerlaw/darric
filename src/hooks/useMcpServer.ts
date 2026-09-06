import { useEffect, useState } from "react";
import { mcpServerStatus } from "../lib/tauri";
import type { McpServerStatus } from "../types";

/**
 * Whether the in-app MCP server is listening, read once on mount.
 *
 * Polled rather than pushed: the bind happens in the backend's `setup`, and an
 * event emitted there reaches no webview — none is listening yet. The outcome
 * does not change for the life of the process, so once is enough.
 */
export function useMcpServer(): McpServerStatus | null {
  const [status, setStatus] = useState<McpServerStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    mcpServerStatus()
      .then((s) => {
        if (!cancelled) setStatus(s);
      })
      .catch((e: unknown) => {
        console.error("reading MCP server status failed:", e);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return status;
}
