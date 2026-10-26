import { getCurrentWindow } from "@tauri-apps/api/window";
import { cn } from "@/lib/utils";

/**
 * Six-dot drag handle shared by the voice bar and the drawing toolbar.
 * Wider-spaced filled dots, with hover and press feedback, and no extra
 * padding so the bar's spacing stays even on both sides.
 */
export const Grip = ({ className, onDragStart }: { className?: string; onDragStart?: () => void }) => (
  <div
    role="button"
    aria-label="Drag to move"
    title="Drag to move (or drag any empty space)"
    onMouseDown={(e) => {
      if (e.button !== 0) return;
      onDragStart?.();
      getCurrentWindow().startDragging();
    }}
    className={cn(
      "group flex h-9 w-6 shrink-0 cursor-grab items-center justify-center rounded-lg transition-all duration-150",
      "hover:bg-accent active:scale-95 active:cursor-grabbing active:bg-accent/80",
      className
    )}
  >
    <div className="grid grid-cols-2 gap-x-[5px] gap-y-[4px]">
      {Array.from({ length: 6 }, (_, i) => (
        <span key={i} className="size-[4px] rounded-full bg-foreground/55 transition-colors group-hover:bg-foreground" />
      ))}
    </div>
  </div>
);
