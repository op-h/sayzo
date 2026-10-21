import { useState } from "react";
import { RotateCcwIcon } from "lucide-react";
import { Button, Card, Header, Kbd } from "@/components";
import { useSettings } from "@/hooks";
import { api, SHORTCUT_LABELS } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageLayout } from "./PageLayout";

const DEFAULTS: Record<string, string> = {
  read_selection: "ctrl+shift+space",
  toggle_active: "ctrl+alt+space",
  pause_resume: "ctrl+alt+p",
  stop: "ctrl+alt+s",
  next: "ctrl+alt+period",
  previous: "ctrl+alt+comma",
  ocr: "ctrl+alt+o",
  toggle_window: "ctrl+backslash",
  dashboard: "ctrl+shift+d",
};

const CODE_NAMES: Record<string, string> = {
  Space: "space",
  Backslash: "backslash",
  Period: "period",
  Comma: "comma",
  Slash: "slash",
  Semicolon: "semicolon",
  Quote: "quote",
  BracketLeft: "bracketleft",
  BracketRight: "bracketright",
  Minus: "minus",
  Equal: "equal",
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Enter: "enter",
};

/** Turns a keydown into the accelerator format the Rust side registers, e.g. "ctrl+alt+p" */
const toAccelerator = (e: React.KeyboardEvent): string | null => {
  const code = e.code;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3).toLowerCase();
  else if (/^Digit\d$/.test(code)) key = code.slice(5);
  else if (/^F\d{1,2}$/.test(code)) key = code.toLowerCase();
  else key = CODE_NAMES[code] ?? null;
  if (!key) return null;
  const mods = [e.ctrlKey && "ctrl", e.altKey && "alt", e.shiftKey && "shift"].filter(Boolean) as string[];
  // Plain keys would fire while typing in every app, so require a modifier (function keys excepted)
  if (!mods.length && !key.startsWith("f")) return null;
  return [...mods, key].join("+");
};

const Recorder = ({ value, onChange, failed }: { value?: string; onChange: (v: string) => void; failed: boolean }) => {
  const [recording, setRecording] = useState(false);
  return (
    <button
      onClick={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={async (e) => {
        if (!recording) return;
        e.preventDefault();
        if (e.key === "Escape") return setRecording(false);
        const accel = toAccelerator(e);
        if (accel && (await api.validateShortcut(accel))) {
          onChange(accel);
          setRecording(false);
        }
      }}
      className={cn(
        "min-w-40 h-9 rounded-xl border px-3 text-xs flex items-center justify-center transition-colors",
        recording ? "border-ring ring-2 ring-ring/40 text-muted-foreground" : "border-input/80 hover:bg-accent",
        failed && "border-destructive"
      )}
    >
      {recording ? "Press the new keys… (Esc to cancel)" : value ? <Kbd accel={value} /> : <span className="text-muted-foreground">Not set</span>}
    </button>
  );
};

export const Shortcuts = () => {
  const { settings, update } = useSettings();
  const [failed, setFailed] = useState<string[]>([]);

  const save = async (shortcuts: Record<string, string>) => setFailed(await update({ shortcuts }));

  return (
    <PageLayout
      title="Shortcuts"
      description="Work from anywhere. These keys work in every app while Sayzo is running."
      rightSlot={
        <Button variant="outline" size="sm" onClick={() => save(DEFAULTS)}>
          <RotateCcwIcon /> Reset to defaults
        </Button>
      }
    >
      {failed.length > 0 && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-xl text-sm text-destructive">
          Another app is already using the keys for: {failed.map((f) => SHORTCUT_LABELS[f]?.name ?? f).join(", ")}. Pick different keys.
        </div>
      )}
      <Card className="gap-0 py-1">
        {Object.entries(SHORTCUT_LABELS).map(([id, label], i) => (
          <div key={id} className={cn("flex items-center justify-between gap-4 px-5 py-3", i > 0 && "border-t border-input/40")}>
            <Header title={label.name} description={label.description} />
            <Recorder
              value={settings?.shortcuts[id]}
              failed={failed.includes(id)}
              onChange={(v) => settings && save({ ...settings.shortcuts, [id]: v })}
            />
          </div>
        ))}
      </Card>
    </PageLayout>
  );
};
