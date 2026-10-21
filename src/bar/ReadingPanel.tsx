import { useEffect, useRef, useState } from "react";
import { CheckIcon, CopyIcon, DownloadIcon, Loader2, RotateCcwIcon, XIcon } from "lucide-react";
import { Badge, Button, Kbd, ScrollArea } from "@/components";
import { api, Settings, Snapshot } from "@/lib/api";
import { cn } from "@/lib/utils";

interface Props {
  reader: Snapshot;
  settings: Settings | null;
  onClose: () => void;
}

/** Full text of the current reading with the audible sentence highlighted; click any sentence to jump there */
export const ReadingPanel = ({ reader, settings, onClose }: Props) => {
  const refs = useRef<(HTMLSpanElement | null)[]>([]);
  const [copied, setCopied] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  const fullText = reader.sentences.map((s) => s.text).join(" ");
  const rtl = reader.sentences[0]?.lang === "ar";
  const idle = reader.status === "idle";

  useEffect(() => {
    refs.current[reader.current]?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [reader.current]);

  const copy = async () => {
    await navigator.clipboard.writeText(fullText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const save = async () => {
    setSaving(true);
    try {
      setSaved(await api.exportAudio(fullText));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
        <div className="flex flex-row gap-2 items-center select-none">
          <h3 className="font-semibold text-xs">{idle ? "Last reading" : "Now reading"}</h3>
          {reader.total > 0 && (
            <Badge variant="outline" className="text-[10px] px-1.5 py-0">
              {reader.current + 1} / {reader.total}
            </Badge>
          )}
          <div className="text-[10px] text-muted-foreground/70 flex items-center gap-1">
            Pause <Kbd accel={settings?.shortcuts.pause_resume} />
          </div>
        </div>
        <div className="flex items-center gap-1 select-none">
          {idle && reader.total > 0 && (
            <Button size="icon" variant="ghost" title="Read again" onClick={() => api.speak(fullText, { source: reader.source })}>
              <RotateCcwIcon />
            </Button>
          )}
          <Button size="icon" variant="ghost" title="Save as audio file (WAV)" disabled={!reader.total || saving} onClick={save}>
            {saving ? <Loader2 className="animate-spin" /> : <DownloadIcon />}
          </Button>
          <Button size="icon" variant="ghost" title="Copy text" disabled={!reader.total} onClick={copy}>
            {copied ? <CheckIcon /> : <CopyIcon />}
          </Button>
          <Button size="icon" variant="ghost" title="Close" onClick={onClose}>
            <XIcon />
          </Button>
        </div>
      </div>

      <ScrollArea className="h-[calc(100vh-7rem)]">
        <div className="p-4">
          {reader.error && (
            <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded text-sm text-destructive">
              <strong>Error:</strong> {reader.error}
            </div>
          )}
          {saved && (
            <div className="mb-4 p-2 bg-muted/50 border rounded text-xs text-muted-foreground break-all">Saved to {saved}</div>
          )}
          {reader.total === 0 ? (
            <p className="text-sm text-muted-foreground select-none">
              Select text in any app and press <Kbd accel={settings?.shortcuts.read_selection} />, or turn on auto-read and just select it.
            </p>
          ) : (
            <p dir={rtl ? "rtl" : "ltr"} className={cn("leading-7", rtl ? "text-[17px]" : "text-sm")}>
              {reader.sentences.map((s, i) => (
                <span
                  key={i}
                  ref={(el) => {
                    refs.current[i] = el;
                  }}
                  dir="auto"
                  onClick={() => api.seek(i)}
                  className={cn(
                    "cursor-pointer rounded px-0.5 transition-colors hover:bg-accent",
                    i === reader.current && !idle
                      ? "bg-primary/10 text-foreground ring-1 ring-primary/20"
                      : i < reader.current && !idle
                        ? "text-muted-foreground"
                        : "text-foreground/90"
                  )}
                >
                  {s.text}{" "}
                </span>
              ))}
            </p>
          )}
        </div>
      </ScrollArea>
    </>
  );
};
