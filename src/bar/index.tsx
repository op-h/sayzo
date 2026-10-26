import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AudioLinesIcon,
  Loader2,
  Minimize2Icon,
  MousePointerClickIcon,
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
import { listen } from "@tauri-apps/api/event";
import { api, DrawState } from "@/lib/api";
import { cn } from "@/lib/utils";
import { ReadingPanel } from "./ReadingPanel";
import { QuickSettings } from "./QuickSettings";

const FULL_WIDTH = 660;

/** Lets any empty part of the bar drag the window (buttons keep working) */
const dragFromBlank = (e: React.MouseEvent) => {
  if (e.button === 0 && e.target === e.currentTarget) getCurrentWindow().startDragging();
};

export const Bar = () => {
  useWindowResize();
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

  // Full bar: fixed width. Minimized: the window hugs the pill exactly, so
  // the spacing is the same on every side whatever buttons it shows.
  const pillRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!settings) return;
    if (!compact) {
      api.setBarWidth(FULL_WIDTH).catch(() => {});
      return;
    }
    const el = pillRef.current;
    if (!el) return;
    let last = 0;
    const fit = () => {
      const w = Math.ceil(el.offsetWidth);
      if (w && w !== last) {
        last = w;
        api.setBarWidth(w).catch(() => {});
      }
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [compact, settings !== null]);

  // The pen button shows when drawing is on
  const [drawing, setDrawing] = useState(false);
  useEffect(() => {
    api.drawGetState().then((s) => setDrawing(s.open)).catch(() => {});
    const un = listen<DrawState>("draw://state", (e) => setDrawing(e.payload.open));
    return () => {
      un.then((f) => f());
    };
  }, []);
  // Blue while drawing is on, the same way auto-read turns green
  const penClass = cn(drawing && "bg-blue-600 text-white hover:bg-blue-600/90 dark:bg-blue-500 dark:hover:bg-blue-500/90");
  const penTitle = drawing ? "Stop drawing (Ctrl+Alt+D switches to the mouse)" : "Draw on screen (Ctrl+Alt+D)";

  const setCompact = (on: boolean) => {
    if (on) setPanelOpen(false);
    update({ compactBar: on });
  };

  // Pulled in slightly so the dots sit as far from the edge as the first button does
  const grip = <Grip className="-mr-1.5" />;

  const autoReadButton = (
    <Button
      size="icon"
      title={autoRead ? "Auto-read is on: selecting text reads it aloud. Click to turn off." : "Turn on auto-read: any text you select is read aloud"}
      onClick={() => api.setAutoRead(!autoRead)}
      className={cn(autoRead && "bg-green-600 text-white hover:bg-green-600/90 dark:bg-green-500 dark:hover:bg-green-500/90")}
    >
      <MousePointerClickIcon className="h-4 w-4" />
    </Button>
  );

  if (compact) {
    return (
      <div className="w-screen h-screen flex overflow-hidden justify-center items-start">
        <Card ref={pillRef} className="w-fit shrink-0 flex flex-row items-center gap-2 p-2.5 whitespace-nowrap" onMouseDown={dragFromBlank}>
          <Button size="icon" title="Show the full bar" onClick={() => setCompact(false)}>
            {busy && reader.status !== "loading" ? <Visualizer active={!paused} paused={paused} /> : <AudioLinesIcon className="h-4 w-4" />}
          </Button>
          {busy ? (
            <>
              <Button size="icon" title={paused ? "Resume" : "Pause"} onClick={() => api.control("toggle")}>
                {paused ? <PlayIcon className="h-4 w-4" /> : <PauseIcon className="h-4 w-4" />}
              </Button>
              <Button size="icon" variant="ghost" title="Stop" onClick={() => api.control("stop")}>
                <SquareIcon className="h-4 w-4" />
              </Button>
            </>
          ) : (
            <>
              {autoReadButton}
              <Button size="icon" className={penClass} title={penTitle} onClick={() => api.drawToggle()}>
                <PenLineIcon className="h-4 w-4" />
              </Button>
            </>
          )}
          {grip}
        </Card>
      </div>
    );
  }

  return (
    <div className="w-screen h-screen flex overflow-hidden justify-center items-start">
      <Card className="w-full flex flex-row items-center gap-2 p-2.5" onMouseDown={dragFromBlank}>
        {autoReadButton}

        <Popover open={panelOpen} onOpenChange={setPanelOpen}>
          <div className="relative flex-1 min-w-0">
            {toast ? (
              <div className="flex h-9 items-center gap-2 rounded-xl border border-primary/50 dark:border-input/80 px-3 text-sm select-none">
                <InfoIcon className="h-4 w-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{toast}</span>
              </div>
            ) : busy ? (
              <PopoverTrigger asChild className="!bg-transparent">
                <button
                  title="Show the full text"
                  className="flex h-9 w-full items-center gap-3 rounded-xl border border-primary/50 dark:border-input/80 px-3 text-sm select-none cursor-pointer hover:bg-accent/40 transition-colors"
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
                    className="pr-28"
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
          <PopoverContent align="end" side="bottom" className="w-screen p-0 border shadow-lg overflow-hidden" sideOffset={8}>
            <ReadingPanel reader={reader} settings={settings} onClose={() => setPanelOpen(false)} />
          </PopoverContent>
        </Popover>

        {busy ? (
          <>
            <Button size="icon" variant="ghost" title="Previous sentence" onClick={() => api.control("previous")}>
              <SkipBackIcon className="h-4 w-4" />
            </Button>
            <Button size="icon" title={paused ? "Resume" : "Pause"} onClick={() => api.control("toggle")}>
              {paused ? <PlayIcon className="h-4 w-4" /> : <PauseIcon className="h-4 w-4" />}
            </Button>
            <Button size="icon" variant="ghost" title="Next sentence" onClick={() => api.control("next")}>
              <SkipForwardIcon className="h-4 w-4" />
            </Button>
            <Button size="icon" variant="ghost" title="Stop" onClick={() => api.control("stop")}>
              <SquareIcon className="h-4 w-4" />
            </Button>
          </>
        ) : (
          <>
            <Button size="icon" title="Read text from a screen area (images, videos, scanned PDFs)" onClick={() => api.openScreenReader()}>
              <ScanTextIcon className="h-4 w-4" />
            </Button>
            <Button size="icon" className={penClass} title={penTitle} onClick={() => api.drawToggle()}>
              <PenLineIcon className="h-4 w-4" />
            </Button>
            <QuickSettings settings={settings} update={update} />
            <Button size="icon" title="Open dashboard" onClick={() => api.openDashboard()}>
              <SparklesIcon className="h-4 w-4" />
            </Button>
          </>
        )}

        <Button variant="ghost" size="icon" className="w-fit px-1.5" title="Minimize the bar" onClick={() => setCompact(true)}>
          <Minimize2Icon className="h-4 w-4" />
        </Button>
        {grip}
      </Card>
    </div>
  );
};
