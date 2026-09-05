import { useCallback, useEffect, useRef, useState } from "react";
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
  isRecording: boolean;
  isStarting: boolean;
  /**
   * The recording a start is in flight *for*: the id when resuming, `null` when
   * starting a fresh one (no session exists until `start_session` returns).
   * Callers scope "Starting…" to the right recording with this rather than with
   * `activeSessionId`, which is not assigned until the start has finished.
   */
  startingSessionId: string | null;
  isStopping: boolean;
  modelReady: boolean;
  downloadProgress: number | null;
  elapsedSeconds: number;
  error: string | null;
  start: (topic?: string) => Promise<void>;
  stop: () => Promise<void>;
  resume: (id: string) => Promise<void>;
  update: (id: string, topic?: string) => Promise<void>;
  /**
   * Deletes the recording. Resolves true only when the delete completed *and*
   * the session list was refreshed; false means the recording may still be
   * there, so a caller must not act as though it is gone. Deliberately not just
   * "did `delete_session` succeed" — a refresh that failed afterwards leaves
   * `sessions` still holding the recording, and a UI that deselected it would
   * be pointing away from a row the user can still see.
   *
   * Never rejects: the reason goes to `error`, which `App` renders.
   */
  remove: (id: string) => Promise<boolean>;
  refresh: () => Promise<void>;
}

export function useSession(): UseSessionReturn {
  const [sessions, setSessions] = useState<Session[]>([]);
  // The most recently active recording — deliberately NOT cleared on stop. It
  // outlives the recording so post-stop state can still be attributed to the
  // session it belongs to (the dropped-segment warning is read after
  // `isRecording` goes false). Anything meaning "recording right now" must
  // conjoin `isRecording` rather than read this alone.
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [startingSessionId, setStartingSessionId] = useState<string | null>(null);
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
  // The message the session-write commands (`update`, `remove`) last put in
  // `error`. They clear only their own message, because `error` is one shared
  // slot with no provenance: a succeeding delete has no business erasing a
  // model-download failure that is still true, and `start`/`resume` get away
  // with clearing unconditionally only because they *are* the retry of the
  // thing that failed.
  const writeErrorRef = useRef<string | null>(null);

  const reportWriteFailure = useCallback((e: unknown): void => {
    const message = String(e);
    writeErrorRef.current = message;
    setError(message);
  }, []);

  // Clears this command's own failure message on a later success, and leaves
  // anything another subsystem has since written in place.
  const clearOwnWriteError = useCallback((): void => {
    // Read the ref *before* nulling it: `setError`'s updater runs after this
    // function returns, so it would otherwise compare against the null this
    // very call had just written and clear nothing.
    const own = writeErrorRef.current;
    writeErrorRef.current = null;
    setError((current) => (current === own ? null : current));
  }, []);

  const refresh = useCallback(async (): Promise<void> => {
    const list = await listSessions();
    setSessions(list);
  }, []);

  // Caught here rather than inside `refresh`, which has to keep rejecting: its
  // other callers all await it inside their own `try`, and that is what lets
  // `remove` tell a caller the list is stale. This is the one call site with
  // nobody above it — an unreadable list at mount would otherwise be an
  // unhandled rejection behind an empty sidebar that explains nothing.
  useEffect(() => {
    void refresh().catch((e: unknown) => {
      setError(String(e));
    });
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
        setStartingSessionId(id);
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
        // Only clear the id this call set. `canResume` stays true for the whole
        // resume, so a second one can start while this is in flight — clearing
        // unconditionally would drop the later resume's "Starting…" while it is
        // still starting.
        setStartingSessionId((current) => (current === id ? null : current));
      }
    },
    [refresh],
  );

  const update = useCallback(
    async (id: string, topic?: string): Promise<void> => {
      try {
        await updateSession(id, topic);
        await refresh();
        clearOwnWriteError();
      } catch (e) {
        reportWriteFailure(e);
      }
    },
    [refresh, clearOwnWriteError, reportWriteFailure],
  );

  const remove = useCallback(
    async (id: string): Promise<boolean> => {
      try {
        await deleteSession(id);
        await refresh();
        // After the refresh, not before. On the refresh-failure path this
        // returns false — "the recording may still be there" — and clearing the
        // id first would half-apply the delete behind that answer, leaving a
        // resumable-looking row whose session is gone.
        //
        // Functional, so only the id this call deleted is cleared: reading
        // `activeSessionId` from the closure could be a render behind, and it is
        // what kept this callback depending on the value.
        setActiveSessionId((current) => (current === id ? null : current));
        clearOwnWriteError();
        return true;
      } catch (e) {
        reportWriteFailure(e);
        return false;
      }
    },
    [refresh, clearOwnWriteError, reportWriteFailure],
  );

  return {
    sessions,
    activeSessionId,
    isRecording,
    isStarting,
    startingSessionId,
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
