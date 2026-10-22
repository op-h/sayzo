import { useEffect, useState } from "react";
import { SlidersHorizontalIcon } from "lucide-react";
import { Button, Label, Popover, PopoverContent, PopoverTrigger, Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue, Slider, Switch } from "@/components";
import { api, Settings, Voice } from "@/lib/api";

interface Props {
  settings: Settings | null;
  update: (patch: Partial<Settings>) => Promise<string[]>;
}

export const voiceLabel = (v: Voice) => `${v.name} · ${v.accent} ${v.gender}${v.recommended ? " ★" : ""}`;

export const VoiceSelect = ({ voices, value, onChange }: { voices: Voice[]; value?: string; onChange: (id: string) => void }) => {
  const groups = voices.reduce<Record<string, Voice[]>>((acc, v) => {
    const key = [v.imported ? "Imported" : v.accent, v.gender].filter(Boolean).join(" ");
    (acc[key] ??= []).push(v);
    return acc;
  }, {});
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger className="w-full">
        <SelectValue placeholder="Choose a voice" />
      </SelectTrigger>
      <SelectContent>
        {Object.entries(groups).map(([group, list]) => (
          <SelectGroup key={group}>
            <SelectLabel className="capitalize">{group}</SelectLabel>
            {list.map((v) => (
              <SelectItem key={v.id} value={v.id}>
                {v.name}
                {v.recommended ? " ★" : ""}
              </SelectItem>
            ))}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  );
};

/** Voice, speed and volume without opening the dashboard */
export const QuickSettings = ({ settings, update }: Props) => {
  const [voices, setVoices] = useState<Voice[]>([]);
  useEffect(() => {
    api.getVoices().then(setVoices);
  }, []);

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size="icon" title="Voice & speed">
          <SlidersHorizontalIcon className="h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" side="bottom" className="w-80 select-none" sideOffset={8}>
        {settings && (
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs">English voice</Label>
              <VoiceSelect voices={voices.filter((v) => v.language === "en")} value={settings.englishVoice} onChange={(englishVoice) => update({ englishVoice })} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs">Arabic voice</Label>
              <VoiceSelect voices={voices.filter((v) => v.language === "ar")} value={settings.arabicVoice} onChange={(arabicVoice) => update({ arabicVoice })} />
            </div>
            <div className="flex flex-col gap-2">
              <div className="flex justify-between">
                <Label className="text-xs">Speed</Label>
                <span className="text-xs text-muted-foreground tabular-nums">{settings.speed.toFixed(2)}×</span>
              </div>
              <Slider min={0.5} max={2} step={0.05} value={[settings.speed]} onValueChange={([speed]) => update({ speed })} />
            </div>
            <div className="flex flex-col gap-2">
              <div className="flex justify-between">
                <Label className="text-xs">Volume</Label>
                <span className="text-xs text-muted-foreground tabular-nums">{Math.round(settings.volume * 100)}%</span>
              </div>
              <Slider min={0} max={1.5} step={0.05} value={[settings.volume]} onValueChange={([volume]) => update({ volume })} />
            </div>
            <div className="flex items-center justify-between">
              <Label className="text-xs">Open reading panel when reading starts</Label>
              <Switch checked={settings.showPanel} onCheckedChange={(showPanel) => update({ showPanel })} />
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
};
