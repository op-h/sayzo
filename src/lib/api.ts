import { invoke } from "@tauri-apps/api/core";

export type Lang = "en" | "ar";
export type Status = "idle" | "loading" | "speaking" | "paused";

export interface Settings {
  autoRead: boolean;
  englishVoice: string;
  arabicVoice: string;
  speed: number;
  volume: number;
  alwaysOnTop: boolean;
  showPanel: boolean;
  hideFromCapture: boolean;
  restoreClipboard: boolean;
  shortcuts: Record<string, string>;
}

export interface Voice {
  id: string;
  name: string;
  lang: Lang;
  accent: string;
  gender: "female" | "male";
  recommended: boolean;
}

export interface Sentence {
  text: string;
  lang: Lang;
}

export interface Progress {
  status: Status;
  current: number;
  total: number;
  error: string | null;
}

export interface Snapshot extends Progress {
  sentences: Sentence[];
  source: string;
}

export interface HistoryItem {
  id: number;
  text: string;
  source: string;
  at: number;
}

export interface EngineStatus {
  ready: boolean;
  error: string | null;
}

export type ReaderAction = "pause" | "resume" | "toggle" | "stop" | "next" | "previous";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  /** Returns the shortcut actions that could not be registered */
  updateSettings: (settings: Settings) => invoke<string[]>("update_settings", { settings }),
  setAutoRead: (enabled: boolean) => invoke("set_auto_read", { enabled }),
  getVoices: () => invoke<Voice[]>("get_voices"),
  getEngineStatus: () => invoke<EngineStatus>("get_engine_status"),
  speak: (text: string, opts: { source?: string; voice?: string } = {}) =>
    invoke("speak_text", { text, source: opts.source ?? null, voice: opts.voice ?? null }),
  control: (action: ReaderAction) => invoke("reader_control", { action }),
  seek: (index: number) => invoke("reader_seek", { index }),
  getReaderState: () => invoke<Snapshot>("get_reader_state"),
  exportAudio: (text: string) => invoke<string>("export_audio", { text }),
  ocrRegion: (x: number, y: number, width: number, height: number) => invoke("ocr_region", { x, y, width, height }),
  getHistory: () => invoke<HistoryItem[]>("get_history"),
  deleteHistory: (id?: number) => invoke("delete_history", { id: id ?? null }),
  validateShortcut: (accel: string) => invoke<boolean>("validate_shortcut", { accel }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke("set_autostart", { enabled }),
  getVersion: () => invoke<string>("get_app_version"),
  exit: () => invoke("exit_app"),
  openAuthorPage: () => invoke("open_author_page"),
  setWindowHeight: (height: number) => invoke("set_window_height", { height }),
  openDashboard: () => invoke("open_dashboard"),
  openScreenReader: () => invoke("open_ocr_overlay"),
};

export const SHORTCUT_LABELS: Record<string, { name: string; description: string }> = {
  read_selection: { name: "Read selection", description: "Read the text selected in any app. Press again with nothing selected to stop." },
  toggle_active: { name: "Toggle auto-read", description: "Turn reading-on-select on or off" },
  pause_resume: { name: "Pause / resume", description: "Pause or continue the current reading" },
  stop: { name: "Stop", description: "Stop reading" },
  next: { name: "Next sentence", description: "Skip ahead one sentence" },
  previous: { name: "Previous sentence", description: "Go back one sentence" },
  ocr: { name: "Read screen area", description: "Drag around text in images, videos or scanned PDFs to hear it" },
  toggle_window: { name: "Show / hide bar", description: "Show or hide the floating bar" },
  dashboard: { name: "Dashboard", description: "Open or close this window" },
};

const KEY_NAMES: Record<string, string> = {
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  super: "Win",
  space: "Space",
  backslash: "\\",
  period: ".",
  comma: ",",
  slash: "/",
  semicolon: ";",
  quote: "'",
  bracketleft: "[",
  bracketright: "]",
  minus: "-",
  equal: "=",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  enter: "Enter",
};

export const formatShortcut = (accel?: string) =>
  (accel ?? "")
    .split("+")
    .filter(Boolean)
    .map((k) => KEY_NAMES[k.toLowerCase()] ?? k.toUpperCase());
