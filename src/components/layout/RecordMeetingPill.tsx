import type React from "react";

interface RecordMeetingPillProps {
  isRecording: boolean;
  onClick: () => void;
}

export function RecordMeetingPill({
  isRecording,
  onClick,
}: RecordMeetingPillProps): React.JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex h-[30px] cursor-pointer items-center gap-[6px] rounded-full border border-line bg-paper px-[14px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken"
    >
      <span
        className={`h-[7px] w-[7px] rounded-full bg-accent ${isRecording ? "pulse-dot" : ""}`}
      />
      <span>{isRecording ? "Recording…" : "Record meeting"}</span>
    </button>
  );
}
