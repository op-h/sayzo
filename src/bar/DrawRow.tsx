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
import { forwardRef } from "react";
import { Button, Popover, PopoverContent, PopoverTrigger } from "@/components";
import { Group, Sep, Swatch, Tile } from "./parts";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Board, CanvasStatus, COLORS, DrawSettings, sendCommand, Shape, SHAPES, SIZES, Tool, TOOL_LABELS } from "@/draw/model";

export const TOOL_ICONS: Record<Tool, LucideIcon> = {
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

export interface DrawRowProps {
  settings: DrawSettings;
  update: (patch: Partial<DrawSettings>) => void;
  status: CanvasStatus;
  passthrough: boolean;
  pick: (tool: Tool) => void;
  /** Which side popovers open on: away from the screen edge the bar is near */
  side: "top" | "bottom";
}

/** The drawing tools, shown as the voice bar's second row while drawing is on */
export const DrawRow = forwardRef<HTMLDivElement, DrawRowProps>(({ settings, update, status, passthrough, pick, side }, ref) => {
  const drawingWith = (t: Tool) => settings.tool === t && !passthrough;
  const ShapeIcon = TOOL_ICONS[settings.shape];
  const shapeActive = (SHAPES as string[]).includes(settings.tool) && !passthrough;

  return (
    <div ref={ref} className="flex w-fit items-center gap-2">
      <Group index={0}>
        <Tile active={passthrough} title="Mouse: use your apps, drawings stay (M or Ctrl+Alt+D)" icon={MousePointer2Icon} onClick={() => api.drawSetPassthrough(true)} />
        <Sep />
        {(["pen", "highlighter", "laser"] as Tool[]).map((t) => (
          <Tile key={t} tone="draw" active={drawingWith(t)} title={TOOL_LABELS[t]} icon={TOOL_ICONS[t]} onClick={() => pick(t)} />
        ))}
        <Popover>
          <PopoverTrigger asChild>
            <Tile tone="draw" active={shapeActive} title="Shapes: line, arrow, rectangle, ellipse" icon={ShapeIcon} />
          </PopoverTrigger>
          <PopoverContent side={side} sideOffset={10} className="w-auto p-1.5">
            <div className="flex gap-1">
              {SHAPES.map((s: Shape) => (
                <Tile key={s} tone="draw" active={settings.tool === s && !passthrough} title={TOOL_LABELS[s]} icon={TOOL_ICONS[s]} className="size-9" onClick={() => pick(s)} />
              ))}
            </div>
            <p className="px-1 pt-1.5 text-[10px] text-muted-foreground">Hold Shift for straight lines and perfect squares</p>
          </PopoverContent>
        </Popover>
        {(["text", "number", "eraser", "read"] as Tool[]).map((t) => (
          <Tile key={t} tone="draw" active={drawingWith(t)} title={TOOL_LABELS[t]} icon={TOOL_ICONS[t]} onClick={() => pick(t)} />
        ))}
        <Sep />
        <Popover>
          <PopoverTrigger asChild>
            <Tile title="Colour and size (1–9, [ ])">
              <Swatch color={settings.color} size={settings.size} />
            </Tile>
          </PopoverTrigger>
          <PopoverContent side={side} sideOffset={10} className="w-56 p-3">
            <div className="grid grid-cols-6 gap-2">
              {COLORS.map((c, i) => (
                <button
                  key={c}
                  title={i < 9 ? `Key ${i + 1}` : undefined}
                  onClick={() => update({ color: c })}
                  className={cn(
                    "size-7 rounded-full ring-offset-2 ring-offset-popover transition-transform hover:scale-110",
                    settings.color === c ? "ring-2 ring-blue-400" : "ring-1 ring-input"
                  )}
                  style={{ background: c }}
                />
              ))}
            </div>
            <label className="mt-3 flex items-center justify-between text-xs text-muted-foreground">
              Custom colour
              <input
                type="color"
                value={settings.color}
                onChange={(e) => update({ color: e.target.value })}
                className="size-7 cursor-pointer rounded-full border-0 bg-transparent p-0 ring-1 ring-input [&::-webkit-color-swatch]:rounded-full [&::-webkit-color-swatch]:border-0 [&::-webkit-color-swatch-wrapper]:p-0"
              />
            </label>
            <div className="mt-3 flex items-center justify-between gap-2">
              {SIZES.map((s) => (
                <button
                  key={s}
                  onClick={() => update({ size: s })}
                  title={`Size ${s}`}
                  className={cn("flex size-9 items-center justify-center rounded-xl transition-colors hover:bg-accent", settings.size === s && "bg-accent ring-1 ring-blue-400/60")}
                >
                  <span className="rounded-full" style={{ width: s + 3, height: s + 3, background: settings.color }} />
                </button>
              ))}
            </div>
          </PopoverContent>
        </Popover>
      </Group>

      <Group index={1}>
        <Tile title="Undo (Ctrl+Z)" icon={Undo2Icon} disabled={!status.canUndo} onClick={() => sendCommand("undo")} />
        <Tile title="Redo (Ctrl+Y)" icon={Redo2Icon} disabled={!status.canRedo} onClick={() => sendCommand("redo")} />
        <Tile title="Clear everything (Delete)" icon={Trash2Icon} disabled={!status.count} onClick={() => sendCommand("clear")} />
      </Group>

      <Group index={2}>
        <Popover>
          <PopoverTrigger asChild>
            <Tile on={settings.board !== "none"} title="Whiteboard / blackboard" icon={PresentationIcon} />
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
        <Tile on={settings.fade} title="Fading ink: new drawings disappear after a few seconds (F)" icon={HourglassIcon} onClick={() => update({ fade: !settings.fade })} />
        <Tile on={settings.spotlight} title="Spotlight: dim everything except around the cursor (G)" icon={FlashlightIcon} onClick={() => update({ spotlight: !settings.spotlight })} />
        <Tile on={status.hidden} title={status.hidden ? "Show drawings (V)" : "Hide drawings (V)"} icon={status.hidden ? EyeOffIcon : EyeIcon} onClick={() => sendCommand("toggle-ink")} />
      </Group>

      <Group index={3}>
        <Tile title="Screenshot with drawings: copies and saves to Pictures\Sayzo (Ctrl+Shift+C)" icon={CameraIcon} onClick={() => sendCommand("snapshot")} />
      </Group>

      {/* Bare, like the minimize button and grip at the end of the voice row */}
      <Button size="icon" variant="ghost" className="shrink-0 hover:bg-destructive/15 hover:text-destructive" title="Stop drawing (Esc)" onClick={() => api.drawClose()}>
        <XIcon className="size-4" />
      </Button>
    </div>
  );
});
DrawRow.displayName = "DrawRow";
