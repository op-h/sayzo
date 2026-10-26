import { useState } from "react";
import { DownloadIcon, Loader2, MousePointerClickIcon, PenLineIcon, PlayIcon, ScanTextIcon, SquareIcon, TextSelectIcon } from "lucide-react";
import { Badge, Button, Card, Header, Kbd, Switch, Textarea } from "@/components";
import { useEngineStatus, useReader, useSettings } from "@/hooks";
import { api } from "@/lib/api";
import { PageLayout } from "./PageLayout";

const SAMPLE = `Sayzo reads any text on your screen with a natural voice, and it works completely offline.
يقرأ Sayzo أي نص على شاشتك بصوت طبيعي، ويعمل بدون إنترنت.`;

export const Home = () => {
  const { settings } = useSettings();
  const engine = useEngineStatus();
  const reader = useReader();
  const [text, setText] = useState(SAMPLE);
  const [exporting, setExporting] = useState(false);
  const busy = reader.status !== "idle";

  const ways = [
    {
      icon: MousePointerClickIcon,
      title: "Auto-read on select",
      body: "Turn it on, then just select text with the mouse in any app. Double-click reads a word, triple-click a paragraph.",
      accel: settings?.shortcuts.toggle_active,
    },
    {
      icon: TextSelectIcon,
      title: "Read the selection",
      body: "Select text anywhere and press the shortcut. Press it again with nothing selected to stop.",
      accel: settings?.shortcuts.read_selection,
    },
    {
      icon: ScanTextIcon,
      title: "Read a screen area",
      body: "For text you can't select: images, videos, scanned PDFs, games. Drag a box around it.",
      accel: settings?.shortcuts.ocr,
    },
    {
      icon: PenLineIcon,
      title: "Draw on screen",
      body: "Pen, highlighter, laser, shapes, text and steps over any app. Box text with the Read tool to hear it.",
      accel: settings?.shortcuts.draw,
      action: () => api.drawOpen(),
    },
  ];

  return (
    <PageLayout
      title="Sayzo"
      description="Natural, offline read-aloud for any text on your screen"
      rightSlot={
        <Badge variant={engine.error ? "destructive" : "outline"} className="gap-1.5">
          {engine.ready ? (
            <span className="size-2 rounded-full bg-green-500" />
          ) : engine.error ? null : (
            <Loader2 className="size-3 animate-spin" />
          )}
          {engine.ready ? "Voices ready · offline" : engine.error ? "Voice models missing" : "Loading voices…"}
        </Badge>
      }
    >
      {engine.error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-xl text-sm text-destructive">{engine.error}</div>
      )}

      <Card className="flex flex-row items-center justify-between gap-4 px-5 py-4">
        <Header title="Auto-read on select" description="When on, any text you select with the mouse is read aloud right away." />
        <Switch checked={settings?.autoRead ?? false} onCheckedChange={(on) => api.setAutoRead(on)} />
      </Card>

      <div className="grid grid-cols-2 gap-3">
        {ways.map((w) => (
          <Card key={w.title} className="gap-2 px-4 py-4">
            <div className="flex items-center gap-2">
              <div className="flex size-7 items-center justify-center rounded-lg bg-primary">
                <w.icon className="size-4 text-primary-foreground" />
              </div>
              <h3 className="text-sm font-medium">{w.title}</h3>
            </div>
            <p className="text-xs text-muted-foreground leading-relaxed">{w.body}</p>
            <div className="mt-1 flex items-center justify-between">
              <Kbd accel={w.accel} />
              {"action" in w && w.action && (
                <Button size="sm" variant="outline" className="h-7 text-xs" onClick={w.action}>
                  Start drawing
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>

      <div className="flex flex-col gap-2">
        <Header title="Try it" description="Type or paste anything. The language is detected automatically, even when languages are mixed in one sentence." />
        <Textarea dir="auto" value={text} onChange={(e) => setText(e.target.value)} className="min-h-32 text-sm" />
        <div className="flex gap-2">
          {busy ? (
            <Button onClick={() => api.control("stop")}>
              <SquareIcon /> Stop
            </Button>
          ) : (
            <Button disabled={!text.trim() || !engine.ready} onClick={() => api.speak(text, { source: "typed" })}>
              <PlayIcon /> Listen
            </Button>
          )}
          <Button
            variant="outline"
            disabled={!text.trim() || exporting || !engine.ready}
            onClick={async () => {
              setExporting(true);
              try {
                await api.exportAudio(text);
              } finally {
                setExporting(false);
              }
            }}
          >
            {exporting ? <Loader2 className="animate-spin" /> : <DownloadIcon />} Save as audio
          </Button>
        </div>
      </div>
    </PageLayout>
  );
};
