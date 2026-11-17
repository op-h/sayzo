import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AudioLinesIcon,
  Loader2,
  Minimize2Icon,
  MousePointerClickIcon,
  MousePointer2Icon,
  PenLineIcon,
  PauseIcon,
  PlayIcon,
  ScanTextIcon,
  SkipBackIcon,
  SkipForwardIcon,
  SparklesIcon,
  SquareIcon,
  InfoIcon,
} from "lucide-react";
import { Button, Card, Grip, Input, Kbd, Popover, PopoverAnchor, PopoverContent, PopoverTrigger, Visualizer } from "@/components";
import { useEngineStatus, useReader, useSettings, useToast, useWindowResize } from "@/hooks";
import { emit, listen } from "@tauri-apps/api/event";
import { api, DrawState } from "@/lib/api";
import { cn } from "@/lib/utils";
import { CanvasStatus, handleDrawKey, Tool, useDrawSettings } from "@/draw/model";
import { ReadingPanel } from "./ReadingPanel";
import { QuickSettings } from "./QuickSettings";
import { DrawRow } from "./DrawRow";
import { Group, Swatch, Tile } from "./parts";

const FULL_WIDTH = 660;
/** Padding (10 px each side) plus the panel's 1 px border, around the drawing row */
const ROW_CHROME = 22;
const NO_INK: CanvasStatus = { canUndo: false, canRedo: false, count: 0, hidden: false };

/** Lets any empty part of the bar drag the window (buttons keep working) */
const dragFromBlank = (e: React.MouseEvent) => {
  if (e.button === 0 && e.target === e.currentTarget) getCurrentWindow().startDragging();
};

const isPopoverOpen = () => document.querySelector("[data-radix-popper-content-wrapper]") !== null;

