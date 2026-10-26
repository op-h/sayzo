import { useEffect, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CameraIcon,
  CircleIcon,
  EraserIcon,
  EyeIcon,
  EyeOffIcon,
  FlashlightIcon,
  GripVerticalIcon,
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
import { Button, Card, Popover, PopoverContent, PopoverTrigger } from "@/components";
import { api, DrawState } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Board, CanvasStatus, COLORS, handleDrawKey, sendCommand, Shape, SHAPES, SIZES, Tool, TOOL_LABELS, useDrawSettings } from "./model";

const WIDTH = 820;
const COLLAPSED_WIDTH = 132;
const HEIGHT = 54;
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

/** Grows the toolbar window upwards while a popover is open so it isn't clipped */
const usePopoverRoom = (collapsed: boolean) => {
  useEffect(() => {
    let open = false;
    const sync = () => {
      const now = document.querySelectorAll("[data-radix-popper-content-wrapper]").length > 0;
      if (now !== open) {
        open = now;
        api.drawToolbarSize(collapsed ? COLLAPSED_WIDTH : WIDTH, now ? HEIGHT + POPOVER_HEIGHT : HEIGHT).catch(() => {});
      }
    };
    const observer = new MutationObserver(sync);
    observer.observe(document.body, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, [collapsed]);
};

const Divider = () => <div className="mx-0.5 h-6 w-px shrink-0 bg-input/60" />;

/** Floating drawing toolbar in Sayzo's glass style */
export const DrawToolbar = () => {
  const { settings, update } = useDrawSettings();
  const [status, setStatus] = useState<CanvasStatus>({ canUndo: false, canRedo: false, count: 0, hidden: false });
  const [passthrough, setPassthrough] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  usePopoverRoom(collapsed);

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

  // The same single-key shortcuts work when the toolbar has focus
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      // While a popover is open, Esc just closes it (Radix handles that)
      if (document.querySelector("[data-radix-popper-content-wrapper]")) return;
      if (handleDrawKey(e, settings, update, { exit: () => api.drawClose(), mouseMode: () => api.drawSetPassthrough(!passthrough) })) {
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [settings, update, passthrough]);

  const pick = (tool: Tool) => {
    update({ tool });
    // Picking a tool means you want to draw again
    if (passthrough) api.drawSetPassthrough(false);
  };

  const collapse = (on: boolean) => {
    setCollapsed(on);
    api.drawToolbarSize(on ? COLLAPSED_WIDTH : WIDTH, HEIGHT).catch(() => {});
  };

  const grip = (
    <Button
      variant="ghost"
      size="icon"
      className="size-8 w-fit shrink-0 cursor-grab px-0.5"
      title="Drag to move"
      onMouseDown={(e) => e.button === 0 && getCurrentWindow().startDragging()}
    >
      <GripVerticalIcon className="h-4 w-4" />
    </Button>
  );

  const ToolButton = ({ tool }: { tool: Tool }) => {
    const Icon = TOOL_ICONS[tool];
    const active = settings.tool === tool && !passthrough;
    return (
      <Button size="icon" variant={active ? "default" : "ghost"} className="size-8 shrink-0" title={TOOL_LABELS[tool]} onClick={() => pick(tool)}>
        <Icon className="h-4 w-4" />
      </Button>
    );
  };

  const Toggle = ({ on, title, icon: Icon, onClick, disabled }: { on?: boolean; title: string; icon: LucideIcon; onClick: () => void; disabled?: boolean }) => (
    <Button
      size="icon"
      variant="ghost"
      disabled={disabled}
      className={cn("size-8 shrink-0", on && "bg-primary/15 text-primary ring-1 ring-primary/40")}
      title={title}
      onClick={onClick}
    >
      <Icon className="h-4 w-4" />
    </Button>
  );

  if (collapsed) {
    return (
      <div className="flex h-screen w-screen items-end justify-center overflow-hidden">
        <Card className="flex w-full flex-row items-center gap-1 p-2">
          {grip}
          <div className="size-5 shrink-0 rounded-full ring-2 ring-white/70" style={{ background: settings.color }} title="Current colour" />
          <Toggle on={passthrough} title="Mouse mode (M)" icon={MousePointer2Icon} onClick={() => api.drawSetPassthrough(!passthrough)} />
          <Toggle title="Show the full toolbar" icon={Maximize2Icon} onClick={() => collapse(false)} />
        </Card>
      </div>
    );
  }

  const ShapeIcon = TOOL_ICONS[settings.shape];
  const shapeActive = (SHAPES as string[]).includes(settings.tool) && !passthrough;

  return (
    <div className="flex h-screen w-screen items-end justify-center overflow-hidden">
      <Card className="flex w-full flex-row items-center gap-1 p-2">
        {grip}
        <ToolButton tool="pen" />
        <ToolButton tool="highlighter" />
        <ToolButton tool="laser" />
        <Popover>
          <PopoverTrigger asChild>
            <Button size="icon" variant={shapeActive ? "default" : "ghost"} className="size-8 shrink-0" title="Shapes: line, arrow, rectangle, ellipse">
              <ShapeIcon className="h-4 w-4" />
            </Button>
          </PopoverTrigger>
          <PopoverContent side="top" sideOffset={10} className="w-auto p-1.5">
            <div className="flex gap-1">
              {SHAPES.map((s: Shape) => {
                const Icon = TOOL_ICONS[s];
                return (
                  <Button key={s} size="icon" variant={settings.tool === s ? "default" : "ghost"} className="size-9" title={TOOL_LABELS[s]} onClick={() => pick(s)}>
                    <Icon className="h-4 w-4" />
                  </Button>
                );
              })}
            </div>
            <p className="px-1 pt-1.5 text-[10px] text-muted-foreground">Hold Shift for straight lines and perfect squares</p>
          </PopoverContent>
        </Popover>
        <ToolButton tool="text" />
        <ToolButton tool="number" />
        <ToolButton tool="eraser" />
        <ToolButton tool="read" />
        <Divider />

        <Popover>
          <PopoverTrigger asChild>
            <Button size="icon" variant="ghost" className="size-8 shrink-0" title="Colour and size (1–9, [ ])">
              <span className="relative flex size-5 items-center justify-center rounded-full ring-2 ring-white/70" style={{ background: settings.color }}>
                <span className="rounded-full bg-black/40" style={{ width: Math.min(12, settings.size + 2), height: Math.min(12, settings.size + 2) }} />
              </span>
            </Button>
          </PopoverTrigger>
          <PopoverContent side="top" sideOffset={10} className="w-56 p-3">
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
            <Button size="icon" variant="ghost" className={cn("size-8 shrink-0", settings.board !== "none" && "bg-primary/15 text-primary ring-1 ring-primary/40")} title="Whiteboard / blackboard">
              <PresentationIcon className="h-4 w-4" />
            </Button>
          </PopoverTrigger>
          <PopoverContent side="top" sideOffset={10} className="w-48 p-1.5">
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

        <Toggle on={passthrough} title="Mouse mode: use your apps, drawings stay (M or Ctrl+Alt+D)" icon={MousePointer2Icon} onClick={() => api.drawSetPassthrough(!passthrough)} />
        <Toggle title="Screenshot with drawings: copies and saves to Pictures\Sayzo (Ctrl+Shift+C)" icon={CameraIcon} onClick={() => sendCommand("snapshot")} />
        <Toggle title="Make the toolbar small" icon={Minimize2Icon} onClick={() => collapse(true)} />
        <Button size="icon" variant="ghost" className="size-8 shrink-0 hover:text-destructive" title="Stop drawing (Esc)" onClick={() => api.drawClose()}>
          <XIcon className="h-4 w-4" />
        </Button>
      </Card>
    </div>
  );
};
