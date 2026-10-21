import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ScanTextIcon } from "lucide-react";
import { api } from "@/lib/api";

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Full-screen layer for dragging a box around text that can't be selected */
export const ScreenAreaPicker = () => {
  const start = useRef<{ x: number; y: number } | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    document.documentElement.style.setProperty("--cursor-type", "crosshair");
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") getCurrentWindow().close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const toRect = (a: { x: number; y: number }, b: { x: number; y: number }): Rect => ({
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(a.x - b.x),
    h: Math.abs(a.y - b.y),
  });

  return (
    <div
      className="fixed inset-0 select-none"
      style={{ background: rect ? "transparent" : "rgba(0,0,0,0.35)" }}
      onMouseDown={(e) => {
        if (busy || e.button !== 0) return;
        start.current = { x: e.clientX, y: e.clientY };
        setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
      }}
      onMouseMove={(e) => {
        if (start.current) setRect(toRect(start.current, { x: e.clientX, y: e.clientY }));
      }}
      onMouseUp={() => {
        const r = rect;
        start.current = null;
        if (!r || r.w < 8 || r.h < 8) {
          setRect(null);
          return;
        }
        setBusy(true);
        // Rust hides this window, captures the area underneath and reads it
        api.ocrRegion(r.x, r.y, r.w, r.h).catch(() => getCurrentWindow().close());
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        getCurrentWindow().close();
      }}
    >
      {rect && (
        <div
          className="absolute border-2 border-white/90 rounded-sm"
          style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h, boxShadow: "0 0 0 9999px rgba(0,0,0,0.35)" }}
        />
      )}
      {!rect && (
        <div className="absolute top-14 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-xl bg-neutral-900/90 text-white text-sm px-4 py-2 shadow-lg">
          <ScanTextIcon className="size-4" />
          Drag around the text you want to hear
          <span className="text-white/60 text-xs ml-2">Esc to cancel</span>
        </div>
      )}
    </div>
  );
};