export const Bar = () => {
  const { settings, update } = useSettings();
  const reader = useReader();
  const engine = useEngineStatus();
  const toast = useToast();
  const [text, setText] = useState("");
  const [panelOpen, setPanelOpen] = useState(false);
  const previousStatus = useRef(reader.status);

  const busy = reader.status !== "idle";
  const paused = reader.status === "paused";
  const current = reader.sentences[reader.current];

  // Open the reading panel when a new read starts, if the user asked for that
  useEffect(() => {
    if (previousStatus.current === "idle" && reader.status === "loading" && settings?.showPanel) {
      setPanelOpen(true);
    }
    previousStatus.current = reader.status;
  }, [reader.status, settings?.showPanel]);

  const speakTyped = async () => {
    if (!text.trim()) return;
    await api.speak(text, { source: "typed" });
    setText("");
  };

  const autoRead = settings?.autoRead ?? false;
  const compact = settings?.compactBar ?? false;

  // Drawing: its tools appear as the bar's second row
  const [drawing, setDrawing] = useState(false);
  const [passthrough, setPassthrough] = useState(true);
  const [status, setStatus] = useState<CanvasStatus>(NO_INK);
  const draw = useDrawSettings(true);
  useEffect(() => {
    const apply = (s: DrawState) => {
      setDrawing(s.open);
      setPassthrough(s.passthrough);
    };
    api.drawGetState().then(apply).catch(() => {});
    const a = listen<DrawState>("draw://state", (e) => apply(e.payload));
    const b = listen<CanvasStatus>("draw://status", (e) => setStatus(e.payload));
    emit("draw://status-request");
    return () => {
      a.then((f) => f());
      b.then((f) => f());
    };
  }, []);
  // Each drawing starts empty, on the plain screen
  useEffect(() => {
    if (drawing) return;
    setStatus(NO_INK);
    if (draw.settings.board !== "none" || draw.settings.fade || draw.settings.spotlight) {
      draw.update({ board: "none", fade: false, spotlight: false });
    }
  }, [drawing]);

  /** Picking a tool starts drawing; clicking the active tool again goes back to the mouse */
  const pick = (tool: Tool) => {
    if (!passthrough && draw.settings.tool === tool) {
      api.drawSetPassthrough(true);
      return;
    }
    draw.update({ tool });
    if (passthrough) api.drawSetPassthrough(false);
  };

  // The drawing keys (P, H, 1–9, Esc…) also work while the bar has focus,
  // except while typing in its text box. Delete and Backspace are left to
  // the canvas, so a stray key after clicking a button never wipes the drawing.
  const onDrawKey = useRef<(e: KeyboardEvent) => void>(() => {});
  onDrawKey.current = (e) => {
    if (!drawing || isPopoverOpen()) return;
    if ((e.target as HTMLElement | null)?.closest("input, textarea, [contenteditable]")) return;
    if (e.code === "Delete" || e.code === "Backspace") return;
    if (handleDrawKey(e, draw.settings, draw.update, { exit: () => api.drawClose(), mouseMode: () => api.drawSetPassthrough(!passthrough), pickTool: pick })) {
      e.preventDefault();
    }
  };
  useEffect(() => {
    const down = (e: KeyboardEvent) => onDrawKey.current(e);
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, []);

  // Near the bottom of the screen, the drawing row and popovers open upwards
  const [up, setUp] = useState(false);
  const upRef = useRef(up);
  upRef.current = up;
  useEffect(() => {
    let timer = 0;
    const check = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        // Only while the window hugs the panel: our own growing moves it too
        if (isPopoverOpen()) return;
        api.barGrowsUp(upRef.current).then(setUp).catch(() => {});
      }, 150);
    };
    check();
    const un = getCurrentWindow().onMoved(check);
    return () => {
      window.clearTimeout(timer);
      un.then((f) => f());
    };
  }, []);
  const side = up ? "top" : "bottom";

  // The window's height always hugs the panel (plus room for an open popover)
  const panelRef = useRef<HTMLDivElement>(null);
  useWindowResize(panelRef, up, compact ? "pill" : drawing ? "drawing" : "bar");

  // Full bar: fixed width, or as wide as the drawing tools. Minimized: the
  // window hugs the pill exactly, so the spacing is the same on every side.
  const drawRowRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!settings) return;
    const el = compact ? panelRef.current : drawing ? drawRowRef.current : null;
    if (!el) {
      api.setBarWidth(FULL_WIDTH).catch(() => {});
      return;
    }
    let last = 0;
    const fit = () => {
      const w = compact ? Math.ceil(el.offsetWidth) : Math.max(FULL_WIDTH, Math.ceil(el.offsetWidth) + ROW_CHROME);
      if (w && w !== last) {
        last = w;
        api.setBarWidth(w).catch(() => {});
      }
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [compact, drawing, settings !== null]);

  const penTitle = drawing ? "Stop drawing (Ctrl+Alt+D switches to the mouse)" : "Draw on screen (Ctrl+Alt+D)";

  const setCompact = (on: boolean) => {
    if (on) setPanelOpen(false);
    update({ compactBar: on });
  };

  // Pulled in slightly so the dots sit as far from the edge as the first button does
  const grip = <Grip className="-mr-1.5" />;

  // While drawing, the whole panel takes a faint blue edge
  const panelClass = cn("rounded-[22px] shadow-lg shadow-black/25 transition-colors duration-300", drawing && "border-blue-400/40");

  const autoReadButton = (
    <Group>
      <Tile
        tone="auto"
        active={autoRead}
        icon={MousePointerClickIcon}
        title={autoRead ? "Auto-read is on: selecting text reads it aloud. Click to turn off." : "Turn on auto-read: any text you select is read aloud"}
        onClick={() => api.setAutoRead(!autoRead)}
      />
    </Group>
  );

  const playback = (full: boolean) => (
    <Group>
      {full && <Tile title="Previous sentence" icon={SkipBackIcon} onClick={() => api.control("previous")} />}
      <Tile active title={paused ? "Resume" : "Pause"} icon={paused ? PlayIcon : PauseIcon} onClick={() => api.control("toggle")} />
      {full && <Tile title="Next sentence" icon={SkipForwardIcon} onClick={() => api.control("next")} />}
      <Tile title="Stop" icon={SquareIcon} onClick={() => api.control("stop")} />
    </Group>
  );

  if (compact) {
    return (
      <div className={cn("w-screen h-screen flex overflow-hidden justify-center", up ? "items-end" : "items-start")}>
        <Card ref={panelRef} className={cn("w-fit shrink-0 flex flex-row items-center gap-2 p-2.5 whitespace-nowrap", panelClass)} onMouseDown={dragFromBlank}>
          <Group>
            <Tile active title="Show the full bar" onClick={() => setCompact(false)}>
              {busy && reader.status !== "loading" ? <Visualizer active={!paused} paused={paused} /> : <AudioLinesIcon className="size-4" />}
            </Tile>
          </Group>
          {busy ? (
            playback(false)
          ) : (
            <Group>
              <Tile
                tone="auto"
                active={autoRead}
                icon={MousePointerClickIcon}
                title={autoRead ? "Auto-read is on. Click to turn off." : "Turn on auto-read"}
                onClick={() => api.setAutoRead(!autoRead)}
              />
              <Tile tone="draw" active={drawing} icon={PenLineIcon} title={penTitle} onClick={() => api.drawToggle()} />
              {drawing && (
                <Tile
                  active={passthrough}
                  title={passthrough ? "Mouse mode: click to draw again (M)" : "Switch to the mouse, drawings stay (M)"}
                  onClick={() => api.drawSetPassthrough(!passthrough)}
                >
                  {passthrough ? <MousePointer2Icon className="size-4" /> : <Swatch color={draw.settings.color} size={draw.settings.size} />}
                </Tile>
              )}
            </Group>
          )}
          {grip}
        </Card>
      </div>
    );
  }

  return (
    <div className={cn("w-screen h-screen flex overflow-hidden justify-center", up ? "items-end" : "items-start")}>
      {/* One panel: the voice row, and the drawing row while drawing (above it when the bar is near the bottom) */}
      <Card ref={panelRef} className={cn("w-full gap-0 p-0", up ? "flex-col-reverse" : "flex-col", panelClass)}>
        <div className="flex flex-row items-center gap-2 p-2.5" onMouseDown={dragFromBlank}>
          {autoReadButton}

          <Popover open={panelOpen} onOpenChange={setPanelOpen}>
            <div className="relative flex-1 min-w-0">
              {toast ? (
                <div className="flex h-9 items-center gap-2 rounded-xl border border-input/60 bg-foreground/[0.04] px-3 text-sm select-none">
                  <InfoIcon className="h-4 w-4 shrink-0 text-muted-foreground" />
                  <span className="truncate">{toast}</span>
                </div>
              ) : busy ? (
                <PopoverTrigger asChild className="!bg-transparent">
                  <button
                    title="Show the full text"
                    className="flex h-9 w-full items-center gap-3 rounded-xl border border-input/60 bg-foreground/[0.04] px-3 text-sm select-none cursor-pointer hover:bg-accent/40 transition-colors"
                  >
                    {reader.status === "loading" ? (
                      <Loader2 className="h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
                    ) : (
                      <Visualizer active={!paused} paused={paused} className="shrink-0" />
                    )}
                    <span dir="auto" className="flex-1 min-w-0 truncate text-left">
                      {reader.status === "loading" && reader.current === 0 ? "Preparing voice…" : current?.text}
                    </span>
                    <span className="text-[10px] text-muted-foreground shrink-0 tabular-nums">
                      {reader.current + 1}/{reader.total}
                    </span>
                  </button>
                </PopoverTrigger>
              ) : (
                <PopoverAnchor asChild>
                  <div className="relative select-none">
                    <Input
                      dir="auto"
                      placeholder={engine.ready ? "Type text, press Enter…" : engine.error ? "Voice models failed to load" : "Loading voices…"}
                      value={text}
                      onChange={(e) => setText(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") speakTyped();
                      }}
                      className="pr-28 bg-foreground/[0.04] dark:bg-foreground/[0.04] border-input/60"
                    />
                    <div className="absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none">
                      {engine.ready ? (
                        <Kbd accel={settings?.shortcuts.read_selection} />
                      ) : (
                        !engine.error && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                      )}
                    </div>
                  </div>
                </PopoverAnchor>
              )}
            </div>
            <PopoverContent align="end" side={side} className="w-screen p-0 border shadow-lg overflow-hidden" sideOffset={8}>
              <ReadingPanel reader={reader} settings={settings} onClose={() => setPanelOpen(false)} />
            </PopoverContent>
          </Popover>

          {busy ? (
            playback(true)
          ) : (
            <Group>
              <Tile title="Read text from a screen area (images, videos, scanned PDFs)" icon={ScanTextIcon} onClick={() => api.openScreenReader()} />
              <Tile tone="draw" active={drawing} title={penTitle} icon={PenLineIcon} onClick={() => api.drawToggle()} />
              <QuickSettings settings={settings} update={update} side={side} />
              <Tile title="Open dashboard" icon={SparklesIcon} onClick={() => api.openDashboard()} />
            </Group>
          )}

          <Button variant="ghost" size="icon" className="w-fit px-1.5" title="Minimize the bar" onClick={() => setCompact(true)}>
            <Minimize2Icon className="h-4 w-4" />
          </Button>
          {grip}
        </div>
        {drawing && (
          <>
            {/* Inset hairline: separates the rows without splitting the panel */}
            <div className="mx-4 h-px bg-gradient-to-r from-transparent via-input/80 to-transparent" />
            <div
              className={cn(
                "flex justify-center px-2.5 animate-in fade-in duration-200 ease-out",
                up ? "pt-2.5 pb-2 slide-in-from-bottom-2" : "pt-2 pb-2.5 slide-in-from-top-2"
              )}
              onMouseDown={dragFromBlank}
            >
              <DrawRow ref={drawRowRef} settings={draw.settings} update={draw.update} status={status} passthrough={passthrough} pick={pick} side={side} />
            </div>
          </>
        )}
      </Card>
    </div>
  );
};
