import { useEffect, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";

export type Tool = "pen" | "highlighter" | "laser" | "line" | "arrow" | "rect" | "ellipse" | "text" | "number" | "eraser" | "read";
export type Shape = "line" | "arrow" | "rect" | "ellipse";
export type Board = "none" | "white" | "black";

export const SHAPES: Shape[] = ["line", "arrow", "rect", "ellipse"];

export interface Point {
  x: number;
  y: number;
  /** Pen pressure 0..1 (0.5 for a mouse) */
  p: number;
}

interface Base {
  id: number;
  color: string;
  size: number;
  /** Time (ms) when fading ink should be gone; undefined = permanent */
  fadeAt?: number;
}

export type Element =
  | (Base & { kind: "pen" | "highlighter"; points: Point[] })
  | (Base & { kind: Shape; a: Point; b: Point })
  | (Base & { kind: "text"; at: Point; text: string })
  | (Base & { kind: "number"; at: Point; n: number });

/** Settings shared by the canvas and the voice bar's drawing row */
export interface DrawSettings {
  tool: Tool;
  /** Last shape picked, shown on the shapes button */
  shape: Shape;
  color: string;
  size: number;
  board: Board;
  fade: boolean;
  spotlight: boolean;
}

export const COLORS = [
  "#ef4444", "#f97316", "#facc15", "#22c55e", "#06b6d4", "#3b82f6",
  "#8b5cf6", "#ec4899", "#ffffff", "#9ca3af", "#111827", "#a16207",
];

export const SIZES = [2, 4, 8, 14];

export const DEFAULT_SETTINGS: DrawSettings = {
  tool: "pen",
  shape: "arrow",
  color: COLORS[0],
  size: 4,
  board: "none",
  fade: false,
  spotlight: false,
};

/** How long fading ink stays before it disappears */
export const FADE_MS = 3500;

export type Command = "undo" | "redo" | "clear" | "toggle-ink" | "snapshot";

export interface CanvasStatus {
  canUndo: boolean;
  canRedo: boolean;
  count: number;
  hidden: boolean;
}

const SETTINGS_KEY = "sayzo-draw";

/**
 * The bar's last pen, so a new drawing starts with the same tool, colour and
 * size. Board, fading ink and spotlight start off every time.
 */
const saved = (): DrawSettings => {
  try {
    const { tool, shape, color, size } = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "{}");
    return { ...DEFAULT_SETTINGS, ...(tool && { tool }), ...(shape && { shape }), ...(color && { color }), ...(size && { size }) };
  } catch {
    return DEFAULT_SETTINGS;
  }
};

/**
 * The canvas and the voice bar each keep a copy of the settings; whoever
 * changes them broadcasts the new value so the other window follows. The bar
 * outlives every drawing, so it's the owner: it remembers the settings and
 * answers a new canvas's "hello" with them (only the owner answers, or a
 * fresh canvas would reset the bar to the defaults).
 */
export const useDrawSettings = (owner = false) => {
  const [settings, setSettings] = useState<DrawSettings>(() => (owner ? saved() : DEFAULT_SETTINGS));
  useEffect(() => {
    const un = listen<DrawSettings>("draw://settings", (e) => setSettings(e.payload));
    const hello = owner ? listen("draw://hello", () => setSettings((s) => (emit("draw://settings", s), s))) : null;
    if (!owner) emit("draw://hello");
    return () => {
      un.then((f) => f());
      hello?.then((f) => f());
    };
  }, [owner]);
  useEffect(() => {
    if (!owner) return;
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    } catch {
      // Private storage unavailable: settings just aren't remembered
    }
  }, [owner, settings]);
  const update = (patch: Partial<DrawSettings>) =>
    setSettings((s) => {
      const next = { ...s, ...patch };
      if (patch.tool && (SHAPES as string[]).includes(patch.tool)) next.shape = patch.tool as Shape;
      emit("draw://settings", next);
      return next;
    });
  return { settings, update };
};

export const sendCommand = (cmd: Command) => emit("draw://command", cmd);

/**
 * Single-key shortcuts while drawing, by physical key (KeyboardEvent.code),
 * so they work with any keyboard layout: an Arabic layout types "ق" on the R
 * key, but its code is still "KeyR".
 */
export const KEY_TOOLS: Record<string, Tool> = {
  KeyP: "pen",
  KeyH: "highlighter",
  KeyK: "laser",
  KeyL: "line",
  KeyA: "arrow",
  KeyR: "rect",
  KeyO: "ellipse",
  KeyT: "text",
  KeyN: "number",
  KeyE: "eraser",
  KeyS: "read",
};

export const TOOL_LABELS: Record<Tool, string> = {
  pen: "Pen (P)",
  highlighter: "Highlighter (H)",
  laser: "Laser pointer: fades as you move (K)",
  line: "Line (L)",
  arrow: "Arrow (A)",
  rect: "Rectangle (R)",
  ellipse: "Ellipse (O)",
  text: "Text (T)",
  number: "Numbered steps (N)",
  eraser: "Eraser: removes whole strokes (E)",
  read: "Read aloud: draw a box around text (S)",
};

/**
 * Keyboard shortcuts shared by the canvas and the voice bar. Returns true when the
 * key was handled. Typing in the text tool must not trigger these.
 */
export const handleDrawKey = (
  e: KeyboardEvent,
  settings: DrawSettings,
  update: (p: Partial<DrawSettings>) => void,
  actions: { exit: () => void; mouseMode: () => void; pickTool: (tool: Tool) => void }
): boolean => {
  // Physical keys, so shortcuts work whatever keyboard language is active
  const code = e.code;
  if (e.ctrlKey && code === "KeyZ") return sendCommand(e.shiftKey ? "redo" : "undo"), true;
  if (e.ctrlKey && code === "KeyY") return sendCommand("redo"), true;
  if (e.ctrlKey && code === "KeyC" && e.shiftKey) return sendCommand("snapshot"), true;
  if (e.ctrlKey || e.altKey || e.metaKey) return false;
  if (code === "Escape") return actions.exit(), true;
  if (code === "Delete" || code === "Backspace") return sendCommand("clear"), true;
  if (code === "KeyM") return actions.mouseMode(), true;
  if (code === "KeyW") return update({ board: settings.board === "white" ? "none" : "white" }), true;
  if (code === "KeyB") return update({ board: settings.board === "black" ? "none" : "black" }), true;
  if (code === "KeyF") return update({ fade: !settings.fade }), true;
  if (code === "KeyG") return update({ spotlight: !settings.spotlight }), true;
  if (code === "KeyV") return sendCommand("toggle-ink"), true;
  const digit = /^(?:Digit|Numpad)([1-9])$/.exec(code);
  if (digit) return update({ color: COLORS[Number(digit[1]) - 1] }), true;
  if (code === "BracketLeft" || code === "BracketRight") {
    const i = SIZES.indexOf(settings.size);
    return update({ size: SIZES[Math.min(SIZES.length - 1, Math.max(0, i + (code === "BracketRight" ? 1 : -1)))] }), true;
  }
  // Same as clicking the tool: starts drawing, or returns to the mouse if it's already in use
  const tool = KEY_TOOLS[code];
  if (tool) return actions.pickTool(tool), true;
  return false;
};
