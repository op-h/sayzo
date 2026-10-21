import { formatShortcut } from "@/lib/api";
import { cn } from "@/lib/utils";

export const Kbd = ({ accel, className }: { accel?: string; className?: string }) => {
  const keys = formatShortcut(accel);
  if (!keys.length) return null;
  return (
    <span className={cn("inline-flex items-center gap-0.5 select-none", className)}>
      {keys.map((k, i) => (
        <span
          key={`${k}-${i}`}
          className="text-[10px] text-muted-foreground bg-muted/30 px-1 rounded border border-input/50 leading-4"
        >
          {k}
        </span>
      ))}
    </span>
  );
};
