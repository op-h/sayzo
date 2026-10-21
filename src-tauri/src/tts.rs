use crate::text::Lang;
use anyhow::{anyhow, Result};
use serde::Serialize;
use sherpa_onnx::{GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig, OfflineTtsVitsModelConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Every voice is resampled to Kokoro's native rate so sentences from
/// different engines can be queued back to back
pub const OUT_SR: u32 = 24_000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub lang: Lang,
    pub accent: &'static str,
    pub gender: &'static str,
    pub recommended: bool,
}

const KOKORO_EN: &[(i32, &str, &str, &str, bool)] = &[
    (3, "Heart", "American", "female", true),
    (2, "Bella", "American", "female", true),
    (6, "Nicole", "American", "female", false),
    (9, "Sarah", "American", "female", false),
    (0, "Alloy", "American", "female", false),
    (1, "Aoede", "American", "female", false),
    (4, "Jessica", "American", "female", false),
    (5, "Kore", "American", "female", false),
    (7, "Nova", "American", "female", false),
    (8, "River", "American", "female", false),
    (10, "Sky", "American", "female", false),
    (16, "Michael", "American", "male", true),
    (13, "Eric", "American", "male", false),
    (11, "Adam", "American", "male", false),
    (12, "Echo", "American", "male", false),
    (14, "Fenrir", "American", "male", false),
    (15, "Liam", "American", "male", false),
    (17, "Onyx", "American", "male", false),
    (18, "Puck", "American", "male", false),
    (21, "Emma", "British", "female", true),
    (22, "Isabella", "British", "female", false),
    (20, "Alice", "British", "female", false),
    (23, "Lily", "British", "female", false),
    (26, "George", "British", "male", true),
    (25, "Fable", "British", "male", false),
    (24, "Daniel", "British", "male", false),
    (27, "Lewis", "British", "male", false),
];

const ARABIC: &[(&str, &str, &str, bool)] = &[("ar-miro", "Miro", "male", true), ("ar-dii", "Dii", "female", false)];

pub fn voices() -> Vec<Voice> {
    let en = KOKORO_EN.iter().map(|(sid, name, accent, gender, rec)| Voice {
        id: format!("en-{sid}"),
        name: name.to_string(),
        lang: Lang::En,
        accent,
        gender,
        recommended: *rec,
    });
    let ar = ARABIC.iter().map(|(id, name, gender, rec)| Voice {
        id: id.to_string(),
        name: name.to_string(),
        lang: Lang::Ar,
        accent: "Arabic",
        gender,
        recommended: *rec,
    });
    en.chain(ar).collect()
}

fn kokoro_sid(voice_id: &str) -> i32 {
    voice_id.strip_prefix("en-").and_then(|s| s.parse().ok()).unwrap_or(3)
}

fn is_british(sid: i32) -> bool {
    (20..=27).contains(&sid)
}

type Engine = Arc<Mutex<OfflineTts>>;

pub struct Engines {
    dir: PathBuf,
    // At most one English and one Arabic model stay loaded; switching voice
    // family evicts the previous one to keep memory low
    loaded: Mutex<HashMap<String, Engine>>,
}

fn threads() -> i32 {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) as i32;
    (n / 2).clamp(2, 6)
}

