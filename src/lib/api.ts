import { invoke } from "@tauri-apps/api/core";

export type Lang = "en" | "ar";
export type Status = "idle" | "loading" | "speaking" | "paused";

export interface Settings {
  autoRead: boolean;
  englishVoice: string;
  arabicVoice: string;
  /** Voice for every other language, keyed by ISO 639-1 code */
  languageVoices: Record<string, string>;
  speed: number;
  volume: number;
  alwaysOnTop: boolean;
  showPanel: boolean;
  hideFromCapture: boolean;
  restoreClipboard: boolean;
  /** The floating bar is minimized to a small pill */
  compactBar: boolean;
  shortcuts: Record<string, string>;
}

export interface Voice {
  id: string;
  name: string;
  /** ISO 639-1 code, or "und" when an imported voice's language is unknown */
  language: string;
  accent: string;
  gender: string;
  recommended: boolean;
  imported: boolean;
}

export const LANGUAGE_NAMES: Record<string, string> = {
  en: "English", ar: "Arabic", es: "Spanish", fr: "French", it: "Italian", pt: "Portuguese", hi: "Hindi", de: "German",
  ru: "Russian", uk: "Ukrainian", tr: "Turkish", fa: "Persian", ur: "Urdu", ku: "Kurdish", zh: "Chinese", ja: "Japanese",
  ko: "Korean", nl: "Dutch", pl: "Polish", sv: "Swedish", nb: "Norwegian", da: "Danish", fi: "Finnish", el: "Greek",
  he: "Hebrew", cs: "Czech", hu: "Hungarian", ro: "Romanian", vi: "Vietnamese", id: "Indonesian", bn: "Bengali",
  th: "Thai", ca: "Catalan", sw: "Swahili", ka: "Georgian", kk: "Kazakh", sr: "Serbian", sk: "Slovak", sl: "Slovenian",
  is: "Icelandic", cy: "Welsh", ne: "Nepali", ml: "Malayalam", te: "Telugu", lb: "Luxembourgish", lv: "Latvian", et: "Estonian",
};

export const languageName = (code: string) => LANGUAGE_NAMES[code] ?? (code === "und" ? "Unknown language" : code.toUpperCase());

/** The voice currently chosen for a language */
export const chosenVoice = (s: Settings, lang: string) =>
  lang === "en" ? s.englishVoice : lang === "ar" ? s.arabicVoice : s.languageVoices[lang];

/** Settings patch that makes `id` the voice for `lang` */
export const chooseVoice = (s: Settings, lang: string, id: string): Partial<Settings> =>
  lang === "en" ? { englishVoice: id } : lang === "ar" ? { arabicVoice: id } : { languageVoices: { ...s.languageVoices, [lang]: id } };

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

export interface DrawState {
  open: boolean;
  /** Mouse mode: clicks go to the apps below while the ink stays visible */
  passthrough: boolean;
}

export type ReaderAction = "pause" | "resume" | "toggle" | "stop" | "next" | "previous";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  /** Returns the shortcut actions that could not be registered */
  updateSettings: (settings: Settings) => invoke<string[]>("update_settings", { settings }),
  setAutoRead: (enabled: boolean) => invoke("set_auto_read", { enabled }),
  getVoices: () => invoke<Voice[]>("get_voices"),
  /** Imports a voice package (archive, folder or .onnx) and returns the voices it added */
  importVoice: (path: string) => invoke<Voice[]>("import_voice", { path }),
  removeVoice: (id: string) => invoke("remove_voice", { id }),
  setVoiceLanguage: (id: string, language: string) => invoke("set_voice_language", { id, language }),
  openVoiceLibrary: () => invoke("open_voice_library"),
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
  openDashboard: () => invoke("open_dashboard_command"),
  openScreenReader: () => invoke("open_ocr_overlay_command"),
  setBarWidth: (width: number) => invoke("set_bar_width", { width }),
  drawOpen: () => invoke("draw_open"),
  drawClose: () => invoke("draw_close"),
  /** Opens drawing, or closes it when it is already open */
  drawToggle: () => invoke("draw_toggle"),
  drawSetPassthrough: (on: boolean) => invoke("draw_set_passthrough", { on }),
  drawGetState: () => invoke<DrawState>("draw_get_state"),
  /** Resizes the toolbar; returns which way it grew so popovers open on the side with room */
  drawToolbarSize: (width: number, height: number) => invoke<"up" | "down">("draw_toolbar_size", { width, height }),
  /** The screen behind the drawing, as PNG bytes */
  drawCaptureScreen: () => invoke<ArrayBuffer>("draw_capture_screen"),
  /** Saves PNG bytes to Pictures\Sayzo and copies them; returns the file path */
  drawSaveImage: (png: Uint8Array) => invoke<string>("draw_save_image", png),
  drawReadRegion: (x: number, y: number, width: number, height: number) => invoke("draw_read_region", { x, y, width, height }),
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
  draw: { name: "Draw on screen", description: "Start drawing, or switch between drawing and using your apps (ink stays)" },
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
