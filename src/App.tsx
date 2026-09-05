import type React from "react";
import { useEffect, useState } from "react";
import { Header } from "./components/layout/Header";
import { RecorderPane } from "./components/RecorderPane";
import { RecordingList } from "./components/RecordingList";
import { useSession } from "./hooks/useSession";
import { useTranscript } from "./hooks/useTranscript";
import type { CaptureDevice } from "./types";

// Phase 1 captures the default microphone only. The real device list arrives with
// the multi-device capture engine; this placeholder keeps the layout honest until then.
const PLACEHOLDER_DEVICES: CaptureDevice[] = [
  { id: "default-input", name: "Default microphone", direction: "input", enabled: true },
];

export default function App(): React.JSX.Element {
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);

  const {
    sessions,
    activeSessionId,
    isRecording,
    isStarting,
    downloadProgress,
    elapsedSeconds,
    start,
    stop,
    resume,
    update,
    remove: removeSession,
  } = useSession();

  const transcriptLines = useTranscript(
    viewingSessionId,
    isRecording && viewingSessionId === activeSessionId,
  );

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = (dark: boolean): void => {
      document.documentElement.classList.toggle("dark", dark);
    };
    apply(mq.matches);
    const handler = (e: MediaQueryListEvent): void => {
      apply(e.matches);
    };
    mq.addEventListener("change", handler);
    return () => {
      mq.removeEventListener("change", handler);
    };
  }, []);

  useEffect(() => {
    if (isRecording && activeSessionId !== null) setViewingSessionId(activeSessionId);
  }, [isRecording, activeSessionId]);

  const handleRecord = (): void => {
    void start("Recording").catch((err: unknown) => {
      console.error("start recording failed:", err);
    });
  };

  const handleStop = (): void => {
    void stop().catch((err: unknown) => {
      console.error("stop recording failed:", err);
    });
  };

  const handleResume = (): void => {
    if (viewingSessionId === null) return;
    void resume(viewingSessionId);
  };

  const handleRename = (topic: string): void => {
    if (viewingSessionId === null) return;
    const trimmed = topic.trim();
    void update(viewingSessionId, trimmed !== "" ? trimmed : undefined);
  };

  const handleDelete = (id: string): void => {
    if (id === viewingSessionId) setViewingSessionId(null);
    void removeSession(id);
  };

  const viewingSession = sessions.find((s) => s.id === viewingSessionId) ?? null;

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-paper font-sans text-ink">
      <Header
        isRecording={isRecording}
        isStarting={isStarting}
        elapsedSeconds={elapsedSeconds}
        onRecord={handleRecord}
        onStop={handleStop}
      />

      <div className="border-t border-line" />

      <div className="flex flex-1 overflow-hidden">
        <RecordingList
          sessions={sessions}
          selectedId={viewingSessionId}
          activeId={activeSessionId}
          onSelect={setViewingSessionId}
          onDelete={handleDelete}
        />
        <RecorderPane
          session={viewingSession}
          transcriptLines={transcriptLines}
          devices={PLACEHOLDER_DEVICES}
          isRecording={isRecording && viewingSessionId === activeSessionId}
          isStarting={isStarting}
          elapsedSeconds={elapsedSeconds}
          downloadProgress={downloadProgress}
          onResume={handleResume}
          onRename={handleRename}
        />
      </div>
    </div>
  );
}
