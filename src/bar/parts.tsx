import { forwardRef } from "react";
import { LucideIcon } from "lucide-react";
import { Button } from "@/components";
import { cn } from "@/lib/utils";

/*
 * Building blocks shared by the bar's voice row, its drawing row and the
 * minimized pill, so the whole panel has one button style:
 * - white fill means "selected" (the current tool, the mouse, pause),
 * - blue means drawing (the pen button, the active drawing tool, options
 *   that are switched on),
 * - green means auto-read is on.
 */

/** Blue, the drawing accent */
export const DRAW_ACCENT = "bg-blue-600 text-white shadow-sm shadow-blue-500/30 hover:bg-blue-600/90 dark:bg-blue-500 dark:hover:bg-blue-500/90";
/** Green, auto-read is on */
export const AUTO_READ_ACCENT = "bg-green-600 text-white shadow-sm shadow-green-500/30 hover:bg-green-600/90 dark:bg-green-500 dark:hover:bg-green-500/90";

/** A rounded segment holding related buttons; 36 px tall like the text box */
export const Group = ({ children, className, index }: { children: React.ReactNode; className?: string; index?: number }) => (
  <div
    className={cn(
      "flex shrink-0 items-center gap-0.5 rounded-xl bg-foreground/[0.04] p-0.5 ring-1 ring-inset ring-input/50",
      index !== undefined && "animate-in fade-in zoom-in-95 duration-200 ease-out",
      className
    )}
    // The drawing row's segments unfold one after another
    style={index !== undefined ? { animationDelay: `${index * 25}ms`, animationFillMode: "both" } : undefined}
  >
    {children}
  </div>
);

/** Thin line between sub-groups inside a Group */
export const Sep = () => <div className="pointer-events-none mx-0.5 h-5 w-px shrink-0 bg-input/60" />;

export type Tone = "select" | "draw" | "auto";

/*
 * Tiles are defined outside their rows so they keep their identity across
 * re-renders: defined inside, every status update from the canvas recreated
 * them, and a click whose mouse-down and mouse-up straddled that re-render
 * was silently lost.
 */
export const Tile = forwardRef<
  HTMLButtonElement,
  {
    /** Selected: filled with the tone's colour */
    active?: boolean;
    /** Switched on (an option, not a selection): a soft blue tint */
    on?: boolean;
    tone?: Tone;
    title: string;
    icon?: LucideIcon;
    children?: React.ReactNode;
    className?: string;
  } & Omit<React.ComponentProps<"button">, "children">
>(({ active, on, tone = "select", title, icon: Icon, children, className, ...props }, ref) => (
  <Button
    ref={ref}
    size="icon"
    variant={active && tone === "select" ? "default" : "ghost"}
    title={title}
    className={cn(
      "size-8 rounded-[10px] transition-all duration-200",
      active && tone === "select" && "shadow-sm",
      active && tone === "draw" && DRAW_ACCENT,
      active && tone === "auto" && AUTO_READ_ACCENT,
      on && !active && "bg-blue-500/15 text-blue-600 ring-1 ring-inset ring-blue-400/30 hover:bg-blue-500/20 dark:text-blue-300",
      className
    )}
    {...props}
  >
    {Icon ? <Icon className="size-4" /> : children}
  </Button>
));
Tile.displayName = "Tile";

/** Brush preview: the colour, with the dot growing with the brush size */
export const Swatch = ({ color, size }: { color: string; size: number }) => (
  <span className="flex size-5 items-center justify-center rounded-full ring-1 ring-foreground/25">
    <span className="rounded-full shadow-sm" style={{ background: color, width: Math.min(16, 6 + size), height: Math.min(16, 6 + size) }} />
  </span>
);
