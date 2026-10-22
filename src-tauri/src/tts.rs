use crate::arabic::{self, Diacritizer};
use crate::voices::{ImportedVoice, ModelKind, VoiceStore};
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use serde::Serialize;
use sherpa_onnx::{GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig, OfflineTtsVitsModelConfig};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Every voice is resampled to Kokoro's native rate so sentences from
/// different engines can be queued back to back
pub const OUT_SR: u32 = 24_000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub name: String,
    /// ISO 639-1 code of the language this voice speaks
    pub language: String,
    pub accent: String,
    pub gender: String,
    pub recommended: bool,
    pub imported: bool,
}

/// Built-in Kokoro speakers: (speaker id, name, language, accent, gender, recommended)
const KOKORO: &[(i32, &str, &str, &str, &str, bool)] = &[
    (3, "Heart", "en", "American", "female", true),
    (2, "Bella", "en", "American", "female", true),
    (6, "Nicole", "en", "American", "female", false),
    (9, "Sarah", "en", "American", "female", false),
    (0, "Alloy", "en", "American", "female", false),
    (1, "Aoede", "en", "American", "female", false),
    (4, "Jessica", "en", "American", "female", false),
    (5, "Kore", "en", "American", "female", false),
    (7, "Nova", "en", "American", "female", false),
    (8, "River", "en", "American", "female", false),
    (10, "Sky", "en", "American", "female", false),
    (16, "Michael", "en", "American", "male", true),
    (13, "Eric", "en", "American", "male", false),
    (11, "Adam", "en", "American", "male", false),
    (12, "Echo", "en", "American", "male", false),
    (14, "Fenrir", "en", "American", "male", false),
    (15, "Liam", "en", "American", "male", false),
    (17, "Onyx", "en", "American", "male", false),
    (18, "Puck", "en", "American", "male", false),
    (21, "Emma", "en", "British", "female", true),
    (22, "Isabella", "en", "British", "female", false),
    (20, "Alice", "en", "British", "female", false),
    (23, "Lily", "en", "British", "female", false),
    (26, "George", "en", "British", "male", true),
    (25, "Fable", "en", "British", "male", false),
    (24, "Daniel", "en", "British", "male", false),
    (27, "Lewis", "en", "British", "male", false),
    (28, "Dora", "es", "Spain", "female", true),
    (29, "Alex", "es", "Spain", "male", false),
    (30, "Siwis", "fr", "France", "female", true),
    (35, "Sara", "it", "Italy", "female", true),
    (36, "Nicola", "it", "Italy", "male", false),
    (42, "Dora", "pt", "Brazil", "female", true),
    (43, "Alex", "pt", "Brazil", "male", false),
    (44, "Santa", "pt", "Brazil", "male", false),
    (31, "Alpha", "hi", "India", "female", true),
    (32, "Beta", "hi", "India", "female", false),
    (33, "Omega", "hi", "India", "male", false),
    (34, "Psi", "hi", "India", "male", false),
];

/// Built-in Arabic voices: (id, name, gender, what it's best at, recommended)
const ARABIC: &[(&str, &str, &str, &str, bool)] = &[
    ("ar-miro", "Miro", "male", "Best for formal Arabic", true),
    ("ar-kareem", "Kareem", "male", "Best for everyday dialects", true),
    ("ar-dii", "Dii", "female", "Formal Arabic", false),
];

/// Languages written in the Arabic script that fall back to the Arabic voice
const ARABIC_SCRIPT: &[&str] = &["ar", "fa", "ur", "ps", "ku", "ug"];
/// Languages whose script an English voice can't read at all
const NON_LATIN: &[&str] = &[
    "ru", "uk", "be", "bg", "sr", "mk", "kk", "el", "he", "yi", "hi", "mr", "ne", "bn", "pa", "gu", "or", "ta", "te", "kn", "ml",
    "si", "th", "km", "my", "zh", "ja", "ko", "am", "ka", "hy",
];

fn kokoro_lang(sid: i32) -> &'static str {
    match sid {
        20..=27 => "en-gb",
        28 | 29 => "es",
        30 => "fr",
        31..=34 => "hi",
        35 | 36 => "it",
        42..=44 => "pt-br",
        _ => "en-us",
    }
}

/// Which voice reads which language for one read
#[derive(Clone, Default)]
pub struct VoicePlan {
    pub english: String,
    pub arabic: String,
    pub others: BTreeMap<String, String>,
    /// Forces one voice for everything (voice previews)
    pub force: Option<String>,
}

