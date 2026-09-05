import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import {
  deleteSession,
  listSessions,
  onModelDownloadDone,
  onModelDownloadError,
  onModelDownloadProgress,
  onModelDownloadStart,
  onModelReady,
  resumeSession,
  startSession,
  stopSession,
  updateSession,
} from "../lib/tauri";
import type { Session } from "../types";

interface UseSessionReturn {
  sessions: Session[];
  activeSessionId: string | null;
  setActiveSessionId: Dispatch<SetStateAction<string | null>>;
  isRecording: boolean;
  isStarting: boolean;
  modelReady: boolean;
  downloadProgress: number | null;
  elapsedSeconds: number;
  error: string | null;
  start: (topic?: string) => Promise<void>;
  stop: () => Promise<void>;
  resume: (id: string) => Promise<void>;
  update: (id: string, topic?: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useSession(): UseSessionReturn {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refresh = useCallback(async (): Promise<void> => {
    const list = await listSessions();
    setSessions(list);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const unStart = onModelDownloadStart(() => {
      setDownloadProgress(0);
    });
    const unProgress = onModelDownloadProgress((pct) => {
      setDownloadProgress(pct);
    });
    const unDone = onModelDownloadDone(() => {
      setDownloadProgress(null);
    });
    // Clearing the progress is the point, not the message: without a terminal
    // event on the failure path the indicator stays pinned at its last
    // percentage and the Record button stays disabled for the whole session.
    const unError = onModelDownloadError((message) => {
      setDownloadProgress(null);
      setError(`Speech model download failed: ${message}`);
    });
    const unReady = onModelReady(() => {
      setModelReady(true);
    });
    return () => {
      void unStart.then((fn) => {
        fn();
      });
      void unProgress.then((fn) => {
        fn();
      });
      void unDone.then((fn) => {
        fn();
      });
      void unError.then((fn) => {
        fn();
      });
      void unReady.then((fn) => {
        fn();
      });
    };
  }, []);

  const start = useCallback(
    async (topic?: string): Promise<void> => {
      try {
        setIsStarting(true);
        const id = await startSession(topic);
        setActiveSessionId(id);
        setIsRecording(true);
        setElapsedSeconds(0);
        timerRef.current = setInterval(() => {
          setElapsedSeconds((s) => s + 1);
        }, 1000);
        await refresh();
      } catch (e) {
        setError(String(e));
      } finally {
        setIsStarting(false);
      }
    },
    [refresh],
  );

  const stop = useCallback(async (): Promise<void> => {
    try {
      await stopSession();
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRecording(false);
      if (timerRef.current !== null) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      await refresh();
    }
  }, [refresh]);

  const resume = useCallback(
    async (id: string): Promise<void> => {
      try {
        setIsStarting(true);
        await resumeSession(id);
        setActiveSessionId(id);
        setIsRecording(true);
        setElapsedSeconds(0);
        timerRef.current = setInterval(() => {
          setElapsedSeconds((s) => s + 1);
        }, 1000);
        await refresh();
      } catch (e) {
        setError(String(e));
      } finally {
        setIsStarting(false);
      }
    },
    [refresh],
  );

  const update = useCallback(
    async (id: string, topic?: string): Promise<void> => {
      await updateSession(id, topic);
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (id: string): Promise<void> => {
      await deleteSession(id);
      if (activeSessionId === id) setActiveSessionId(null);
      await refresh();
    },
    [activeSessionId, refresh],
  );

  return {
    sessions,
    activeSessionId,
    setActiveSessionId,
    isRecording,
    isStarting,
    modelReady,
    downloadProgress,
    elapsedSeconds,
    error,
    start,
    stop,
    resume,
    update,
    remove,
    refresh,
  };
}
