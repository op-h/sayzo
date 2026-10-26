import { useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CameraIcon,
  CircleIcon,
  EraserIcon,
  EyeIcon,
  EyeOffIcon,
  FlashlightIcon,
  HighlighterIcon,
  HourglassIcon,
  ListOrderedIcon,
  LucideIcon,
  Maximize2Icon,
  Minimize2Icon,
  MousePointer2Icon,
  MoveUpRightIcon,
  PenLineIcon,
  PresentationIcon,
  Redo2Icon,
  ScanTextIcon,
  SlashIcon,
  SquareIcon,
  Trash2Icon,
  TypeIcon,
  Undo2Icon,
  XIcon,
  ZapIcon,
} from "lucide-react";
import { Button, Card, Grip, Popover, PopoverContent, PopoverTrigger } from "@/components";
import { useShowWhenReady } from "@/hooks";
import { api, DrawState } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Board, CanvasStatus, COLORS, handleDrawKey, sendCommand, Shape, SHAPES, SIZES, Tool, TOOL_LABELS, useDrawSettings } from "./model";

/** Card height: 36 px buttons + 10 px padding each side + 1 px border (width is measured) */
const HEIGHT = 58;
/** Room above the toolbar for a popover */
const POPOVER_HEIGHT = 330;

const TOOL_ICONS: Record<Tool, LucideIcon> = {
  pen: PenLineIcon,
  highlighter: HighlighterIcon,
  laser: ZapIcon,
  line: SlashIcon,
  arrow: MoveUpRightIcon,
  rect: SquareIcon,
  ellipse: CircleIcon,
  text: TypeIcon,
  number: ListOrderedIcon,
  eraser: EraserIcon,
  read: ScanTextIcon,
};

const BOARDS: { id: Board; label: string }[] = [
  { id: "none", label: "Draw on screen" },
  { id: "white", label: "Whiteboard (W)" },
  { id: "black", label: "Blackboard (B)" },
];

/** Lets any empty part of the toolbar drag the window (buttons keep working) */
const dragFromBlank = (e: React.MouseEvent) => {
  if (e.button === 0 && e.target === e.currentTarget) {
    // Moving the toolbar yourself undocks it from the voice bar
    api.drawUndock();
    getCurrentWindow().startDragging();
  }
};

const Divider = () => <div className="pointer-events-none mx-1 h-7 w-px shrink-0 bg-input/60" />;

/*
 * Button components live outside the toolbar so they keep their identity
 * across re-renders. Defined inside, every status update from the canvas
 * recreated them, and a click whose mouse-down and mouse-up straddled that
 * re-render was silently lost.
 */
const ToolButton = ({ tool, active, onPick }: { tool: Tool; active: boolean; onPick: (t: Tool) => void }) => {
  const Icon = TOOL_ICONS[tool];
  return (
    <Button size="icon" variant={active ? "default" : "ghost"} className="size-9 shrink-0" title={TOOL_LABELS[tool]} onClick={() => onPick(tool)}>
      <Icon className="size-[18px]" />
    </Button>
  );
};

const Toggle = ({
  on,
  title,
  icon: Icon,
  onClick,
  disabled,
  className,
}: {
  on?: boolean;
  title: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
  className?: string;
}) => (
  <Button
    size="icon"
    variant="ghost"
    disabled={disabled}
    className={cn("size-9 shrink-0", on && "bg-primary/15 text-primary ring-1 ring-primary/40", className)}
    title={title}
    onClick={onClick}
  >
    <Icon className="size-[18px]" />
  </Button>
);