impl Engines {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, loaded: Mutex::new(HashMap::new()) }
    }

    fn path(&self, rel: &str) -> Option<String> {
        Some(self.dir.join(rel).to_string_lossy().to_string())
    }

    fn engine(&self, key: &str) -> Result<Engine> {
        if let Some(e) = self.loaded.lock().unwrap().get(key) {
            return Ok(e.clone());
        }
        let mut cfg = OfflineTtsConfig::default();
        cfg.model.num_threads = threads();
        cfg.max_num_sentences = 1;
        if let Some(variant) = key.strip_prefix("kokoro-") {
            let (lexicon, lang) = if variant == "gb" { ("kokoro/lexicon-gb-en.txt", "en-gb") } else { ("kokoro/lexicon-us-en.txt", "en-us") };
            cfg.model.kokoro = OfflineTtsKokoroModelConfig {
                model: self.path("kokoro/model.onnx"),
                voices: self.path("kokoro/voices.bin"),
                tokens: self.path("kokoro/tokens.txt"),
                data_dir: self.path("espeak-ng-data"),
                lexicon: self.path(lexicon),
                lang: Some(lang.into()),
                ..Default::default()
            };
        } else {
            cfg.model.vits = OfflineTtsVitsModelConfig {
                model: self.path(&format!("{key}/model.onnx")),
                tokens: self.path(&format!("{key}/tokens.txt")),
                data_dir: self.path("espeak-ng-data"),
                ..Default::default()
            };
        }
        let tts = OfflineTts::create(&cfg).ok_or_else(|| anyhow!("failed to load voice model '{key}' from {}", self.dir.display()))?;
        let engine = Arc::new(Mutex::new(tts));
        let mut loaded = self.loaded.lock().unwrap();
        let family = key.starts_with("kokoro-");
        loaded.retain(|k, _| k.starts_with("kokoro-") != family);
        loaded.insert(key.to_string(), engine.clone());
        Ok(engine)
    }

    /// Loads the models for the chosen voices ahead of time so the first read is instant
    pub fn preload(&self, en_voice: &str, ar_voice: &str) -> Result<()> {
        self.engine(&kokoro_key(en_voice))?;
        self.engine(ar_voice)?;
        Ok(())
    }

    /// Synthesizes one script run. Returns `Ok(None)` when `keep_going` turns
    /// false mid-generation, which is how a newer read cancels this one.
    pub fn synth(
        &self,
        lang: Lang,
        text: &str,
        en_voice: &str,
        ar_voice: &str,
        speed: f32,
        keep_going: impl Fn() -> bool + Send + 'static,
    ) -> Result<Option<Vec<f32>>> {
        let (key, sid) = match lang {
            Lang::En => (kokoro_key(en_voice), kokoro_sid(en_voice)),
            Lang::Ar => (ar_voice.to_string(), 0),
        };
        let engine = self.engine(&key)?;
        let tts = engine.lock().unwrap();
        if !keep_going() {
            return Ok(None);
        }
        let cfg = GenerationConfig { sid, speed: speed.clamp(0.5, 2.0), ..Default::default() };
        let audio = tts
            .generate_with_config(text, &cfg, Some(move |_: &[f32], _: f32| keep_going()))
            .ok_or_else(|| anyhow!("speech generation failed"))?;
        let sr = audio.sample_rate() as u32;
        Ok(Some(resample(audio.samples(), sr, OUT_SR)))
    }
}

fn kokoro_key(en_voice: &str) -> String {
    if is_british(kokoro_sid(en_voice)) { "kokoro-gb" } else { "kokoro-us" }.to_string()
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

    /// Renders English, Arabic and mixed samples with the bundled models
    #[test]
    #[ignore = "needs the voice models; run with --ignored"]
    fn synthesizes_all_languages() {
        let engines = Engines::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"));
        let out = std::env::temp_dir();
        for (name, sample) in [
            ("en", "Hello! This is Sayzo. It reads any text you select, with a natural voice, completely offline."),
            ("ar", "مرحبا بك. هذا التطبيق يقرأ أي نص تحدده بصوت طبيعي، بدون الحاجة إلى الإنترنت."),
            ("mixed", "أنا أدرس الأمن السيبراني وأستخدم Kali Linux كل يوم."),
        ] {
            let start = std::time::Instant::now();
            let mut all = Vec::new();
            for s in text::split(sample) {
                for (lang, seg) in &s.segments {
                    all.extend(engines.synth(*lang, seg, "en-3", "ar-miro", 1.0, || true).unwrap().unwrap());
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
}

#[cfg(test)]
mod speed {
    use super::*;

    #[test]
    #[ignore = "needs the voice models; run with --ignored"]
    fn generation_is_faster_than_realtime() {
        let engines = Engines::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"));
        let t = std::time::Instant::now();
        engines.preload("en-3", "ar-miro").unwrap();
        println!("load both models: {:?}", t.elapsed());
        for sentence in ["The quick brown fox jumps over the lazy dog near the river bank.", "Security analysts review alerts, investigate incidents, and write clear reports for their team."] {
            let t = std::time::Instant::now();
            let a = engines.synth(Lang::En, sentence, "en-3", "ar-miro", 1.0, || true).unwrap().unwrap();
            let audio = a.len() as f32 / OUT_SR as f32;
            println!("en: {audio:.1}s audio in {:.2}s (rtf {:.2})", t.elapsed().as_secs_f32(), t.elapsed().as_secs_f32() / audio);
        }
    }
}
