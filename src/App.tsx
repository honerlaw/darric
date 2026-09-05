import type React from "react";
import { useEffect, useState } from "react";
import { Header } from "./components/layout/Header";
import { RecorderPane } from "./components/RecorderPane";
import { RecordingList } from "./components/RecordingList";
import { useDevices } from "./hooks/useDevices";
import { useSession } from "./hooks/useSession";
import { useTranscript } from "./hooks/useTranscript";
import { captureDropCount } from "./lib/tauri";

/** How often the dropped-segment counter refreshes while recording. */
const DROP_POLL_MS = 2000;

export default function App(): React.JSX.Element {
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);

  const {
    sessions,
    activeSessionId,
    isRecording,
    isStarting,
    downloadProgress,
    elapsedSeconds,
    error,
    start,
    stop,
    resume,
    update,
    remove: removeSession,
  } = useSession();

  const { devices, toggle: toggleDevice } = useDevices(isRecording);
  const [droppedSegments, setDroppedSegments] = useState(0);

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

  useEffect(() => {
    if (!isRecording) return;
    const timer = setInterval(() => {
      void captureDropCount()
        .then(setDroppedSegments)
        .catch(() => {
          // A failed count is not worth surfacing; the next tick retries.
        });
    }, DROP_POLL_MS);
    return () => {
      clearInterval(timer);
    };
  }, [isRecording]);

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
          devices={devices}
          onToggleDevice={(id, enabled) => {
            void toggleDevice(id, enabled);
          }}
          droppedSegments={droppedSegments}
          isRecording={isRecording && viewingSessionId === activeSessionId}
          isStarting={isStarting}
          elapsedSeconds={elapsedSeconds}
          canResume={!isRecording}
          downloadProgress={downloadProgress}
          onResume={handleResume}
          onRename={handleRename}
        />
      </div>

      {error !== null && (
        <div className="shrink-0 border-t border-line bg-paper-sunken px-6 py-2">
          <span className="font-mono text-[11px] text-danger">{error}</span>
        </div>
      )}
    </div>
  );
}
