//! Voices the user adds themselves.
//!
//! Any speech model packaged for sherpa-onnx works: the thousands of Piper,
//! Coqui, MMS and Kokoro voices published at
//! https://github.com/k2-fsa/sherpa-onnx/releases/tag/tts-models, in 50+
//! languages. The user picks the downloaded archive (or an extracted folder);
//! it's unpacked into the app's data folder and described by a `voice.json`.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tract_onnx::prelude::Framework;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    Vits,
    Kokoro,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportedVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub accent: String,
    pub gender: String,
    pub kind: ModelKind,
    /// Files relative to `dir`
    pub model: String,
    pub tokens: String,
    pub data_dir: Option<String>,
    pub lexicon: Option<String>,
    /// espeak-ng language for Kokoro packages
    pub espeak: Option<String>,
    pub speaker: i32,
    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Clone)]
pub struct VoiceStore {
    root: PathBuf,
}

impl VoiceStore {
    pub fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    /// Every imported voice; a multi-speaker package contributes one entry per speaker
    pub fn list(&self) -> Vec<ImportedVoice> {
        let mut out = Vec::new();
        let Ok(dirs) = fs::read_dir(&self.root) else { return out };
        for d in dirs.flatten().filter(|d| d.path().is_dir()) {
            let path = d.path().join("voice.json");
            if let Some(list) = fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<Vec<ImportedVoice>>(&s).ok()) {
                out.extend(list.into_iter().map(|mut v| {
                    v.dir = d.path();
                    v
                }));
            }
        }
        out.sort_by(|a, b| (&a.language, &a.name).cmp(&(&b.language, &b.name)));
        out
    }

    pub fn get(&self, id: &str) -> Option<ImportedVoice> {
        self.list().into_iter().find(|v| v.id == id)
    }

    /// Unpacks and registers a voice package; returns the new voices
    pub fn import(&self, source: &Path) -> Result<Vec<ImportedVoice>> {
        let staging = self.root.join(format!("_incoming-{}", std::process::id()));
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir_all(&staging)?;
        let result = self.import_into(source, &staging);
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn import_into(&self, source: &Path, staging: &Path) -> Result<Vec<ImportedVoice>> {
        let name = source.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if source.is_dir() {
            copy_dir(source, staging)?;
        } else if name.ends_with(".onnx") {
            // A bare model file: bring its whole folder (tokens, phoneme data)
            copy_dir(source.parent().ok_or_else(|| anyhow!("can't read the model's folder"))?, staging)?;
        } else if [".tar.bz2", ".tbz2", ".tar.gz", ".tgz", ".tar.xz", ".tar", ".zip"].iter().any(|e| name.ends_with(e)) {
            // Windows' built-in tar (bsdtar) understands all of these
            let status = Command::new("tar")
                .arg("-xf")
                .arg(source)
                .arg("-C")
                .arg(staging)
                .creation_flags_no_window()
                .status()
                .context("couldn't run the archive extractor")?;
            if !status.success() {
                bail!("the archive couldn't be extracted");
            }
        } else {
            bail!("choose a voice package (.tar.bz2, .zip) or a folder that contains a .onnx model and tokens.txt");
        }

        let pkg = find_package(staging)?;
        let meta = read_metadata(&pkg.dir.join(&pkg.model));
        let folder = pkg.dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let model_type = meta.get("model_type").map(String::as_str).unwrap_or("vits");
        let kind = match model_type {
            "kokoro" => ModelKind::Kokoro,
            "vits" | "" => ModelKind::Vits,
            other => bail!("'{other}' models aren't supported yet. Use a Piper, Coqui, MMS (VITS) or Kokoro voice."),
        };
        if kind == ModelKind::Kokoro && !pkg.dir.join("voices.bin").exists() {
            bail!("this Kokoro package is missing voices.bin");
        }

        let (language, accent) = guess_language(&meta, &folder);
        let espeak = meta.get("voice").cloned();
        let base_name = pretty_name(&folder);
        let slug = unique_slug(&self.root, &slugify(&folder));
        let speakers = speaker_names(&meta);

        let mut voices = Vec::new();
        let many = speakers.len() > 1 && speakers.len() <= 64;
        for (sid, speaker) in if many { speakers.clone() } else { vec![(0, String::new())] } {
            voices.push(ImportedVoice {
                id: if many { format!("user-{slug}-{sid}") } else { format!("user-{slug}") },
                name: if many { format!("{base_name} · {}", pretty_speaker(&speaker)) } else { base_name.clone() },
                language: language.clone(),
                accent: accent.clone(),
                gender: guess_gender(&speaker),
                kind,
                model: rel(&pkg.dir, &pkg.model),
                tokens: rel(&pkg.dir, &pkg.tokens),
                data_dir: pkg.data_dir.as_ref().map(|p| rel(&pkg.dir, p)),
                lexicon: pkg.lexicon.as_ref().map(|p| rel(&pkg.dir, p)),
                espeak: espeak.clone(),
                speaker: sid,
                dir: PathBuf::new(),
            });
        }

        // The app's own phoneme data is used for every voice, so the package's copy is dead weight
        if let Some(d) = &pkg.data_dir {
            let _ = fs::remove_dir_all(d);
        }
        let dest = self.root.join(&slug);
        fs::rename(&pkg.dir, &dest).or_else(|_| copy_dir(&pkg.dir, &dest))?;
        fs::write(dest.join("voice.json"), serde_json::to_string_pretty(&voices)?)?;
        let _ = fs::remove_dir_all(staging);
        Ok(voices.into_iter().map(|mut v| {
            v.dir = dest.clone();
            v
        }).collect())
    }

    /// Deletes the package a voice came from (all its speakers)
    pub fn remove(&self, id: &str) -> Result<()> {
        let v = self.get(id).ok_or_else(|| anyhow!("voice not found"))?;
        if !v.dir.starts_with(&self.root) {
            bail!("refusing to delete outside the voices folder");
        }
        fs::remove_dir_all(&v.dir)?;
        Ok(())
    }

    /// Lets the user correct the language a package was detected as
    pub fn set_language(&self, id: &str, language: &str) -> Result<()> {
        let v = self.get(id).ok_or_else(|| anyhow!("voice not found"))?;
        let path = v.dir.join("voice.json");
        let mut list: Vec<ImportedVoice> = serde_json::from_str(&fs::read_to_string(&path)?)?;
        for item in &mut list {
            item.language = language.to_string();
        }
        fs::write(path, serde_json::to_string_pretty(&list)?)?;
        Ok(())
    }
}

