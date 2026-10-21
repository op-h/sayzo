import { useEffect, useState } from "react";
import { CheckIcon, PlayIcon, SquareIcon } from "lucide-react";
import { Badge, Button, Card, Header, Label, Slider } from "@/components";
import { useReader, useSettings } from "@/hooks";
import { api, Voice } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageLayout } from "./PageLayout";

const PREVIEW = {
  en: "Hi there! This is how I sound when I read your text aloud.",
  ar: "مرحبا! هكذا يبدو صوتي عندما أقرأ لك النص.",
};

export const Voices = () => {
  const { settings, update } = useSettings();
  const reader = useReader();
  const [voices, setVoices] = useState<Voice[]>([]);
  const [previewing, setPreviewing] = useState<string | null>(null);

  useEffect(() => {
    api.getVoices().then(setVoices);
  }, []);
  useEffect(() => {
    if (reader.status === "idle") setPreviewing(null);
  }, [reader.status]);

  const preview = (v: Voice) => {
    if (previewing === v.id) {
      api.control("stop");
      return;
    }
    setPreviewing(v.id);
    api.speak(PREVIEW[v.lang], { voice: v.id, source: "preview" });
  };

  const selected = (v: Voice) => (v.lang === "en" ? settings?.englishVoice : settings?.arabicVoice) === v.id;
  const select = (v: Voice) => update(v.lang === "en" ? { englishVoice: v.id } : { arabicVoice: v.id });

  const groups = [
    { title: "English · American", list: voices.filter((v) => v.lang === "en" && v.accent === "American") },
    { title: "English · British", list: voices.filter((v) => v.lang === "en" && v.accent === "British") },
    { title: "Arabic", list: voices.filter((v) => v.lang === "ar") },
  ];

  return (
    <PageLayout title="Voices" description="Natural neural voices that run on your PC. Text in each language is read by the voice you pick for it.">
      {settings && (
        <Card className="grid grid-cols-2 gap-8 px-5 py-4">
          <div className="flex flex-col gap-3">
            <div className="flex justify-between">
              <Label className="text-sm">Speed</Label>
              <span className="text-xs text-muted-foreground tabular-nums">{settings.speed.toFixed(2)}×</span>
            </div>
            <Slider min={0.5} max={2} step={0.05} value={[settings.speed]} onValueChange={([speed]) => update({ speed })} />
          </div>
          <div className="flex flex-col gap-3">
            <div className="flex justify-between">
              <Label className="text-sm">Volume</Label>
              <span className="text-xs text-muted-foreground tabular-nums">{Math.round(settings.volume * 100)}%</span>
            </div>
            <Slider min={0} max={1.5} step={0.05} value={[settings.volume]} onValueChange={([volume]) => update({ volume })} />
          </div>
        </Card>
      )}

      {groups.map((g) => (
        <div key={g.title} className="flex flex-col gap-2">
          <Header title={g.title} description={g.list[0]?.lang === "ar" ? "Used for Arabic text" : "Used for English text"} />
          <div className="grid grid-cols-3 gap-2">
            {g.list.map((v) => (
              <div
                key={v.id}
                onClick={() => select(v)}
                className={cn(
                  "flex items-center justify-between gap-2 rounded-xl border px-3 py-2 cursor-pointer transition-colors hover:bg-accent/50",
                  selected(v) ? "border-primary bg-accent/40" : "border-input/50"
                )}
              >
                <div className="flex flex-col min-w-0">
                  <div className="flex items-center gap-1.5 text-sm font-medium">
                    {selected(v) && <CheckIcon className="size-3.5 shrink-0" />}
                    <span className="truncate">{v.name}</span>
                    {v.recommended && (
                      <Badge variant="secondary" className="text-[9px] px-1 py-0">
                        Best
                      </Badge>
                    )}
                  </div>
                  <span className="text-[11px] text-muted-foreground capitalize">{v.gender}</span>
                </div>
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-8 shrink-0"
                  title="Preview"
                  onClick={(e) => {
                    e.stopPropagation();
                    preview(v);
                  }}
                >
                  {previewing === v.id ? <SquareIcon /> : <PlayIcon />}
                </Button>
              </div>
            ))}
          </div>
        </div>
      ))}
    </PageLayout>
  );
};
