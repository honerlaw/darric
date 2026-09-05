import { useCallback, useEffect, useState } from "react";
import { listCaptureDevices, setDeviceEnabled } from "../lib/tauri";
import type { CaptureDevice } from "../types";

/** How often device levels refresh while recording. Fast enough to read as a meter. */
const LIVE_POLL_MS = 250;

interface UseDevicesReturn {
  devices: CaptureDevice[];
  toggle: (id: string, enabled: boolean) => Promise<void>;
  refresh: () => Promise<void>;
}

/**
 * The machine's capture devices, polled for level meters while recording.
 *
 * Polling only runs when `isRecording` — an idle app has no meters to animate,
 * and enumerating devices on a timer forever is a needless wakeup.
 */
export function useDevices(isRecording: boolean): UseDevicesReturn {
  const [devices, setDevices] = useState<CaptureDevice[]>([]);

  const refresh = useCallback(async (): Promise<void> => {
    try {
      setDevices(await listCaptureDevices());
    } catch (e) {
      console.error("listing capture devices failed:", e);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!isRecording) return;
    const timer = setInterval(() => {
      void refresh();
    }, LIVE_POLL_MS);
    return () => {
      clearInterval(timer);
    };
  }, [isRecording, refresh]);

  const toggle = useCallback(
    async (id: string, enabled: boolean): Promise<void> => {
      // Optimistic: the switch should not lag the click.
      setDevices((prev) => prev.map((d) => (d.id === id ? { ...d, enabled } : d)));
      try {
        await setDeviceEnabled(id, enabled);
      } catch (e) {
        // Swallowing this would leave the switch showing a state the backend
        // never accepted. The refresh below re-reads the truth either way.
        console.error("toggling capture device failed:", e);
      } finally {
        await refresh();
      }
    },
    [refresh],
  );

  return { devices, toggle, refresh };
}
