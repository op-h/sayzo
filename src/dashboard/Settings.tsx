import { useEffect, useState } from "react";
import { GithubIcon } from "lucide-react";
import { Card, Header, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Slider, Switch } from "@/components";
import { Theme, useSettings, useTheme } from "@/hooks";
import { api, Settings } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageLayout } from "./PageLayout";

const Row = ({ title, description, children, first }: { title: string; description: string; children: React.ReactNode; first?: boolean }) => (
  <div className={cn("flex items-center justify-between gap-6 px-5 py-3", !first && "border-t border-input/40")}>
    <Header title={title} description={description} />
    {children}
  </div>
);

const TOGGLES: { key: keyof Settings; title: string; description: string }[] = [
  { key: "alwaysOnTop", title: "Always on top", description: "Keep the floating bar above other windows" },
  { key: "showPanel", title: "Open reading panel automatically", description: "Show the full text with the current sentence highlighted when reading starts" },
  { key: "restoreClipboard", title: "Restore clipboard", description: "When text has to be copied to read it, put back what was on your clipboard" },
  { key: "hideFromCapture", title: "Hide from screen sharing", description: "Sayzo's windows won't appear in screenshots, recordings or meetings" },
];

export const SettingsPage = () => {
  const { settings, update } = useSettings();
  const { theme, setTheme, transparency, setTransparency } = useTheme();
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    api.getAutostart().then(setAutostart);
  }, []);

  return (
    <PageLayout title="Settings" description="Appearance and behavior">
      <Card className="gap-0 py-1">
        <Row first title="Theme" description="Light, dark, or follow Windows">
          <Select value={theme} onValueChange={(t) => setTheme(t as Theme)}>
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="system">System</SelectItem>
              <SelectItem value="light">Light</SelectItem>
              <SelectItem value="dark">Dark</SelectItem>
            </SelectContent>
          </Select>
        </Row>
        <Row title="Transparency" description="How much the bar lets the screen behind it show through">
          <div className="flex items-center gap-3 w-48">
            <Slider min={0} max={80} step={5} value={[transparency]} onValueChange={([v]) => setTransparency(v)} />
            <span className="text-xs text-muted-foreground w-8 tabular-nums">{transparency}%</span>
          </div>
        </Row>
      </Card>

      <Card className="gap-0 py-1">
        <Row first title="Launch at startup" description="Start Sayzo in the background when you sign in to Windows">
          <Switch
            checked={autostart}
            onCheckedChange={async (on) => {
              await api.setAutostart(on);
              setAutostart(on);
            }}
          />
        </Row>
        {settings &&
          TOGGLES.map((t) => (
            <Row key={t.key} title={t.title} description={t.description}>
              <Switch checked={settings[t.key] as boolean} onCheckedChange={(on) => update({ [t.key]: on })} />
            </Row>
          ))}
      </Card>

      <Card className="gap-0 py-1">
        <Row first title="About Sayzo" description="Designed and developed by OPH">
          <button
            onClick={() => api.openAuthorPage()}
            className="flex items-center gap-2 rounded-xl border border-input/60 px-3 py-1.5 text-xs hover:bg-accent transition-colors"
          >
            <GithubIcon className="size-3.5" />
            github.com/op-h
          </button>
        </Row>
      </Card>

      <p className="text-[11px] text-muted-foreground leading-relaxed select-none">
        Everything runs on this PC; no text or audio ever leaves it. English voices: Kokoro-82M (Apache-2.0). Arabic voices: Piper / OpenVoiceOS.
        Speech engine: sherpa-onnx. Interface design based on Pluely (GPL-3.0).
      </p>
    </PageLayout>
  );
};
