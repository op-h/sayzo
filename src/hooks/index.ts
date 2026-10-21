import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, EngineStatus, Progress, Settings, Snapshot } from "@/lib/api";

const useTauriEvent = <T,>(event: string, handler: (payload: T) => void) => {
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => handler(e.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event]);
};

/** Settings live in the Rust side so every window sees the same values */
export const useSettings = () => {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings);
  }, []);
  useTauriEvent<Settings>("settings://changed", setSettings);

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return [];
      const next = { ...settings, ...patch };
      setSettings(next);
      return api.updateSettings(next);
    },
    [settings]
  );

  return { settings, update };
};

const EMPTY: Snapshot = { sentences: [], source: "", status: "idle", current: 0, total: 0, error: null };

export const useReader = () => {
  const [state, setState] = useState<Snapshot>(EMPTY);

  useEffect(() => {
    api.getReaderState().then(setState);
  }, []);
  useTauriEvent<Snapshot>("reader://session", setState);
  useTauriEvent<Progress>("reader://progress", (p) => setState((s) => ({ ...s, ...p })));

  return state;
};

export const useEngineStatus = () => {
  const [status, setStatus] = useState<EngineStatus>({ ready: false, error: null });
  useEffect(() => {
    api.getEngineStatus().then(setStatus);
  }, []);
  useTauriEvent<EngineStatus>("engine://status", setStatus);
  return status;
};

/** Short status messages sent from Rust, such as "Select some text first" */
export const useToast = (duration = 2600) => {
  const [message, setMessage] = useState<string | null>(null);
  useTauriEvent<string>("toast", setMessage);
  useEffect(() => {
    if (!message) return;
    const t = setTimeout(() => setMessage(null), duration);
    return () => clearTimeout(t);
  }, [message, duration]);
  return message;
};

export { useWindowResize } from "./useWindowResize";
export { useTheme, ThemeProvider } from "./theme";
export type { Theme } from "./theme";
