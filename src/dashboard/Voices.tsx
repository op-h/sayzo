import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { CheckIcon, ExternalLinkIcon, FolderOpenIcon, Loader2, PackagePlusIcon, PlayIcon, SquareIcon, Trash2Icon } from "lucide-react";
import { Badge, Button, Card, Header, Label, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Slider } from "@/components";
import { useReader, useSettings } from "@/hooks";
import { api, chooseVoice, chosenVoice, LANGUAGE_NAMES, languageName, Voice } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageLayout } from "./PageLayout";

const PREVIEW: Record<string, string> = {
  en: "Hi there! This is how I sound when I read your text aloud.",
  ar: "مرحبا! هكذا يبدو صوتي عندما أقرأ لك النص.",
  es: "¡Hola! Así sueno cuando leo tu texto en voz alta.",
  fr: "Bonjour ! Voici ma voix quand je lis votre texte à haute voix.",
  it: "Ciao! Ecco come suono quando leggo il tuo testo ad alta voce.",
  pt: "Olá! É assim que eu soo quando leio o seu texto em voz alta.",
  hi: "नमस्ते! जब मैं आपका पाठ पढ़ता हूँ तो मेरी आवाज़ ऐसी लगती है।",
  de: "Hallo! So klinge ich, wenn ich deinen Text vorlese.",
  ru: "Привет! Вот так звучит мой голос, когда я читаю ваш текст.",
  tr: "Merhaba! Metninizi sesli okurken sesim böyle duyuluyor.",
  fa: "سلام! وقتی متن شما را می‌خوانم صدایم این‌طور است.",
  nl: "Hallo! Zo klink ik als ik je tekst voorlees.",
  pl: "Cześć! Tak brzmię, kiedy czytam twój tekst na głos.",
  uk: "Привіт! Ось так звучить мій голос, коли я читаю ваш текст.",
};
const previewText = (v: Voice) => PREVIEW[v.language] ?? "1, 2, 3, 4, 5.";

/** Built-in languages first, then imported ones alphabetically */
const ORDER = ["en", "ar", "es", "fr", "it", "pt", "hi"];