/// What actually loads into memory for a voice
#[derive(Clone)]
enum Spec {
    Kokoro { dir: PathBuf, lang: String, sid: i32, lexicon: Option<PathBuf> },
    Vits { model: PathBuf, tokens: PathBuf, data_dir: Option<PathBuf>, lexicon: Option<PathBuf>, sid: i32 },
}

impl Spec {
    fn key(&self) -> String {
        match self {
            Spec::Kokoro { dir, lang, .. } => format!("kokoro|{}|{lang}", dir.display()),
            Spec::Vits { model, .. } => format!("vits|{}", model.display()),
        }
    }
    fn sid(&self) -> i32 {
        match self {
            Spec::Kokoro { sid, .. } | Spec::Vits { sid, .. } => *sid,
        }
    }
}

type Engine = Arc<Mutex<OfflineTts>>;

/// Most models loaded at once; each Kokoro language is its own instance
const MAX_LOADED: usize = 3;

pub struct Engines {
    dir: PathBuf,
    store: VoiceStore,
    loaded: Mutex<Vec<(String, Engine)>>,
    // Restores the vowels Arabic voices need; loaded on first Arabic read
    diacritizer: OnceCell<Option<Diacritizer>>,
}

fn threads() -> i32 {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) as i32;
    (n / 2).clamp(2, 6)
}