trait NoWindow {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}

impl NoWindow for Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(0x0800_0000);
        }
        self
    }
}

struct Package {
    dir: PathBuf,
    model: PathBuf,
    tokens: PathBuf,
    data_dir: Option<PathBuf>,
    lexicon: Option<PathBuf>,
}

/// Finds the folder holding the model, its tokens and optional phoneme data
fn find_package(root: &Path) -> Result<Package> {
    let mut onnx = Vec::new();
    walk(root, &mut |p| {
        let in_espeak = p.components().any(|c| c.as_os_str() == "espeak-ng-data");
        if !in_espeak && p.extension().is_some_and(|e| e == "onnx") {
            onnx.push(p.to_path_buf());
        }
    });
    // Prefer full precision over int8 when both are shipped
    onnx.sort_by_key(|p| {
        let n = p.file_name().unwrap().to_string_lossy().to_lowercase();
        (n.contains("int8"), std::cmp::Reverse(fs::metadata(p).map(|m| m.len()).unwrap_or(0)))
    });
    let model = onnx.into_iter().next().ok_or_else(|| anyhow!("no .onnx speech model found in the package"))?;
    let dir = model.parent().unwrap().to_path_buf();
    let tokens = dir.join("tokens.txt");
    if !tokens.exists() {
        bail!("tokens.txt is missing next to {}", model.file_name().unwrap().to_string_lossy());
    }
    let data_dir = Some(dir.join("espeak-ng-data")).filter(|p| p.is_dir());
    let lexicon = fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("lexicon") && n.to_string_lossy().ends_with(".txt")))
        .min_by_key(|p| p.file_name().unwrap().len());
    Ok(Package { dir, model, tokens, data_dir, lexicon })
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, f);
            } else {
                f(&p);
            }
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &target)?;
        } else {
            fs::copy(e.path(), target)?;
        }
    }
    Ok(())
}

