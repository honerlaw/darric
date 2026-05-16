import type React from "react";
import { useRef, useState } from "react";
import { getSetting } from "../../lib/tauri";
import type { Screen } from "../../types";

const PLACEHOLDERS: Record<Screen, string> = {
  timeline: "Ask Darric, or capture a note…",
  meeting: "Ask Darric — about the meeting, your notes, or anything else…",
  board: "Add a task, ask about your work…",
};

interface DockProps {
  activeScreen: Screen;
  onSubmit: (value: string) => void;
  onOpenSettings: () => void;
}

export function Dock({ activeScreen, onSubmit, onOpenSettings }: DockProps): React.JSX.Element {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const handleSubmit = (): void => {
    const trimmed = value.trim();
    if (trimmed.length === 0) return;
    onSubmit(trimmed);
    setValue("");
  };

  const handleFocus = (): void => {
    void (async () => {
      const [provider, claudeKey, geminiKey] = await Promise.all([
        getSetting("ai.provider"),
        getSetting("ai.claude.api_key"),
        getSetting("ai.gemini.api_key"),
      ]);
      const activeProvider = provider ?? "claude";
      const hasKey = activeProvider === "claude"
        ? (claudeKey ?? "").length > 0
        : (geminiKey ?? "").length > 0;
      if (!hasKey) {
        inputRef.current?.blur();
        onOpenSettings();
      }
    })();
  };

  return (
    <div className="shrink-0 px-4 py-[14px]">
      <div className="mx-auto flex max-w-[780px] items-center gap-2 rounded-full border border-line bg-paper-sunken px-[22px] py-[10px] transition-colors focus-within:border-accent focus-within:bg-paper">
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
          }}
          onFocus={handleFocus}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={PLACEHOLDERS[activeScreen]}
          className="flex-1 bg-transparent font-sans text-[15px] text-ink placeholder-ink-4 outline-none placeholder:italic"
        />
        <button
          type="button"
          onClick={handleSubmit}
          disabled={value.trim().length === 0}
          className="flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-full bg-accent text-[13px] text-accent-fg transition-colors hover:bg-accent-hover disabled:cursor-default disabled:opacity-40"
          aria-label="Send"
        >
          ↵
        </button>
      </div>
    </div>
  );
}