fn path_str(p: &Path) -> Option<String> {
    Some(p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
}

impl Engines {
    pub fn new(dir: PathBuf, store: VoiceStore) -> Self {
        Self { dir, store, loaded: Mutex::new(Vec::new()), diacritizer: OnceCell::new() }
    }

    pub fn store(&self) -> &VoiceStore {
        &self.store
    }

    /// Built-in voices followed by the ones the user imported
    pub fn voices(&self) -> Vec<Voice> {
        let kokoro = KOKORO.iter().map(|(sid, name, lang, accent, gender, rec)| Voice {
            id: format!("en-{sid}"),
            name: name.to_string(),
            language: lang.to_string(),
            accent: accent.to_string(),
            gender: gender.to_string(),
            recommended: *rec,
            imported: false,
        });
        let arabic = ARABIC.iter().map(|(id, name, gender, accent, rec)| Voice {
            id: id.to_string(),
            name: name.to_string(),
            language: "ar".into(),
            accent: accent.to_string(),
            gender: gender.to_string(),
            recommended: *rec,
            imported: false,
        });
        let imported = self.store.list().into_iter().map(|v| Voice {
            id: v.id,
            name: v.name,
            language: v.language,
            accent: v.accent,
            gender: v.gender,
            recommended: false,
            imported: true,
        });
        kokoro.chain(arabic).chain(imported).collect()
    }

    /// Every language some installed voice can speak
    pub fn languages(&self) -> Vec<String> {
        let mut langs: Vec<String> = self.voices().into_iter().map(|v| v.language).collect();
        langs.sort();
        langs.dedup();
        langs
    }

    fn language_of_voice(&self, id: &str) -> Option<String> {
        self.voices().into_iter().find(|v| v.id == id).map(|v| v.language)
    }

    /// The voice that should read text in `lang`, or None when nothing installed can
    pub fn voice_for(&self, lang: &str, plan: &VoicePlan) -> Option<String> {
        if let Some(v) = &plan.force {
            return Some(v.clone());
        }
        let picked = match lang {
            "en" => Some(plan.english.clone()),
            "ar" => Some(plan.arabic.clone()),
            _ => plan.others.get(lang).cloned(),
        };
        if let Some(v) = picked.filter(|v| self.spec(v).is_ok()) {
            return Some(v);
        }
        // Any installed voice for this language beats a fallback
        if let Some(v) = self.voices().into_iter().find(|v| v.language == lang) {
            return Some(v.id);
        }
        if ARABIC_SCRIPT.contains(&lang) {
            Some(plan.arabic.clone())
        } else if NON_LATIN.contains(&lang) {
            None
        } else {
            Some(plan.english.clone())
        }
    }

    fn spec(&self, voice_id: &str) -> Result<Spec> {
        if let Some(sid) = voice_id.strip_prefix("en-").and_then(|s| s.parse::<i32>().ok()) {
            let lang = kokoro_lang(sid);
            let lexicon = match lang {
                "en-us" => Some(self.dir.join("kokoro/lexicon-us-en.txt")),
                "en-gb" => Some(self.dir.join("kokoro/lexicon-gb-en.txt")),
                _ => None,
            };
            return Ok(Spec::Kokoro { dir: self.dir.join("kokoro"), lang: lang.into(), sid, lexicon });
        }
        if ARABIC.iter().any(|(id, ..)| *id == voice_id) {
            return Ok(Spec::Vits {
                model: self.dir.join(voice_id).join("model.onnx"),
                tokens: self.dir.join(voice_id).join("tokens.txt"),
                data_dir: Some(self.dir.join("espeak-ng-data")),
                lexicon: None,
                sid: 0,
            });
        }
        let v: ImportedVoice = self.store.get(voice_id).ok_or_else(|| anyhow!("voice '{voice_id}' is not installed"))?;
        Ok(match v.kind {
            // espeak-ng is one global instance per process: every voice must
            // share the same phoneme data or loading one breaks the others
            ModelKind::Vits => Spec::Vits {
                model: v.dir.join(&v.model),
                tokens: v.dir.join(&v.tokens),
                data_dir: v.data_dir.as_ref().map(|_| self.dir.join("espeak-ng-data")),
                lexicon: v.lexicon.as_ref().map(|d| v.dir.join(d)),
                sid: v.speaker,
            },
            ModelKind::Kokoro => Spec::Kokoro {
                dir: v.dir.clone(),
                lang: v.espeak.clone().unwrap_or_else(|| v.language.clone()),
                sid: v.speaker,
                lexicon: v.lexicon.as_ref().map(|d| v.dir.join(d)),
            },
        })
    }

    fn engine(&self, spec: &Spec) -> Result<Engine> {
        let key = spec.key();
        {
            let mut loaded = self.loaded.lock().unwrap();
            if let Some(pos) = loaded.iter().position(|(k, _)| *k == key) {
                // Most recently used goes to the back
                let item = loaded.remove(pos);
                let engine = item.1.clone();
                loaded.push(item);
                return Ok(engine);
            }
        }
        let mut cfg = OfflineTtsConfig::default();
        cfg.model.num_threads = threads();
        cfg.max_num_sentences = 1;
        match spec {
            Spec::Kokoro { dir, lang, lexicon, .. } => {
                cfg.model.kokoro = OfflineTtsKokoroModelConfig {
                    model: path_str(&dir.join("model.onnx")),
                    voices: path_str(&dir.join("voices.bin")),
                    tokens: path_str(&dir.join("tokens.txt")),
                    data_dir: path_str(&self.espeak_for(dir)),
                    lexicon: lexicon.as_deref().and_then(path_str),
                    lang: Some(lang.clone()),
                    ..Default::default()
                };
            }
            Spec::Vits { model, tokens, data_dir, lexicon, .. } => {
                cfg.model.vits = OfflineTtsVitsModelConfig {
                    model: path_str(model),
                    tokens: path_str(tokens),
                    data_dir: data_dir.as_deref().and_then(path_str),
                    lexicon: lexicon.as_deref().and_then(path_str),
                    ..Default::default()
                };
            }
        }
        let tts = OfflineTts::create(&cfg).ok_or_else(|| anyhow!("failed to load the voice model ({key})"))?;
        let engine = Arc::new(Mutex::new(tts));
        let mut loaded = self.loaded.lock().unwrap();
        if loaded.len() >= MAX_LOADED {
            loaded.remove(0);
        }
        loaded.push((key, engine.clone()));
        Ok(engine)
    }

    /// All voices share one copy of the phoneme data (espeak-ng is process-global)
    fn espeak_for(&self, _kokoro_dir: &Path) -> PathBuf {
        self.dir.join("espeak-ng-data")
    }

    fn diacritizer(&self) -> Option<&Diacritizer> {
        self.diacritizer.get_or_init(|| arabic::load_diacritizer(&self.dir)).as_ref()
    }

    /// Turns plain Arabic into the vowelled, number-free text the Arabic voices expect
    pub fn prepare_arabic(&self, text: &str) -> String {
        let text = arabic::prepare(text);
        match self.diacritizer() {
            Some(d) => d.diacritize(&text).unwrap_or_else(|e| {
                eprintln!("diacritization failed: {e}");
                text
            }),
            None => text,
        }
    }

    /// Loads the models for the chosen voices ahead of time so the first read is instant
    pub fn preload(&self, plan: &VoicePlan) -> Result<()> {
        self.engine(&self.spec(&plan.english)?)?;
        self.engine(&self.spec(&plan.arabic)?)?;
        self.diacritizer();
        Ok(())
    }

    /// Checks that a voice loads and speaks; used right after importing it
    pub fn test_voice(&self, voice_id: &str) -> Result<()> {
        let spec = self.spec(voice_id)?;
        let engine = self.engine(&spec)?;
        let tts = engine.lock().unwrap();
        let cfg = GenerationConfig { sid: spec.sid(), ..Default::default() };
        let audio = tts.generate_with_config("1, 2, 3.", &cfg, None::<fn(&[f32], f32) -> bool>).ok_or_else(|| anyhow!("the voice loaded but produced no audio"))?;
        if audio.samples().is_empty() {
            return Err(anyhow!("the voice loaded but produced no audio"));
        }
        Ok(())
    }

    /// Frees a voice's memory, e.g. before deleting its files
    pub fn unload(&self, voice_id: &str) {
        if let Ok(spec) = self.spec(voice_id) {
            let key = spec.key();
            self.loaded.lock().unwrap().retain(|(k, _)| *k != key);
        }
    }

    /// Synthesizes one single-language run. Returns `Ok(None)` when
    /// `keep_going` turns false mid-generation, which is how a newer read
    /// cancels this one, and an empty clip when no installed voice can read it.
    pub fn synth(
        &self,
        lang: &str,
        text: &str,
        plan: &VoicePlan,
        speed: f32,
        keep_going: impl Fn() -> bool + Send + 'static,
    ) -> Result<Option<Vec<f32>>> {
        let Some(voice) = self.voice_for(lang, plan) else {
            return Ok(Some(Vec::new()));
        };
        let spec = self.spec(&voice)?;
        // Vowels are only restored for actual Arabic read by an Arabic voice
        let prepared;
        let text = if lang == "ar" && self.language_of_voice(&voice).as_deref() == Some("ar") {
            prepared = self.prepare_arabic(text);
            prepared.as_str()
        } else if ARABIC_SCRIPT.contains(&lang) {
            prepared = arabic::prepare(text);
            prepared.as_str()
        } else {
            text
        };
        let engine = self.engine(&spec)?;
        let tts = engine.lock().unwrap();
        if !keep_going() {
            return Ok(None);
        }
        let cfg = GenerationConfig { sid: spec.sid(), speed: speed.clamp(0.5, 2.0), ..Default::default() };
        let audio = tts
            .generate_with_config(text, &cfg, Some(move |_: &[f32], _: f32| keep_going()))
            .ok_or_else(|| anyhow!("speech generation failed"))?;
        let sr = audio.sample_rate() as u32;
        Ok(Some(resample(audio.samples(), sr, OUT_SR)))
    }
}

fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = (input.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx.min(input.len() - 1)];
            let b = input[(idx + 1).min(input.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text;

    fn engines() -> Engines {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Engines::new(root.join("models"), VoiceStore::new(std::env::temp_dir().join("sayzo-test-voices")))
    }

    fn plan() -> VoicePlan {
        VoicePlan { english: "en-3".into(), arabic: "ar-miro".into(), ..Default::default() }
    }

    /// Renders samples in every built-in language with the bundled models
    #[test]
    #[ignore = "needs the voice models; run with --ignored"]
    fn synthesizes_all_languages() {
        let engines = engines();
        let out = std::env::temp_dir();
        for (name, sample) in [
            ("en", "Hello! This is Sayzo. It reads any text you select, with a natural voice, completely offline."),
            ("ar", "مرحبا بك. هذا التطبيق يقرأ أي نص تحدده بصوت طبيعي، بدون الحاجة إلى الإنترنت."),
            ("mixed", "أنا أدرس الأمن السيبراني وأستخدم Kali Linux كل يوم."),
            ("es", "Hola, este programa lee cualquier texto que selecciones con una voz natural."),
            ("fr", "Bonjour, ce programme lit tout texte que vous sélectionnez avec une voix naturelle."),
        ] {
            let start = std::time::Instant::now();
            let mut all = Vec::new();
            for s in text::split(sample) {
                for (lang, seg) in &s.segments {
                    all.extend(engines.synth(lang, seg, &plan(), 1.0, || true).unwrap().unwrap());
                }
            }
            let secs = all.len() as f32 / OUT_SR as f32;
            println!("{name}: {secs:.1}s of audio in {:?}", start.elapsed());
            assert!(secs > 1.0);
            let spec = hound::WavSpec { channels: 1, sample_rate: OUT_SR, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::create(out.join(format!("sayzo-{name}.wav")), spec).unwrap();
            all.iter().for_each(|x| w.write_sample((x.clamp(-1.0, 1.0) * 32767.0) as i16).unwrap());
            w.finalize().unwrap();
        }
    }

    #[test]
    fn routes_languages_to_voices() {
        let e = engines();
        let p = plan();
        assert_eq!(e.voice_for("en", &p).as_deref(), Some("en-3"));
        assert_eq!(e.voice_for("ar", &p).as_deref(), Some("ar-miro"));
        assert_eq!(e.voice_for("es", &p).as_deref(), Some("en-28"));
        assert_eq!(e.voice_for("fa", &p).as_deref(), Some("ar-miro"));
        assert_eq!(e.voice_for("de", &p).as_deref(), Some("en-3"));
        assert_eq!(e.voice_for("ru", &p), None);
    }
}

#[cfg(test)]
mod import {
    use super::*;
    use crate::text;

    /// Imports real voice packages through the same path as the "Add voice" button
    #[test]
    #[ignore = "needs downloaded voice packages; set SAYZO_TEST_PACKAGES"]
    fn imports_and_speaks_packages() {
        let dir = PathBuf::from(std::env::var("SAYZO_TEST_PACKAGES").unwrap());
        let store_dir = std::env::temp_dir().join("sayzo-import-test");
        let _ = std::fs::remove_dir_all(&store_dir);
        let engines = Engines::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"), VoiceStore::new(store_dir.clone()));
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let added = engines.store().import(&entry.path()).unwrap();
            let v = &added[0];
            println!("imported {} -> id={} name={} lang={} accent={}", entry.file_name().to_string_lossy(), v.id, v.name, v.language, v.accent);
            engines.test_voice(&v.id).unwrap();
        }
        let mut plan = VoicePlan { english: "en-3".into(), arabic: "ar-miro".into(), ..Default::default() };
        let langs = engines.languages();
        println!("languages: {langs:?}");
        assert!(langs.contains(&"de".to_string()));
        // German text now goes to the imported German voice
        let s = text::split_for("Heute gehen wir mit der ganzen Familie in den Park.", &langs);
        assert_eq!(s[0].segments[0].0, "de");
        let de = engines.voice_for("de", &plan).unwrap();
        assert!(de.starts_with("user-"));
        let audio = engines.synth("de", &s[0].segments[0].1, &plan, 1.0, || true).unwrap().unwrap();
        assert!(audio.len() > OUT_SR as usize);
        // An imported Arabic accent can be chosen as the Arabic voice
        let kareem = engines.voices().into_iter().find(|v| v.language == "ar" && v.imported).unwrap();
        plan.arabic = kareem.id.clone();
        let audio = engines.synth("ar", "ذهب الولد إلى المدرسة", &plan, 1.0, || true).unwrap().unwrap();
        assert!(audio.len() > OUT_SR as usize / 2);
        engines.store().remove(&kareem.id).unwrap();
        assert!(engines.voices().iter().all(|v| v.id != kareem.id));
    }
}

#[cfg(test)]
mod speed {
    use super::*;

    #[test]
    #[ignore = "needs the voice models; run with --ignored"]
    fn generation_is_faster_than_realtime() {
        let engines = Engines::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"), VoiceStore::new(std::env::temp_dir().join("sayzo-test-voices")));
        let plan = VoicePlan { english: "en-3".into(), arabic: "ar-miro".into(), ..Default::default() };
        let t = std::time::Instant::now();
        engines.preload(&plan).unwrap();
        println!("load both models: {:?}", t.elapsed());
        for (lang, sentence) in [
            ("en", "Security analysts review alerts, investigate incidents, and write clear reports for their team."),
            ("ar", "يقوم محللو الأمن بمراجعة التنبيهات والتحقيق في الحوادث وكتابة تقارير واضحة لفريقهم."),
        ] {
            let t = std::time::Instant::now();
            let a = engines.synth(lang, sentence, &plan, 1.0, || true).unwrap().unwrap();
            let audio = a.len() as f32 / OUT_SR as f32;
            println!("{lang}: {audio:.1}s audio in {:.2}s (rtf {:.2})", t.elapsed().as_secs_f32(), t.elapsed().as_secs_f32() / audio);
        }
    }
}