fn rel(base: &Path, p: &Path) -> String {
    p.strip_prefix(base).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// Reads the key/value metadata sherpa-onnx stores inside the model
fn read_metadata(model: &Path) -> HashMap<String, String> {
    tract_onnx::onnx()
        .proto_model_for_path(model)
        .map(|m| m.metadata_props.into_iter().map(|p| (p.key, p.value)).collect())
        .unwrap_or_default()
}

fn speaker_names(meta: &HashMap<String, String>) -> Vec<(i32, String)> {
    if let Some(names) = meta.get("speaker_names") {
        return names.split(',').enumerate().map(|(i, n)| (i as i32, n.trim().to_string())).collect();
    }
    let n: i32 = meta.get("n_speakers").and_then(|s| s.parse().ok()).unwrap_or(1);
    (0..n.max(1)).map(|i| (i, format!("Speaker {}", i + 1))).collect()
}

/// Works out the language from the model's metadata or the package name
/// (e.g. `vits-piper-de_DE-thorsten-high` → German, Germany)
fn guess_language(meta: &HashMap<String, String>, folder: &str) -> (String, String) {
    let lower = folder.to_lowercase();
    // Piper style locale: xx_YY
    let locale = lower.split(['-', '.']).find(|p| p.len() == 5 && p.as_bytes()[2] == b'_' && p[..2].chars().all(|c| c.is_ascii_lowercase()));
    if let Some(loc) = locale {
        let (lang, region) = (&loc[..2], loc[3..].to_uppercase());
        return (lang.to_string(), region_name(&region).unwrap_or(&region).to_string());
    }
    if let Some(v) = meta.get("voice").filter(|v| !v.is_empty()) {
        let lang = v.split(['-', '_']).next().unwrap_or(v).to_lowercase();
        if lang.len() == 2 {
            return (lang, String::new());
        }
    }
    if let Some(name) = meta.get("language") {
        if let Some(code) = code_for_language_name(name) {
            return (code.to_string(), String::new());
        }
    }
    // MMS packages: vits-mms-deu → three-letter code
    if let Some(code3) = lower.strip_prefix("vits-mms-") {
        if let Some(code) = whatlang::Lang::from_code(code3).map(|l| crate_lang_code(l)) {
            return (code.to_string(), String::new());
        }
    }
    ("und".into(), String::new())
}

fn crate_lang_code(l: whatlang::Lang) -> &'static str {
    match l.code() {
        "deu" => "de", "fra" => "fr", "spa" => "es", "eng" => "en", "ara" => "ar", "rus" => "ru", "por" => "pt", "ita" => "it",
        "tur" => "tr", "nld" => "nl", "pol" => "pl", "ukr" => "uk", "pes" => "fa", "hin" => "hi", "cmn" => "zh", "jpn" => "ja",
        "kor" => "ko", "heb" => "he", "ell" => "el", "swe" => "sv", "fin" => "fi", "hun" => "hu", "ces" => "cs", "ron" => "ro",
        "vie" => "vi", "ind" => "id", "urd" => "ur", "ben" => "bn", "tha" => "th",
        _ => "und",
    }
}

fn code_for_language_name(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    [
        ("english", "en"), ("arabic", "ar"), ("german", "de"), ("french", "fr"), ("spanish", "es"), ("italian", "it"),
        ("portuguese", "pt"), ("russian", "ru"), ("turkish", "tr"), ("persian", "fa"), ("farsi", "fa"), ("dutch", "nl"),
        ("polish", "pl"), ("ukrainian", "uk"), ("chinese", "zh"), ("japanese", "ja"), ("korean", "ko"), ("hindi", "hi"),
        ("urdu", "ur"), ("hebrew", "he"), ("greek", "el"), ("swedish", "sv"), ("norwegian", "nb"), ("danish", "da"),
        ("finnish", "fi"), ("czech", "cs"), ("hungarian", "hu"), ("romanian", "ro"), ("vietnamese", "vi"), ("indonesian", "id"),
        ("kurdish", "ku"), ("bengali", "bn"), ("swahili", "sw"), ("thai", "th"), ("catalan", "ca"), ("kazakh", "kk"),
        ("georgian", "ka"), ("serbian", "sr"), ("slovak", "sk"), ("slovenian", "sl"), ("icelandic", "is"), ("welsh", "cy"),
        ("nepali", "ne"), ("malayalam", "ml"), ("telugu", "te"), ("luxembourgish", "lb"), ("latvian", "lv"), ("estonian", "et"),
    ]
    .into_iter()
    .find(|(k, _)| n.contains(k))
    .map(|(_, v)| v)
}

