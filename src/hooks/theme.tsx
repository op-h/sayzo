import { createContext, useContext, useEffect, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export type Theme = "dark" | "light" | "system";

const THEME_KEY = "sayzo-theme";
/** --background of each theme, for the parts of a window the page hasn't painted */
const UNPAINTED: Record<"light" | "dark", [number, number, number, number]> = { light: [255, 255, 255, 255], dark: [10, 10, 10, 255] };
const TRANSPARENCY_KEY = "sayzo-transparency";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  transparency: number;
  setTransparency: (value: number) => void;
}

const ThemeContext = createContext<ThemeState>({
  theme: "system",
  setTheme: () => null,
  transparency: 10,
  setTransparency: () => null,
});

const read = (key: string) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

const write = (key: string, value: string) => {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable; the choice then lasts for this session only
  }
};

/** Theme and glass transparency, shared between windows through localStorage like Pluely */
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => (read(THEME_KEY) as Theme) || "system");
  const [transparency, setTransparencyState] = useState(() => Number(read(TRANSPARENCY_KEY) ?? 10));

  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === THEME_KEY && e.newValue) setThemeState(e.newValue as Theme);
      if (e.key === TRANSPARENCY_KEY && e.newValue) setTransparencyState(Number(e.newValue));
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const mode = theme === "system" ? (media.matches ? "dark" : "light") : theme;
      root.classList.remove("light", "dark");
      root.classList.add(mode);
      // The dashboard's page is transparent, so whenever WebView2 shows it
      // before painting (e.g. restoring it after a long minimize) the window
      // was a blank white sheet. Match the window behind it to the theme
      const win = getCurrentWebviewWindow();
      if (win.label === "dashboard") {
        win.setBackgroundColor(UNPAINTED[mode]).catch(() => {});
      }
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--opacity", ((100 - transparency) / 100).toString());
    root.style.setProperty("--backdrop-blur", transparency > 0 ? "blur(12px)" : "none");
  }, [transparency]);

  const value: ThemeState = {
    theme,
    setTheme: (t) => {
      write(THEME_KEY, t);
      setThemeState(t);
    },
    transparency,
    setTransparency: (v) => {
      write(TRANSPARENCY_KEY, String(v));
      setTransparencyState(v);
    },
  };

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export const useTheme = () => useContext(ThemeContext);