/** Floating drawing toolbar in Sayzo's glass style */
export const DrawToolbar = () => {
  useShowWhenReady();
  const { settings, update } = useDrawSettings();
  const [status, setStatus] = useState<CanvasStatus>({ canUndo: false, canRedo: false, count: 0, hidden: false });
  // Drawing opens in mouse mode, so the screen stays usable until a tool is picked
  const [passthrough, setPassthrough] = useState(true);
  const [collapsed, setCollapsed] = useState(false);
  // Which side popovers open on, decided by where the toolbar sits (docked: below)
  const [side, setSide] = useState<"top" | "bottom">("bottom");

  // The window hugs the card exactly, and grows for an open popover
  const cardRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [popoverOpen, setPopoverOpen] = useState(false);
  useEffect(() => {
    const el = cardRef.current;
    if (!el) return;
    const fit = () => setWidth(Math.ceil(el.offsetWidth));
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [collapsed]);
  useEffect(() => {
    const sync = () => setPopoverOpen(document.querySelectorAll("[data-radix-popper-content-wrapper]").length > 0);
    const mo = new MutationObserver(sync);
    mo.observe(document.body, { childList: true, subtree: true });
    return () => mo.disconnect();
  }, []);
  useEffect(() => {
    if (!width) return;
    api
      .drawToolbarSize(width, popoverOpen ? HEIGHT + POPOVER_HEIGHT : HEIGHT)
      .then((dir) => popoverOpen && setSide(dir === "down" ? "bottom" : "top"))
      .catch(() => {});
  }, [width, popoverOpen]);

  useEffect(() => {
    const a = listen<CanvasStatus>("draw://status", (e) => setStatus(e.payload));
    const b = listen<DrawState>("draw://state", (e) => setPassthrough(e.payload.passthrough));
    api.drawGetState().then((s) => setPassthrough(s.passthrough));
    emit("draw://status-request");
    return () => {
      a.then((f) => f());
      b.then((f) => f());
    };
  }, []);

  /** Picking a tool starts drawing; clicking the active tool again goes back to the mouse */
  const pick = (tool: Tool) => {
    if (!passthrough && settings.tool === tool) {
      api.drawSetPassthrough(true);
      return;
    }
    update({ tool });
    if (passthrough) api.drawSetPassthrough(false);
  };

  // The same single-key shortcuts work when the toolbar has focus
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      // While a popover is open, Esc just closes it (Radix handles that)
      if (document.querySelector("[data-radix-popper-content-wrapper]")) return;
      if (handleDrawKey(e, settings, update, { exit: () => api.drawClose(), mouseMode: () => api.drawSetPassthrough(!passthrough), pickTool: pick })) {
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [settings, update, passthrough, pick]);


  const collapse = (on: boolean) => setCollapsed(on);

  // Both ends of the toolbar are icon-only (grip dots, ✕), so they're balanced by eye
  const grip = <Grip onDragStart={() => api.drawUndock()} />;

  if (collapsed) {
    return (
      <div className={cn("flex h-screen w-screen justify-center overflow-hidden", side === "top" ? "items-end" : "items-start")}>
        <Card ref={cardRef} className="flex w-fit shrink-0 flex-row items-center gap-1 whitespace-nowrap p-2.5" onMouseDown={dragFromBlank}>
          {grip}
          <div className="size-5 shrink-0 rounded-full ring-2 ring-white/70" style={{ background: settings.color }} title="Current colour" />
          <Toggle on={passthrough} title="Mouse mode (M)" icon={MousePointer2Icon} onClick={() => api.drawSetPassthrough(!passthrough)} />
          <Toggle className="-mr-2" title="Show the full toolbar" icon={Maximize2Icon} onClick={() => collapse(false)} />
        </Card>
      </div>
    );
  }

  const ShapeIcon = TOOL_ICONS[settings.shape];
  const shapeActive = (SHAPES as string[]).includes(settings.tool) && !passthrough;

  return (
    <div className={cn("flex h-screen w-screen justify-center overflow-hidden", side === "top" ? "items-end" : "items-start")}>
      <Card ref={cardRef} className="flex w-fit shrink-0 flex-row items-center gap-1 whitespace-nowrap p-2.5" onMouseDown={dragFromBlank}>
        {grip}
        <Button
          size="icon"
          variant={passthrough ? "default" : "ghost"}
          className="size-9 shrink-0"
          title="Mouse: use your apps, drawings stay (M or Ctrl+Alt+D)"
          onClick={() => api.drawSetPassthrough(true)}
        >
          <MousePointer2Icon className="size-[18px]" />
        </Button>
        <Divider />
        <ToolButton tool="pen" active={settings.tool === "pen" && !passthrough} onPick={pick} />
        <ToolButton tool="highlighter" active={settings.tool === "highlighter" && !passthrough} onPick={pick} />
        <ToolButton tool="laser" active={settings.tool === "laser" && !passthrough} onPick={pick} />
        <Popover>
          <PopoverTrigger asChild>
            <Button size="icon" variant={shapeActive ? "default" : "ghost"} className="size-9 shrink-0" title="Shapes: line, arrow, rectangle, ellipse">
              <ShapeIcon className="size-[18px]" />
            </Button>
          </PopoverTrigger>
          <PopoverContent side={side} sideOffset={10} className="w-auto p-1.5">
            <div className="flex gap-1">
              {SHAPES.map((s: Shape) => {
                const Icon = TOOL_ICONS[s];
                return (
                  <Button key={s} size="icon" variant={settings.tool === s ? "default" : "ghost"} className="size-9" title={TOOL_LABELS[s]} onClick={() => pick(s)}>
                    <Icon className="size-[18px]" />
                  </Button>
                );
              })}
            </div>
            <p className="px-1 pt-1.5 text-[10px] text-muted-foreground">Hold Shift for straight lines and perfect squares</p>
          </PopoverContent>
        </Popover>
        <ToolButton tool="text" active={settings.tool === "text" && !passthrough} onPick={pick} />
        <ToolButton tool="number" active={settings.tool === "number" && !passthrough} onPick={pick} />
        <ToolButton tool="eraser" active={settings.tool === "eraser" && !passthrough} onPick={pick} />
        <ToolButton tool="read" active={settings.tool === "read" && !passthrough} onPick={pick} />
        <Divider />

        <Popover>
          <PopoverTrigger asChild>
            <Button size="icon" variant="ghost" className="size-9 shrink-0" title="Colour and size (1–9, [ ])">
              <span className="relative flex size-5 items-center justify-center rounded-full ring-2 ring-white/70" style={{ background: settings.color }}>
                <span className="rounded-full bg-black/40" style={{ width: Math.min(12, settings.size + 2), height: Math.min(12, settings.size + 2) }} />
              </span>
            </Button>
          </PopoverTrigger>
          <PopoverContent side={side} sideOffset={10} className="w-56 p-3">
            <div className="grid grid-cols-6 gap-2">
              {COLORS.map((c, i) => (
                <button
                  key={c}
                  title={i < 9 ? `Key ${i + 1}` : undefined}
                  onClick={() => update({ color: c })}
                  className={cn("size-7 rounded-full ring-offset-2 ring-offset-popover transition-transform hover:scale-110", settings.color === c ? "ring-2 ring-primary" : "ring-1 ring-input")}
                  style={{ background: c }}
                />
              ))}
            </div>
            <label className="mt-3 flex items-center justify-between text-xs text-muted-foreground">
              Custom colour
              <input type="color" value={settings.color} onChange={(e) => update({ color: e.target.value })} className="h-6 w-10 cursor-pointer rounded border-0 bg-transparent" />
            </label>
            <div className="mt-3 flex items-center justify-between gap-2">
              {SIZES.map((s) => (
                <button
                  key={s}
                  onClick={() => update({ size: s })}
                  title={`Size ${s}`}
                  className={cn("flex size-9 items-center justify-center rounded-xl transition-colors hover:bg-accent", settings.size === s && "bg-accent ring-1 ring-primary/50")}
                >
                  <span className="rounded-full" style={{ width: s + 3, height: s + 3, background: settings.color }} />
                </button>
              ))}
            </div>
          </PopoverContent>
        </Popover>
        <Divider />

        <Toggle title="Undo (Ctrl+Z)" icon={Undo2Icon} disabled={!status.canUndo} onClick={() => sendCommand("undo")} />
        <Toggle title="Redo (Ctrl+Y)" icon={Redo2Icon} disabled={!status.canRedo} onClick={() => sendCommand("redo")} />
        <Toggle title="Clear everything (Delete)" icon={Trash2Icon} disabled={!status.count} onClick={() => sendCommand("clear")} />
        <Divider />

        <Popover>
          <PopoverTrigger asChild>
            <Button size="icon" variant="ghost" className={cn("size-9 shrink-0", settings.board !== "none" && "bg-primary/15 text-primary ring-1 ring-primary/40")} title="Whiteboard / blackboard">
              <PresentationIcon className="size-[18px]" />
            </Button>
          </PopoverTrigger>
          <PopoverContent side={side} sideOffset={10} className="w-48 p-1.5">
            {BOARDS.map((b) => (
              <button
                key={b.id}
                onClick={() => update({ board: b.id })}
                className={cn("flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-xs hover:bg-accent", settings.board === b.id && "bg-accent")}
              >
                <span
                  className="size-4 rounded border border-input"
                  style={{ background: b.id === "white" ? "#fbfbf8" : b.id === "black" ? "#16181d" : "repeating-conic-gradient(#888 0 25%, transparent 0 50%) 0 0/8px 8px" }}
                />
                {b.label}
              </button>
            ))}
          </PopoverContent>
        </Popover>
        <Toggle on={settings.fade} title="Fading ink: new drawings disappear after a few seconds (F)" icon={HourglassIcon} onClick={() => update({ fade: !settings.fade })} />
        <Toggle on={settings.spotlight} title="Spotlight: dim everything except around the cursor (G)" icon={FlashlightIcon} onClick={() => update({ spotlight: !settings.spotlight })} />
        <Toggle on={status.hidden} title={status.hidden ? "Show drawings (V)" : "Hide drawings (V)"} icon={status.hidden ? EyeOffIcon : EyeIcon} onClick={() => sendCommand("toggle-ink")} />
        <Divider />

        <Toggle title="Screenshot with drawings: copies and saves to Pictures\Sayzo (Ctrl+Shift+C)" icon={CameraIcon} onClick={() => sendCommand("snapshot")} />
        <Toggle title="Make the toolbar small" icon={Minimize2Icon} onClick={() => collapse(true)} />
        <Button size="icon" variant="ghost" className="-mr-2 size-9 shrink-0 hover:text-destructive" title="Stop drawing (Esc)" onClick={() => api.drawClose()}>
          <XIcon className="size-[18px]" />
        </Button>
      </Card>
    </div>
  );
};
