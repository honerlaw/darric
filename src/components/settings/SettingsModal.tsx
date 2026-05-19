import { openUrl } from "@tauri-apps/plugin-opener";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { getSetting, listMcpServers, mcpServerStatus, saveSetting } from "../../lib/tauri";
import type { McpServerStatus } from "../../types";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

const DEFAULT_MCP_PORT = 27842;

function buildClaudeDesktopSnippet(url: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        darric: { url },
      },
    },
    null,
    2,
  );
}

export function SettingsModal({ open, onClose }: SettingsModalProps): React.JSX.Element | null {
  const inputRef = useRef<HTMLInputElement>(null);

  const [aiProvider, setAiProvider] = useState<"claude" | "gemini">("claude");
  const [claudeKey, setClaudeKey] = useState("");
  const [geminiKey, setGeminiKey] = useState("");
  const [showClaudeKey, setShowClaudeKey] = useState(false);
  const [showGeminiKey, setShowGeminiKey] = useState(false);
  const [aiSaved, setAiSaved] = useState(false);
  const [mcpServers, setMcpServers] = useState<string[]>([]);

  const [mcpServerEnabled, setMcpServerEnabled] = useState(true);
  const [mcpServerPort, setMcpServerPort] = useState<string>(String(DEFAULT_MCP_PORT));
  const [mcpServerLive, setMcpServerLive] = useState<McpServerStatus | null>(null);
  const [mcpServerSaved, setMcpServerSaved] = useState(false);
  const [snippetCopied, setSnippetCopied] = useState(false);

  useEffect(() => {
    if (!open) return;
    inputRef.current?.focus();

    void Promise.all([
      getSetting("ai.provider"),
      getSetting("ai.claude.api_key"),
      getSetting("ai.gemini.api_key"),
      listMcpServers(),
      getSetting("mcp_server.enabled"),
      getSetting("mcp_server.port"),
      mcpServerStatus(),
    ]).then(([provider, claude, gemini, servers, srvEnabled, srvPort, srvStatus]) => {
      setAiProvider((provider as "claude" | "gemini" | null) ?? "claude");
      setClaudeKey(claude ?? "");
      setGeminiKey(gemini ?? "");
      setMcpServers(servers);
      setMcpServerEnabled(srvEnabled !== "false");
      setMcpServerPort(srvPort ?? String(DEFAULT_MCP_PORT));
      setMcpServerLive(srvStatus);
    });
  }, [open]);

  if (!open) return null;

  const parsedPort = Number.parseInt(mcpServerPort, 10);
  const portValid = Number.isFinite(parsedPort) && parsedPort > 0 && parsedPort < 65536;

  const handleSave = async (): Promise<void> => {
    await saveSetting("ai.provider", aiProvider);
    if (claudeKey.trim().length > 0) await saveSetting("ai.claude.api_key", claudeKey.trim());
    if (geminiKey.trim().length > 0) await saveSetting("ai.gemini.api_key", geminiKey.trim());
    setAiSaved(true);
    setTimeout(() => {
      setAiSaved(false);
    }, 2000);
  };

  const handleSaveMcpServer = async (): Promise<void> => {
    if (!portValid) return;
    await saveSetting("mcp_server.enabled", mcpServerEnabled ? "true" : "false");
    await saveSetting("mcp_server.port", String(parsedPort));
    setMcpServerSaved(true);
    setTimeout(() => {
      setMcpServerSaved(false);
    }, 2000);
  };

  const effectivePort = portValid ? parsedPort : DEFAULT_MCP_PORT;
  const fallbackUrl = `http://127.0.0.1:${String(effectivePort)}/mcp`;
  const serverDisplayUrl = mcpServerLive?.url ?? fallbackUrl;

  const handleCopySnippet = async (): Promise<void> => {
    await navigator.clipboard.writeText(buildClaudeDesktopSnippet(serverDisplayUrl));
    setSnippetCopied(true);
    setTimeout(() => {
      setSnippetCopied(false);
    }, 2000);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="max-h-[90vh] w-full max-w-md overflow-y-auto rounded-xl border border-neutral-700 bg-neutral-900 p-6 shadow-2xl">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-base font-semibold text-neutral-100">Settings</h2>
          <button
            onClick={onClose}
            className="text-neutral-500 transition-colors hover:text-neutral-300"
          >
            <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <p className="mb-3 text-sm font-medium text-neutral-400">AI Provider</p>

        <div className="mb-3 flex gap-3">
          {(["claude", "gemini"] as const).map((p) => (
            <button
              key={p}
              type="button"
              onClick={() => {
                setAiProvider(p);
              }}
              className={`rounded px-3 py-1.5 text-sm font-medium capitalize transition-colors ${
                aiProvider === p
                  ? "bg-indigo-600 text-white"
                  : "bg-neutral-800 text-neutral-400 hover:text-neutral-200"
              }`}
            >
              {p}
            </button>
          ))}
        </div>

        {aiProvider === "claude" && (
          <div className="flex flex-col gap-4">
            <div>
              <label className="mb-1.5 block text-xs font-medium tracking-wider text-neutral-500 uppercase">
                Anthropic API Key
              </label>
              <div className="relative">
                <input
                  ref={inputRef}
                  type={showClaudeKey ? "text" : "password"}
                  value={claudeKey}
                  onChange={(e) => {
                    setClaudeKey(e.target.value);
                  }}
                  placeholder="sk-ant-..."
                  className="w-full rounded border border-neutral-700 bg-neutral-800 px-3 py-2 pr-9 text-sm text-neutral-100 placeholder:text-neutral-600 focus:ring-1 focus:ring-neutral-500 focus:outline-none"
                />
                <button
                  type="button"
                  onClick={() => {
                    setShowClaudeKey((v) => !v);
                  }}
                  className="absolute inset-y-0 right-0 flex items-center px-2.5 text-neutral-500 transition-colors hover:text-neutral-300"
                  tabIndex={-1}
                >
                  {showClaudeKey ? (
                    <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
                      />
                    </svg>
                  ) : (
                    <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                      />
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                      />
                    </svg>
                  )}
                </button>
              </div>
              <button
                type="button"
                onClick={() => {
                  void openUrl("https://console.anthropic.com/settings/keys");
                }}
                className="mt-1.5 text-xs text-indigo-400 transition-colors hover:text-indigo-300"
              >
                Get your Anthropic API key →
              </button>
            </div>

            <div>
              <p className="mb-2 text-xs font-medium tracking-wider text-neutral-500 uppercase">
                MCP Servers
              </p>
              {mcpServers.length === 0 ? (
                <p className="text-xs text-neutral-600">
                  No MCP servers detected from Claude Desktop.
                </p>
              ) : (
                <ul className="flex flex-col gap-1">
                  {mcpServers.map((name) => (
                    <li
                      key={name}
                      className="flex items-center gap-2 rounded bg-neutral-800 px-3 py-1.5"
                    >
                      <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500" />
                      <span className="text-sm text-neutral-200">{name}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}

        {aiProvider === "gemini" && (
          <div>
            <label className="mb-1.5 block text-xs font-medium tracking-wider text-neutral-500 uppercase">
              Google Gemini API Key
            </label>
            <div className="relative">
              <input
                ref={inputRef}
                type={showGeminiKey ? "text" : "password"}
                value={geminiKey}
                onChange={(e) => {
                  setGeminiKey(e.target.value);
                }}
                placeholder="AIza..."
                className="w-full rounded border border-neutral-700 bg-neutral-800 px-3 py-2 pr-9 text-sm text-neutral-100 placeholder:text-neutral-600 focus:ring-1 focus:ring-neutral-500 focus:outline-none"
              />
              <button
                type="button"
                onClick={() => {
                  setShowGeminiKey((v) => !v);
                }}
                className="absolute inset-y-0 right-0 flex items-center px-2.5 text-neutral-500 transition-colors hover:text-neutral-300"
                tabIndex={-1}
              >
                {showGeminiKey ? (
                  <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
                    />
                  </svg>
                ) : (
                  <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                    />
                  </svg>
                )}
              </button>
            </div>
            <button
              type="button"
              onClick={() => {
                void openUrl("https://aistudio.google.com/app/apikey");
              }}
              className="mt-1.5 text-xs text-indigo-400 transition-colors hover:text-indigo-300"
            >
              Get your Gemini API key →
            </button>
          </div>
        )}

        <p className="mt-2 text-xs text-neutral-600">
          MCP servers from Claude Desktop are auto-discovered and require no extra setup.
        </p>

        <div className="mt-4 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded px-4 py-2 text-sm text-neutral-400 transition-colors hover:text-neutral-200"
          >
            Cancel
          </button>
          <button
            onClick={() => {
              void handleSave();
            }}
            className="rounded bg-indigo-600 px-4 py-2 text-sm text-white transition-colors hover:bg-indigo-700"
          >
            {aiSaved ? "Saved!" : "Save"}
          </button>
        </div>

        <hr className="my-5 border-neutral-800" />

        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-neutral-400">darric as MCP Server</p>
            <p className="mt-1 text-xs text-neutral-600">
              Expose your notes, meetings, and tasks read-only to Claude Desktop / Claude CLI.
            </p>
          </div>
          <label className="relative inline-flex cursor-pointer items-center">
            <input
              type="checkbox"
              checked={mcpServerEnabled}
              onChange={(e) => {
                setMcpServerEnabled(e.target.checked);
              }}
              className="peer sr-only"
            />
            <div className="h-5 w-9 rounded-full bg-neutral-700 transition-colors peer-checked:bg-indigo-600" />
            <div className="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform peer-checked:translate-x-4" />
          </label>
        </div>

        <div className="mt-3 flex items-center gap-3">
          <div className="flex-1">
            <label className="mb-1.5 block text-xs font-medium tracking-wider text-neutral-500 uppercase">
              Port
            </label>
            <input
              type="number"
              min={1}
              max={65535}
              value={mcpServerPort}
              onChange={(e) => {
                setMcpServerPort(e.target.value);
              }}
              disabled={!mcpServerEnabled}
              className="w-full rounded border border-neutral-700 bg-neutral-800 px-3 py-2 text-sm text-neutral-100 focus:ring-1 focus:ring-neutral-500 focus:outline-none disabled:opacity-50"
            />
          </div>
          <div className="flex-1">
            <p className="mb-1.5 text-xs font-medium tracking-wider text-neutral-500 uppercase">
              Status
            </p>
            <div className="flex items-center gap-2 rounded bg-neutral-800 px-3 py-2">
              <span
                className={`h-2 w-2 shrink-0 rounded-full ${
                  mcpServerLive?.listening === true ? "bg-emerald-500" : "bg-neutral-600"
                }`}
              />
              <span className="truncate text-xs text-neutral-300">
                {mcpServerLive?.listening === true
                  ? `Live on ${mcpServerLive.bound_port?.toString() ?? "?"}`
                  : "Not listening"}
              </span>
            </div>
          </div>
        </div>

        {mcpServerEnabled && (
          <div className="mt-3">
            <p className="mb-1.5 text-xs font-medium tracking-wider text-neutral-500 uppercase">
              Claude Desktop config snippet
            </p>
            <pre className="overflow-x-auto rounded border border-neutral-800 bg-neutral-950 px-3 py-2 text-[11px] leading-relaxed text-neutral-300">
              {buildClaudeDesktopSnippet(serverDisplayUrl)}
            </pre>
            <button
              type="button"
              onClick={() => {
                void handleCopySnippet();
              }}
              className="mt-1.5 text-xs text-indigo-400 transition-colors hover:text-indigo-300"
            >
              {snippetCopied ? "Copied!" : "Copy snippet"}
            </button>
          </div>
        )}

        <p className="mt-3 text-xs text-neutral-600">
          Changes to the MCP server require an app restart to take effect.
        </p>

        <div className="mt-3 flex justify-end">
          <button
            onClick={() => {
              void handleSaveMcpServer();
            }}
            disabled={!portValid}
            className="rounded bg-indigo-600 px-4 py-2 text-sm text-white transition-colors hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {mcpServerSaved ? "Saved!" : "Save MCP settings"}
          </button>
        </div>
      </div>
    </div>
  );
}