export const Voices = () => {
  const { settings, update } = useSettings();
  const reader = useReader();
  const [voices, setVoices] = useState<Voice[]>([]);
  const [previewing, setPreviewing] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [tab, setTab] = useState("en");

  const refresh = () => api.getVoices().then(setVoices);
  useEffect(() => {
    refresh();
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
    api.speak(previewText(v), { voice: v.id, source: "preview" });
  };

  const importFrom = async (directory: boolean) => {
    const picked = await open({
      directory,
      multiple: false,
      title: directory ? "Choose a voice folder" : "Choose a voice package",
      filters: directory ? undefined : [{ name: "Voice package", extensions: ["bz2", "tbz2", "gz", "tgz", "xz", "tar", "zip", "onnx"] }],
    });
    if (typeof picked !== "string") return;
    setImporting(true);
    setMessage(null);
    try {
      const added = await api.importVoice(picked);
      await refresh();
      const lang = added[0]?.language ?? "und";
      setTab(lang);
      setMessage({
        ok: true,
        text:
          `Added ${added.length === 1 ? added[0].name : `${added.length} voices`} for ${languageName(lang)}.` +
          (lang === "und" ? " Choose its language below so Sayzo knows which text it should read." : ""),
      });
    } catch (e) {
      setMessage({ ok: false, text: String(e) });
    } finally {
      setImporting(false);
    }
  };

  const remove = async (v: Voice) => {
    await api.removeVoice(v.id);
    await refresh();
  };

  const setLanguage = async (v: Voice, language: string) => {
    await api.setVoiceLanguage(v.id, language);
    await refresh();
  };

  const languages = useMemo(
    () =>
      [...new Set(voices.map((v) => v.language))].sort((a, b) => {
        const ia = ORDER.indexOf(a), ib = ORDER.indexOf(b);
        if (ia !== -1 || ib !== -1) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
        return languageName(a).localeCompare(languageName(b));
      }),
    [voices]
  );

  /** The voice that actually reads a language: the user's pick, or the first one installed */
  const inUse = (lang: string) => {
    const picked = settings && chosenVoice(settings, lang);
    return voices.some((v) => v.id === picked) ? picked : voices.find((v) => v.language === lang)?.id;
  };

  const groups = useMemo(() => {
    return languages.filter((lang) => lang === tab).flatMap((lang) => {
      const list = voices.filter((v) => v.language === lang);
      if (lang !== "en") return [{ lang, title: languageName(lang), list }];
      // English is split by accent so American and British voices are easy to find
      const accents = [...new Set(list.map((v) => (v.imported ? "Imported" : v.accent)))];
      return accents.map((a) => ({ lang, title: `English · ${a}`, list: list.filter((v) => (v.imported ? "Imported" : v.accent) === a) }));
    });
  }, [voices, languages, tab]);

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

      <Card className="flex flex-col gap-3 px-5 py-4">
        <div className="flex items-center justify-between gap-4">
          <Header
            title="Add voices and languages"
            description="Import any free voice from the sherpa-onnx collection (Piper, Coqui, MMS or Kokoro) in 50+ languages and accents. Download a .tar.bz2 and add it here; it keeps working offline."
          />
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" disabled={importing} onClick={() => importFrom(false)}>
            {importing ? <Loader2 className="animate-spin" /> : <PackagePlusIcon />}
            {importing ? "Adding voice…" : "Add voice package…"}
          </Button>
          <Button size="sm" variant="outline" disabled={importing} onClick={() => importFrom(true)}>
            <FolderOpenIcon />
            Add from folder…
          </Button>
          <Button size="sm" variant="ghost" onClick={() => api.openVoiceLibrary()}>
            <ExternalLinkIcon />
            Browse free voices
          </Button>
        </div>
        {message && <p className={cn("text-xs", message.ok ? "text-green-600 dark:text-green-400" : "text-destructive")}>{message.text}</p>}
      </Card>

      <div className="flex flex-wrap gap-1.5">
        {languages.map((lang) => (
          <button
            key={lang}
            onClick={() => setTab(lang)}
            className={cn(
              "rounded-full border px-3 py-1 text-xs transition-colors",
              tab === lang ? "border-primary bg-primary text-primary-foreground" : "border-input/60 text-muted-foreground hover:bg-accent/50 hover:text-foreground"
            )}
          >
            {languageName(lang)}
            <span className="ml-1.5 opacity-60 tabular-nums">{voices.filter((v) => v.language === lang).length}</span>
          </button>
        ))}
      </div>

      {settings &&
        groups.map((g) => (
          <div key={g.title} className="flex flex-col gap-2">
            <Header title={g.title} description={g.lang === "und" ? "Choose a language for these voices" : `Used for ${languageName(g.lang)} text`} />
            <div className="grid grid-cols-3 gap-2">
              {g.list.map((v) => {
                const selected = inUse(v.language) === v.id;
                return (
                  <div
                    key={v.id}
                    onClick={() => v.language !== "und" && update(chooseVoice(settings, v.language, v.id))}
                    className={cn(
                      "flex items-center justify-between gap-2 rounded-xl border px-3 py-2 cursor-pointer transition-colors hover:bg-accent/50",
                      selected ? "border-primary bg-accent/40" : "border-input/50"
                    )}
                  >
                    <div className="flex flex-col min-w-0">
                      <div className="flex items-center gap-1.5 text-sm font-medium">
                        {selected && <CheckIcon className="size-3.5 shrink-0" />}
                        <span className="truncate" title={v.name}>
                          {v.name}
                        </span>
                        {v.recommended && (
                          <Badge variant="secondary" className="text-[9px] px-1 py-0">
                            Best
                          </Badge>
                        )}
                      </div>
                      <span className="text-[11px] text-muted-foreground capitalize truncate">
                        {[g.lang === "en" ? "" : v.accent, v.gender].filter(Boolean).join(" · ") || (v.imported ? "Imported" : "")}
                      </span>
                      {v.language === "und" && (
                        <div onClick={(e) => e.stopPropagation()} className="mt-1">
                          <Select onValueChange={(lang) => setLanguage(v, lang)}>
                            <SelectTrigger className="h-7 text-xs">
                              <SelectValue placeholder="Language…" />
                            </SelectTrigger>
                            <SelectContent>
                              {Object.entries(LANGUAGE_NAMES)
                                .sort((a, b) => a[1].localeCompare(b[1]))
                                .map(([code, name]) => (
                                  <SelectItem key={code} value={code}>
                                    {name}
                                  </SelectItem>
                                ))}
                            </SelectContent>
                          </Select>
                        </div>
                      )}
                    </div>
                    <div className="flex shrink-0">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="size-8"
                        title="Preview"
                        onClick={(e) => {
                          e.stopPropagation();
                          preview(v);
                        }}
                      >
                        {previewing === v.id ? <SquareIcon /> : <PlayIcon />}
                      </Button>
                      {v.imported && (
                        <Button
                          size="icon"
                          variant="ghost"
                          className="size-8 text-muted-foreground hover:text-destructive"
                          title="Remove this voice"
                          onClick={(e) => {
                            e.stopPropagation();
                            remove(v);
                          }}
                        >
                          <Trash2Icon />
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
    </PageLayout>
  );
};
