import type React from "react";
import { formatElapsed } from "../../lib/utils";

interface TopBarProps {
  isRecording: boolean;
  sessionTopic: string | null;
  elapsedSeconds: number;
  onStart: () => void;
  onStop: () => void;
  onOpenSettings: () => void;
}

export function TopBar({
  isRecording,
  sessionTopic,
  elapsedSeconds,
  onStart,
  onStop,
  onOpenSettings,
}: TopBarProps): React.JSX.Element {
  return (
    <div className="flex h-12 shrink-0 items-center justify-between border-b border-neutral-800 bg-neutral-900 px-4">
      <div className="flex items-center gap-3">
        {isRecording ? (
          <>
            <span className="inline-block h-2.5 w-2.5 animate-pulse rounded-full bg-red-500" />
            <span className="text-sm font-medium text-neutral-300">Recording</span>
            {sessionTopic !== null && (
              <span className="text-sm text-neutral-500">· {sessionTopic}</span>
            )}
            <span className="text-sm text-neutral-400 tabular-nums">
              {formatElapsed(elapsedSeconds)}
            </span>
          </>
        ) : (
          <span className="text-sm text-neutral-500">Not recording</span>
        )}
      </div>

      <div className="flex items-center gap-2">
        {isRecording ? (
          <button
            onClick={onStop}
            className="rounded bg-red-600 px-3 py-1.5 text-sm text-white transition-colors hover:bg-red-700"
          >
            Stop
          </button>
        ) : (
          <button
            onClick={onStart}
            className="rounded bg-emerald-600 px-3 py-1.5 text-sm text-white transition-colors hover:bg-emerald-700"
          >
            Record
          </button>
        )}
        <button
          onClick={onOpenSettings}
          className="rounded p-1.5 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-200"
          title="Settings"
        >
          <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
