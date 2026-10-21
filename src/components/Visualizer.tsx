import { cn } from "@/lib/utils";

const BARS = [0.45, 0.8, 0.6, 1, 0.7, 0.9, 0.5, 0.75, 0.55];

/** Speech activity indicator; animates while speaking, freezes when paused */
export const Visualizer = ({ active, paused, className }: { active: boolean; paused?: boolean; className?: string }) => (
  <div className={cn("flex items-center gap-[3px] h-5", active && "sayzo-wave", paused && "sayzo-paused", className)}>
    {BARS.map((h, i) => (
      <span
        key={i}
        className="w-[3px] rounded-full bg-foreground/80"
        style={{ height: `${h * 100}%`, animationDelay: `${i * 0.11}s`, transform: active ? undefined : "scaleY(0.3)" }}
      />
    ))}
  </div>
);
