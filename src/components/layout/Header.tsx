import type React from "react";
import type { Screen } from "../../types";
import { RecordMeetingPill } from "./RecordMeetingPill";

interface HeaderProps {
  activeScreen: Screen;
  isRecording: boolean;
  elapsedSeconds: number;
  onNavigate: (screen: Screen) => void;
  onRecordClick: () => void;
  onNewNote: () => void;
  onOpenSettings: () => void;
  onOpenSearch: () => void;
}

function formatTimer(sec: number): string {
  const m = Math.floor(sec / 60)
    .toString()
    .padStart(2, "0");
  const s = (sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

const NAV: { id: Screen; label: string }[] = [
  { id: "timeline", label: "Today" },
  { id: "board", label: "Board" },
];

export function Header({
  activeScreen,
  isRecording,
  elapsedSeconds,
  onNavigate,
  onRecordClick,
  onNewNote,
  onOpenSettings,
  onOpenSearch,
}: HeaderProps): React.JSX.Element {
  const isMeeting = activeScreen === "meeting";

  return (
    <header className="flex h-14 shrink-0 items-center gap-8 px-6" data-tauri-drag-region="true">
      {/* Brand */}
      <div className="flex items-center gap-2">
        <div className="h-3 w-3 rounded-[2px] bg-ink" />
        <span className="font-sans text-[15px] font-[500] tracking-[-0.02em] text-ink">Darric</span>
      </div>

      {/* Nav */}
      {!isMeeting && (
        <nav className="flex items-center gap-6">
          {NAV.map(({ id, label }) => {
            const active = activeScreen === id;
            return (
              <button
                key={id}
                type="button"
                onClick={() => {
                  onNavigate(id);
                }}
                className={`relative cursor-pointer pb-[4px] text-[14px] font-[400] transition-colors ${
                  active ? "text-accent" : "text-ink-3 hover:text-ink-2"
                }`}
              >
                {label}
                {active && (
                  <span className="absolute right-0 bottom-0 left-0 h-[1.5px] rounded-full bg-accent" />
                )}
              </button>
            );
          })}
        </nav>
      )}

      {/* Meeting breadcrumb */}
      {isMeeting && (
        <div className="flex items-center gap-2 text-[14px] text-ink-3">
          <button
            type="button"
            onClick={() => {
              onNavigate("timeline");
            }}
            className="cursor-pointer hover:text-ink"
          >
            Darric
          </button>
          <span>/</span>
          <span className="text-ink">meeting</span>
        </div>
      )}

      {/* Search trigger */}
      {!isMeeting && (
        <button
          type="button"
          onClick={onOpenSearch}
          className="mx-auto flex h-[30px] cursor-pointer items-center gap-2 rounded-full border border-line bg-paper-sunken px-4 text-[13px] text-ink-3 transition-colors hover:border-line-strong hover:text-ink"
        >
          <span>Search</span>
          <kbd className="font-mono text-[10px] text-ink-4">⌘K</kbd>
        </button>
      )}

      <div className="flex-1" />

      {/* Right side */}
      {isMeeting ? (
        <div className="flex items-center gap-2">
          <span className="pulse-dot h-2 w-2 rounded-full bg-danger" />
          <span className="font-mono text-[11px] tracking-eyebrow text-ink-3 uppercase">
            RECORDING · {formatTimer(elapsedSeconds)}
          </span>
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onNewNote}
            className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken"
          >
            ＋ Note
          </button>
          <RecordMeetingPill isRecording={isRecording} onClick={onRecordClick} />
        </div>
      )}

      {/* Avatar — opens Settings */}
      <button
        type="button"
        onClick={onOpenSettings}
        title="Settings"
        className="flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-full bg-paper-sunken font-mono text-[11px] font-[500] text-ink-3 transition-colors hover:bg-paper-sunken hover:text-ink"
      >
        D
      </button>
    </header>
  );
}
