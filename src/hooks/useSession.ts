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
  modelDownloadState,
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
  isStopping: boolean;
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
  const [isStopping, setIsStopping] = useState(false);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  // Read by `stop` to reject a re-entrant call. A ref rather than the state
  // above because the guard has to see the value set by the call in flight,
  // not the one captured when this `stop` closure was created.
  const stoppingRef = useRef(false);

  const refresh = useCallback(async (): Promise<void> => {
    const list = await listSessions();
    setSessions(list);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // A download started in Tauri's `setup()` emits before the webview holds any
  // listener, so its `model_download_start` — and every tick until the next one
  // — is lost. Seeding from a query is what makes the indicator correct for a
  // frontend that mounted mid-download, which on a fresh install is every run.
  useEffect(() => {
    void modelDownloadState()
      .then((pct) => {
        setDownloadProgress((current) => current ?? pct);
      })
      .catch(() => {
        // An unreadable seed just means the events remain the only source.
      });
  }, []);

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
        // A failed download's message would otherwise outlive the retry that
        // fixed it and sit in the error bar for the rest of the session.
        setError(null);
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
    // `stop_session` takes the engine out of app state on entry, so a second
    // concurrent call finds none and fails with NoSession. Guarding here rather
    // than relying on the Stop button's `disabled` keeps that invariant with the
    // hook that owns it, instead of with whichever control happens to call it.
    if (stoppingRef.current) return;
    // Everything after the guard runs inside the `try`, so the `finally` below
    // is the single reset path — the flag cannot wedge true on a throw.
    try {
      stoppingRef.current = true;
      setIsStopping(true);
      // Capture has already ended by the time the command returns — the seconds
      // it spends there are flush and transcription. Freezing the clock on the
      // click is both the earliest feedback available and the honest elapsed
      // number.
      if (timerRef.current !== null) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      await stopSession();
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRecording(false);
      setIsStopping(false);
      stoppingRef.current = false;
      await refresh();
    }
  }, [refresh]);

  const resume = useCallback(
    async (id: string): Promise<void> => {
      try {
        setIsStarting(true);
        setError(null);
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
    isStopping,
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
