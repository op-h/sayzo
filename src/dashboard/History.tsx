import { useCallback, useEffect, useState } from "react";
import { CopyIcon, PlayIcon, Trash2Icon } from "lucide-react";
import { Badge, Button, Card } from "@/components";
import { useReader } from "@/hooks";
import { api, HistoryItem } from "@/lib/api";
import { PageLayout } from "./PageLayout";

const SOURCE_LABEL: Record<string, string> = { selection: "Selection", typed: "Typed", screen: "Screen area" };

const timeAgo = (ms: number) => {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return new Date(ms).toLocaleDateString();
};

export const History = () => {
  const [items, setItems] = useState<HistoryItem[]>([]);
  const reader = useReader();
  const refresh = useCallback(() => api.getHistory().then(setItems), []);

  // New readings are added on the Rust side, so refresh whenever one starts
  useEffect(() => {
    refresh();
  }, [refresh, reader.sentences]);

  return (
    <PageLayout
      title="History"
      description="The last 100 things you listened to"
      rightSlot={
        items.length > 0 && (
          <Button variant="outline" size="sm" onClick={() => api.deleteHistory().then(refresh)}>
            <Trash2Icon /> Clear all
          </Button>
        )
      }
    >
      {items.length === 0 ? (
        <p className="text-sm text-muted-foreground">Nothing yet. Everything Sayzo reads shows up here so you can replay it.</p>
      ) : (
        <div className="flex flex-col gap-2">
          {items.map((item) => (
            <Card key={item.id} className="flex flex-row items-start gap-3 px-4 py-3">
              <div className="flex-1 min-w-0 flex flex-col gap-1.5">
                <p dir="auto" className="text-sm line-clamp-2 leading-relaxed">
                  {item.text.trim()}
                </p>
                <div className="flex items-center gap-2 text-[11px] text-muted-foreground select-none">
                  <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                    {SOURCE_LABEL[item.source] ?? item.source}
                  </Badge>
                  {timeAgo(item.at)}
                </div>
              </div>
              <div className="flex gap-1 shrink-0">
                <Button size="icon" variant="ghost" className="size-8" title="Play" onClick={() => api.speak(item.text, { source: item.source })}>
                  <PlayIcon />
                </Button>
                <Button size="icon" variant="ghost" className="size-8" title="Copy" onClick={() => navigator.clipboard.writeText(item.text)}>
                  <CopyIcon />
                </Button>
                <Button size="icon" variant="ghost" className="size-8" title="Delete" onClick={() => api.deleteHistory(item.id).then(refresh)}>
                  <Trash2Icon />
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </PageLayout>
  );
};
