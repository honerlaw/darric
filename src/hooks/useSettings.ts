import { useCallback, useEffect, useState } from "react";
import { getSetting, saveSetting } from "../lib/tauri";

interface UseSettingsReturn {
  whisperModelPath: string;
  loading: boolean;
  saveModelPath: (path: string) => Promise<void>;
}

export function useSettings(): UseSettingsReturn {
  const [whisperModelPath, setWhisperModelPath] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    void getSetting("whisper_model_path").then((path) => {
      setWhisperModelPath(path ?? "");
      setLoading(false);
    });
  }, []);

  const saveModelPath = useCallback(async (path: string): Promise<void> => {
    await saveSetting("whisper_model_path", path);
    setWhisperModelPath(path);
  }, []);

  return { whisperModelPath, loading, saveModelPath };
}