/// English name of an ISO 639-1 code, for messages
pub fn language_name(code: &str) -> String {
    let names = [
        ("en", "English"), ("ar", "Arabic"), ("es", "Spanish"), ("fr", "French"), ("de", "German"), ("it", "Italian"),
        ("pt", "Portuguese"), ("ru", "Russian"), ("uk", "Ukrainian"), ("tr", "Turkish"), ("fa", "Persian"), ("ur", "Urdu"),
        ("hi", "Hindi"), ("zh", "Chinese"), ("ja", "Japanese"), ("ko", "Korean"), ("he", "Hebrew"), ("el", "Greek"),
        ("nl", "Dutch"), ("pl", "Polish"), ("sv", "Swedish"), ("bn", "Bengali"), ("th", "Thai"), ("ku", "Kurdish"),
    ];
    names.iter().find(|(c, _)| *c == code).map(|(_, n)| n.to_string()).unwrap_or_else(|| code.to_uppercase())
}

fn region_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "US" => "American", "GB" => "British", "DE" => "Germany", "AT" => "Austria", "CH" => "Switzerland", "FR" => "France",
        "BE" => "Belgium", "ES" => "Spain", "MX" => "Mexico", "AR" => "Argentina", "IT" => "Italy", "BR" => "Brazil",
        "PT" => "Portugal", "RU" => "Russia", "UA" => "Ukraine", "TR" => "Turkey", "IR" => "Iran", "NL" => "Netherlands",
        "PL" => "Poland", "CN" => "China", "JP" => "Japan", "KR" => "Korea", "IN" => "India", "JO" => "Jordan",
        "SA" => "Saudi Arabia", "EG" => "Egypt", "IQ" => "Iraq", "AE" => "UAE", "MA" => "Morocco", "SE" => "Sweden",
        "NO" => "Norway", "DK" => "Denmark", "FI" => "Finland", "CZ" => "Czechia", "HU" => "Hungary", "RO" => "Romania",
        "GR" => "Greece", "VN" => "Vietnam", "ID" => "Indonesia", "KZ" => "Kazakhstan", "GE" => "Georgia", "RS" => "Serbia",
        _ => return None,
    })
}

/// `vits-piper-de_DE-thorsten-high` → `Thorsten (high)`
fn pretty_name(folder: &str) -> String {
    let parts: Vec<&str> = folder.split('-').collect();
    let skip = ["vits", "piper", "coqui", "mms", "kokoro", "matcha", "multi", "lang"];
    let quality = ["x_low", "low", "medium", "high"];
    let mut name = Vec::new();
    let mut q = None;
    for p in parts {
        let pl = p.to_lowercase();
        if skip.contains(&pl.as_str()) || (pl.len() == 5 && pl.as_bytes().get(2) == Some(&b'_')) || pl.starts_with('v') && pl[1..].chars().all(|c| c.is_ascii_digit() || c == '_') {
            continue;
        }
        if quality.contains(&pl.as_str()) {
            q = Some(pl.replace('_', " "));
            continue;
        }
        name.push(capitalize(&p.replace('_', " ")));
    }
    let base = if name.is_empty() { folder.to_string() } else { name.join(" ") };
    match q {
        Some(q) => format!("{base} ({q})"),
        None => base,
    }
}

fn pretty_speaker(s: &str) -> String {
    // Kokoro style ids: af_heart → Heart
    let s = s.split_once('_').map(|(_, n)| n).unwrap_or(s);
    capitalize(&s.replace('_', " "))
}

fn guess_gender(speaker: &str) -> String {
    match speaker.as_bytes().get(1) {
        Some(b'f') if speaker.as_bytes().get(2) == Some(&b'_') => "female",
        Some(b'm') if speaker.as_bytes().get(2) == Some(&b'_') => "male",
        _ => "",
    }
    .to_string()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn slugify(s: &str) -> String {
    let slug: String = s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "voice".into() } else { slug }
}

fn unique_slug(root: &Path, base: &str) -> String {
    let mut slug = base.to_string();
    let mut n = 2;
    while root.join(&slug).exists() {
        slug = format!("{base}-{n}");
        n += 1;
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_languages_from_package_folders() {
        assert_eq!(pretty_name("vits-piper-de_DE-thorsten-high"), "Thorsten (high)");
        assert_eq!(pretty_name("vits-piper-ar_JO-kareem-medium"), "Kareem (medium)");
        let (l, a) = guess_language(&HashMap::new(), "vits-piper-ar_JO-kareem-medium");
        assert_eq!((l.as_str(), a.as_str()), ("ar", "Jordan"));
        let (l, _) = guess_language(&HashMap::new(), "vits-mms-deu");
        assert_eq!(l, "de");
        assert_eq!(slugify("vits-piper-en_US-amy-low"), "vits-piper-en-us-amy-low");
    }
}
