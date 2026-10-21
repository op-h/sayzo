import { Label } from "./ui/label";
import { cn } from "@/lib/utils";

interface HeaderProps {
  title: string;
  description: string;
  isMainTitle?: boolean;
  rightSlot?: React.ReactNode;
  className?: string;
}

export const Header = ({ title, description, isMainTitle = false, rightSlot, className }: HeaderProps) => (
  <div
    className={cn(
      "flex",
      rightSlot ? "flex-row justify-between items-center gap-4" : "flex-col",
      isMainTitle && "border-b border-input/50 pb-2",
      className
    )}
  >
    <div className="flex flex-col">
      <Label className={cn("font-semibold line-clamp-1", isMainTitle ? "text-md lg:text-lg" : "text-xs lg:text-sm")}>{title}</Label>
      <p className={cn("select-none text-muted-foreground leading-relaxed", isMainTitle ? "text-xs lg:text-sm" : "text-[10px] lg:text-xs")}>
        {description}
      </p>
    </div>
    {rightSlot}
  </div>
);
