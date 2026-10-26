import { useCallback, useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { api, DrawState } from "@/lib/api";
import { constrain, hits, recognize } from "./geometry";
import { CanvasStatus, Command, Element, FADE_MS, handleDrawKey, Point, SHAPES, Shape, Tool, useDrawSettings } from "./model";
import { drawElement, drawLaser, drawSpotlight, FONT, LASER_MS, LaserPoint, textSize } from "./render";

/** Holding the pen still this long turns a scribble into a clean shape */
const SNAP_HOLD_MS = 550;
const BOARD_COLORS = { none: "transparent", white: "#fbfbf8", black: "#16181d" } as const;

interface Editing {
  at: Point;
  value: string;
}

/** Full-screen transparent layer that holds the ink */
export const DrawCanvas = () => {
  const { settings, update } = useDrawSettings();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const elements = useRef<Element[]>([]);
  const undoStack = useRef<Element[][]>([]);
  const redoStack = useRef<Element[][]>([]);
  const live = useRef<Element | null>(null);
  const liveStart = useRef<Point | null>(null);
  const snapped = useRef(false);
  const holdTimer = useRef<number | undefined>(undefined);
  const laser = useRef<LaserPoint[]>([]);
  const readBox = useRef<{ a: Point; b: Point } | null>(null);
  const cursor = useRef<{ x: number; y: number } | null>(null);
  const shift = useRef(false);
  const dirty = useRef(true);
  const nextId = useRef(1);
  const [hidden, setHidden] = useState(false);
  // Opens in mouse mode: the screen stays usable until a tool is picked
  const [passthrough, setPassthrough] = useState(true);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const textRef = useRef<HTMLTextAreaElement>(null);
  // Brief hint whenever the mode changes, so it never sits on screen for long
  const [modeHint, setModeHint] = useState(false);
  useEffect(() => {
    setModeHint(true);
    const t = window.setTimeout(() => setModeHint(false), 2200);
    return () => window.clearTimeout(t);
  }, [passthrough]);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const hiddenRef = useRef(hidden);
  hiddenRef.current = hidden;

  const flash = (text: string) => {
    setNotice(text);
    window.setTimeout(() => setNotice((n) => (n === text ? null : n)), 2600);
  };

  const emitStatus = useCallback(() => {
    const status: CanvasStatus = {
      canUndo: undoStack.current.length > 0,
      canRedo: redoStack.current.length > 0,
      count: elements.current.length,
      hidden: hiddenRef.current,
    };
    emit("draw://status", status);
  }, []);

  /** Every change goes through here so undo/redo always works */
  const commit = useCallback(
    (next: Element[]) => {
      undoStack.current.push(elements.current);
      if (undoStack.current.length > 200) undoStack.current.shift();
      redoStack.current = [];
      elements.current = next;
      dirty.current = true;
      emitStatus();
    },
    [emitStatus]
  );

  const measureText = useCallback((el: Element) => {
    if (el.kind !== "text") return { w: 0, h: 0 };
    const ctx = canvasRef.current!.getContext("2d")!;
    const px = textSize(el.size);
    ctx.font = `600 ${px}px ${FONT}`;
    const lines = el.text.split("\n");
    return { w: Math.max(...lines.map((l) => ctx.measureText(l).width)), h: lines.length * px * 1.25 };
  }, []);

  // Canvas sized to the window at full pixel density
  useEffect(() => {
    const resize = () => {
      const c = canvasRef.current!;
      const dpr = window.devicePixelRatio || 1;
      c.width = Math.round(window.innerWidth * dpr);
      c.height = Math.round(window.innerHeight * dpr);
      dirty.current = true;
    };
    resize();
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);

  // Render loop: only repaints when something changed or is animating
  useEffect(() => {
    let frame = 0;
    const loop = () => {
      const now = Date.now();
      const s = settingsRef.current;
      const fading = elements.current.some((e) => e.fadeAt);
      if (fading) {
        const kept = elements.current.filter((e) => !e.fadeAt || e.fadeAt > now);
        if (kept.length !== elements.current.length) {
          elements.current = kept;
          emitStatus();
        }
      }
      laser.current = laser.current.filter((p) => now - p.t < LASER_MS);
      const animating = fading || laser.current.length > 0 || s.spotlight;
      if (dirty.current || animating) {
        dirty.current = false;
        const c = canvasRef.current!;
        const ctx = c.getContext("2d")!;
        const dpr = c.width / window.innerWidth;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
        if (!hiddenRef.current) {
          for (const el of elements.current) drawElement(ctx, el, now);
          if (live.current) drawElement(ctx, live.current, now);
        }
        drawLaser(ctx, laser.current, s.color, now);
        if (readBox.current) {
          const { a, b } = readBox.current;
          ctx.save();
          ctx.setLineDash([7, 5]);
          ctx.strokeStyle = "#3b82f6";
          ctx.fillStyle = "rgba(59,130,246,0.12)";
          ctx.lineWidth = 2;
          const [x, y, w, h] = [Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(b.x - a.x), Math.abs(b.y - a.y)];
          ctx.fillRect(x, y, w, h);
          ctx.strokeRect(x, y, w, h);
          ctx.restore();
        }
        if (s.tool === "eraser" && cursor.current) {
          ctx.save();
          ctx.strokeStyle = s.board === "white" ? "#111827" : "#ffffff";
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.arc(cursor.current.x, cursor.current.y, eraserRadius(s.size), 0, Math.PI * 2);
          ctx.stroke();
          ctx.restore();
        }
        if (s.spotlight && cursor.current) drawSpotlight(ctx, window.innerWidth, window.innerHeight, cursor.current);
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [emitStatus]);

  useEffect(() => {
    dirty.current = true;
  }, [settings, hidden]);

  const undo = useCallback(() => {
    const prev = undoStack.current.pop();
    if (!prev) return;
    redoStack.current.push(elements.current);
    elements.current = prev;
    dirty.current = true;
    emitStatus();
  }, [emitStatus]);

  const redo = useCallback(() => {
    const next = redoStack.current.pop();
    if (!next) return;
    undoStack.current.push(elements.current);
    elements.current = next;
    dirty.current = true;
    emitStatus();
  }, [emitStatus]);

  const snapshot = useCallback(async () => {
    try {
      const s = settingsRef.current;
      const png = await api.drawCaptureScreen();
      const shot = await createImageBitmap(new Blob([png], { type: "image/png" }));
      const off = document.createElement("canvas");
      off.width = shot.width;
      off.height = shot.height;
      const ctx = off.getContext("2d")!;
      if (s.board === "none") ctx.drawImage(shot, 0, 0);
      else {
        ctx.fillStyle = BOARD_COLORS[s.board];
        ctx.fillRect(0, 0, off.width, off.height);
      }
      ctx.scale(off.width / window.innerWidth, off.height / window.innerHeight);
      const now = Date.now();
      if (!hiddenRef.current) for (const el of elements.current) drawElement(ctx, el, now);
      const blob = await new Promise<Blob | null>((r) => off.toBlob(r, "image/png"));
      if (!blob) throw new Error("couldn't create the image");
      const path = await api.drawSaveImage(new Uint8Array(await blob.arrayBuffer()));
      flash(`Screenshot copied and saved to ${path.split(/[\\/]/).slice(-2).join("\\")}`);
    } catch (e) {
      flash(`Screenshot failed: ${e}`);
    }
  }, []);

  // Commands from the toolbar (and from this window's own shortcuts)
  useEffect(() => {
    const un = listen<Command>("draw://command", (e) => {
      switch (e.payload) {
        case "undo":
          return undo();
        case "redo":
          return redo();
        case "clear":
          if (elements.current.length) commit([]);
          return;
        case "toggle-ink":
          setHidden((h) => !h);
          setTimeout(emitStatus, 0);
          return;
        case "snapshot":
          return void snapshot();
      }
    });
    const st = listen<DrawState>("draw://state", (e) => setPassthrough(e.payload.passthrough));
    const ask = listen("draw://status-request", emitStatus);
    api.drawGetState().then((s) => setPassthrough(s.passthrough));
    emitStatus();
    return () => {
      un.then((f) => f());
      st.then((f) => f());
      ask.then((f) => f());
    };
  }, [undo, redo, commit, snapshot, emitStatus]);

  // Keyboard shortcuts (not while typing text)
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      shift.current = e.shiftKey;
      // While a text box is open, keys are text, never tool shortcuts
      if (editing) {
        if (document.activeElement !== textRef.current) textRef.current?.focus();
        return;
      }
      const pickTool = (tool: Tool) => {
        // Pressing the shortcut of the tool in use goes back to the mouse
        if (settingsRef.current.tool === tool) api.drawSetPassthrough(true);
        else update({ tool });
      };
      if (handleDrawKey(e, settingsRef.current, update, { exit: () => api.drawClose(), mouseMode: () => api.drawSetPassthrough(true), pickTool })) {
        e.preventDefault();
      }
    };
    const up = (e: KeyboardEvent) => (shift.current = e.shiftKey);
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, [editing, update]);

  const pointOf = (e: React.PointerEvent | PointerEvent): Point => ({
    x: e.clientX,
    y: e.clientY,
    // Mice report 0.5 while pressed; pens report real pressure
    p: e.pointerType === "pen" ? Math.max(0.05, e.pressure) : 0.5,
  });

  const fadeAt = () => (settingsRef.current.fade ? Date.now() + FADE_MS : undefined);

  const eraseAt = (p: Point) => {
    const r = eraserRadius(settingsRef.current.size);
    const kept = elements.current.filter((el) => !hits(el, p, r, measureText));
    if (kept.length !== elements.current.length) {
      // One eraser drag is one undo step
      if (!erasing.current) {
        erasing.current = true;
        commit(kept);
      } else {
        elements.current = kept;
        dirty.current = true;
        emitStatus();
      }
    }
  };
  const erasing = useRef(false);

  const onDown = (e: React.PointerEvent) => {
    if (e.button !== 0 || editing) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const s = settingsRef.current;
    const p = pointOf(e);
    liveStart.current = p;
    snapped.current = false;
    erasing.current = false;
    switch (s.tool) {
      case "pen":
      case "highlighter":
        live.current = { id: nextId.current++, kind: s.tool, color: s.color, size: s.size, points: [p], fadeAt: fadeAt() };
        break;
      case "line":
      case "arrow":
      case "rect":
      case "ellipse":
        live.current = { id: nextId.current++, kind: s.tool, color: s.color, size: s.size, a: p, b: p, fadeAt: fadeAt() };
        break;
      case "laser":
        laser.current.push({ x: p.x, y: p.y, t: Date.now() });
        break;
      case "eraser":
        eraseAt(p);
        break;
      case "read":
        readBox.current = { a: p, b: p };
        break;
      case "text":
        // Opened on release: opening on press lets the rest of the click steal focus
        e.preventDefault();
        pendingText.current = p;
        break;
      case "number": {
        const n = Math.max(0, ...elements.current.map((el) => (el.kind === "number" ? el.n : 0))) + 1;
        commit([...elements.current, { id: nextId.current++, kind: "number", color: s.color, size: s.size, at: p, n, fadeAt: fadeAt() }]);
        break;
      }
    }
    dirty.current = true;
  };

  const onMove = (e: React.PointerEvent) => {
    const p = pointOf(e);
    cursor.current = { x: p.x, y: p.y };
    const s = settingsRef.current;
    if (s.tool === "eraser" || s.spotlight) dirty.current = true;
    // The laser follows the cursor even without pressing, like a real pointer
    if (s.tool === "laser") laser.current.push({ x: p.x, y: p.y, t: Date.now() });
    if (!(e.buttons & 1)) return;
    const el = live.current;
    if (s.tool === "eraser") return eraseAt(p);
    if (s.tool === "read" && readBox.current) {
      readBox.current.b = p;
      dirty.current = true;
      return;
    }
    if (!el) return;
    if (el.kind === "pen" || el.kind === "highlighter") {
      if (snapped.current) return;
      const coalesced = (e.nativeEvent.getCoalescedEvents?.() ?? []).map(pointOf);
      el.points.push(...(coalesced.length ? coalesced : [p]));
      // Holding still converts the scribble into a clean shape
      window.clearTimeout(holdTimer.current);
      if (el.kind === "pen") {
        holdTimer.current = window.setTimeout(() => {
          const cur = live.current;
          if (!cur || cur.kind !== "pen") return;
          const shape = recognize(cur.points);
          if (shape) {
            live.current = { id: cur.id, kind: shape.kind, color: cur.color, size: cur.size, a: shape.a, b: shape.b, fadeAt: cur.fadeAt };
            snapped.current = true;
            dirty.current = true;
          }
        }, SNAP_HOLD_MS);
      }
    } else if ((SHAPES as string[]).includes(el.kind) && "a" in el) {
      el.b = shift.current || e.shiftKey ? constrain(el.kind as Shape, el.a, p) : p;
    }
    dirty.current = true;
  };

  const pendingText = useRef<Point | null>(null);

  const onUp = () => {
    window.clearTimeout(holdTimer.current);
    if (pendingText.current) {
      const at = pendingText.current;
      pendingText.current = null;
      setEditing({ at, value: "" });
      window.setTimeout(() => textRef.current?.focus(), 30);
    }
    const el = live.current;
    live.current = null;
    erasing.current = false;
    if (el) {
      const tiny = "a" in el && Math.hypot(el.b.x - el.a.x, el.b.y - el.a.y) < 3;
      if (!tiny) commit([...elements.current, el]);
    }
    const box = readBox.current;
    readBox.current = null;
    if (box) {
      const [x, y, w, h] = [Math.min(box.a.x, box.b.x), Math.min(box.a.y, box.b.y), Math.abs(box.b.x - box.a.x), Math.abs(box.b.y - box.a.y)];
      if (w > 8 && h > 8) {
        flash("Reading…");
        api.drawReadRegion(x, y, w, h).catch(() => {});
      }
    }
    dirty.current = true;
  };

  // Activating the canvas window can move focus away right after the box
  // opens; keep it focused so typing goes into the box
  useEffect(() => {
    if (!editing) return;
    const id = requestAnimationFrame(() => textRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [editing?.at]);

  // Switching tool or to the mouse closes the box (keeping what was typed)
  useEffect(() => {
    if ((settings.tool !== "text" || passthrough) && editing) finishText();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.tool, passthrough]);

  const finishText = (cancel = false) => {
    const t = editing;
    setEditing(null);
    if (!t || cancel || !t.value.trim()) return;
    const s = settingsRef.current;
    commit([...elements.current, { id: nextId.current++, kind: "text", color: s.color, size: s.size, at: t.at, text: t.value, fadeAt: fadeAt() }]);
  };

  const cursorStyle =
    passthrough ? "default" : settings.tool === "text" ? "text" : settings.tool === "eraser" || settings.tool === "laser" ? "none" : "crosshair";

  return (
    <div
      className="fixed inset-0 select-none overflow-hidden"
      style={{ background: BOARD_COLORS[settings.board], cursor: cursorStyle }}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
      onPointerLeave={() => {
        cursor.current = null;
        dirty.current = true;
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" />
      {editing && (
        <textarea
          ref={textRef}
          autoFocus
          dir="auto"
          value={editing.value}
          onChange={(e) => setEditing({ ...editing, value: e.target.value })}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Escape") finishText(true);
            else if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              finishText();
            }
          }}
          onBlur={() => {
            // An empty box stays open and takes focus back; a filled one is placed
            if (!editing.value.trim()) {
              window.setTimeout(() => document.hasFocus() && textRef.current?.focus(), 0);
              return;
            }
            finishText();
          }}
          onPointerDown={(e) => e.stopPropagation()}
          rows={Math.max(1, editing.value.split("\n").length)}
          placeholder="Type, Enter to place"
          className="absolute resize-none overflow-hidden border-0 bg-transparent p-0 outline-none placeholder:opacity-60"
          style={{
            left: editing.at.x,
            top: editing.at.y,
            color: settings.color,
            font: `600 ${textSize(settings.size)}px ${FONT}`,
            lineHeight: 1.25,
            minWidth: 220,
            width: Math.max(220, Math.max(...editing.value.split("\n").map((l) => l.length)) * textSize(settings.size) * 0.62),
            caretColor: settings.color,
            textShadow: "0 0 3px rgba(0,0,0,0.5)",
          }}
        />
      )}
      {notice && (
        <div className="pointer-events-none absolute left-1/2 top-6 -translate-x-1/2 rounded-xl border border-white/10 bg-neutral-900/90 px-4 py-2 text-sm text-white shadow-lg">
          {notice}
        </div>
      )}
      {modeHint && (
        <div className="pointer-events-none absolute left-1/2 top-4 -translate-x-1/2 rounded-full bg-neutral-900/85 px-3 py-1 text-xs text-white/85 shadow">
          {passthrough ? "Mouse mode · your apps work normally, drawings stay" : "Drawing · click a tool again or press M for the mouse"}
        </div>
      )}
    </div>
  );
};

const eraserRadius = (size: number) => 8 + size * 2.5;
