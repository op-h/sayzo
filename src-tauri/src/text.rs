use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Ar,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sentence {
    pub text: String,
    pub lang: Lang,
    /// Runs of one language (ISO 639-1 code, text), synthesized separately so a
    /// foreign word or phrase is spoken by the right voice instead of spelled out
    #[serde(skip)]
    pub segments: Vec<(String, String)>,
}

/// Longest piece handed to the synthesizer in one call; longer sentences are
/// broken at punctuation so playback starts quickly and prosody stays natural
const MAX_CHUNK: usize = 260;
/// The first chunk is kept shorter so the voice starts almost immediately
const FIRST_CHUNK: usize = 120;

pub fn is_arabic(c: char) -> bool {
    matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c as u32, 0x00C0..=0x024F)
}

/// Writing system of a word, used to cut sentences into single-language runs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Script {
    Arabic,
    Latin,
    Other(whatlang::Script),
    /// Digits and punctuation: they join whatever run they're in
    Neutral,
}

fn script_of(word: &str) -> Script {
    if word.chars().any(is_arabic) {
        Script::Arabic
    } else if word.chars().any(is_latin) {
        Script::Latin
    } else if !word.chars().any(char::is_alphabetic) {
        Script::Neutral
    } else {
        match whatlang::detect_script(word) {
            Some(whatlang::Script::Latin) | None => Script::Latin,
            Some(whatlang::Script::Arabic) => Script::Arabic,
            Some(other) => Script::Other(other),
        }
    }
}

/// ISO 639-1 code for a whatlang language
fn iso1(lang: whatlang::Lang) -> &'static str {
    use whatlang::Lang::*;
    match lang {
        Eng => "en", Ara => "ar", Pes => "fa", Urd => "ur", Spa => "es", Fra => "fr", Deu => "de", Ita => "it", Por => "pt",
        Rus => "ru", Ukr => "uk", Bel => "be", Bul => "bg", Srp => "sr", Mkd => "mk", Pol => "pl", Ces => "cs", Slk => "sk",
        Slv => "sl", Hrv => "hr", Nld => "nl", Swe => "sv", Dan => "da", Nob => "nb", Fin => "fi", Est => "et", Lav => "lv",
        Lit => "lt", Hun => "hu", Ron => "ro", Ell => "el", Tur => "tr", Aze => "az", Kat => "ka", Hye => "hy", Heb => "he",
        Yid => "yi", Hin => "hi", Mar => "mr", Nep => "ne", Ben => "bn", Pan => "pa", Guj => "gu", Ori => "or", Tam => "ta",
        Tel => "te", Kan => "kn", Mal => "ml", Sin => "si", Tha => "th", Khm => "km", Mya => "my", Vie => "vi", Ind => "id",
        Jav => "jv", Tgl => "tl", Cmn => "zh", Jpn => "ja", Kor => "ko", Amh => "am", Afr => "af", Zul => "zu", Cat => "ca",
        Epo => "eo", Lat => "la", Uzb => "uz", Tuk => "tk", Sna => "sn", Aka => "ak",
        _ => "und",
    }
}

/// Language most likely used when detection on a short run isn't reliable
fn script_default(script: Script) -> &'static str {
    use whatlang::Script as W;
    match script {
        Script::Arabic => "ar",
        Script::Latin | Script::Neutral => "en",
        Script::Other(s) => match s {
            W::Cyrillic => "ru",
            W::Greek => "el",
            W::Hebrew => "he",
            W::Devanagari => "hi",
            W::Bengali => "bn",
            W::Mandarin => "zh",
            W::Hiragana | W::Katakana => "ja",
            W::Hangul => "ko",
            W::Thai => "th",
            W::Georgian => "ka",
            W::Armenian => "hy",
            W::Tamil => "ta",
            W::Telugu => "te",
            W::Kannada => "kn",
            W::Malayalam => "ml",
            W::Gujarati => "gu",
            W::Gurmukhi => "pa",
            W::Ethiopic => "am",
            W::Khmer => "km",
            W::Myanmar => "my",
            W::Sinhala => "si",
            W::Oriya => "or",
            _ => "und",
        },
    }
}

/// Languages built into Sayzo; imported voices add more
#[cfg(test)]
const BUILT_IN_LANGUAGES: &[&str] = &["en", "ar", "es", "fr", "it", "pt", "hi"];

/// Picks the language of a single-script run. Detection only chooses among
/// languages the user has a voice for (plus Persian/Urdu, which must never get
/// Arabic vowels), so similar languages can't be confused with ones that
/// can't be spoken anyway. Short runs keep the script's usual language.
fn language_of(run: &str, script: Script, known: &[String]) -> String {
    let default = script_default(script);
    let words = run.split_whitespace().count();
    let long_enough = words >= 4 || matches!(script, Script::Other(_));
    let is_known = |code: &str| known.iter().any(|k| k == code) || matches!(code, "fa" | "ur");
    let Some(info) = whatlang::detect(run) else { return default.to_string() };
    let code = iso1(info.lang());
    // Persian and Urdu share the Arabic script but must not get Arabic vowels.
    // Only trust that when letters Arabic never uses are present.
    if script == Script::Arabic {
        let persian_letters = run.chars().any(|c| "پچژگکیےٹڈڑںۂھ".contains(c));
        return if persian_letters && matches!(code, "fa" | "ur") { code.to_string() } else { "ar".to_string() };
    }
    if !long_enough || code == "und" {
        return default.to_string();
    }
    if info.is_reliable() {
        // A clear language with no voice is read by the script's default voice
        return if is_known(code) { code.to_string() } else { default.to_string() };
    }
    // Close calls (Spanish vs Portuguese vs Italian) are settled among the
    // languages that can actually be spoken
    let allow: Vec<whatlang::Lang> = whatlang::Lang::all().iter().copied().filter(|l| is_known(iso1(*l)) && *l != whatlang::Lang::Ara).collect();
    if is_known(code) && words >= 5 {
        if let Some(best) = whatlang::Detector::with_allowlist(allow).detect(run) {
            return iso1(best.lang()).to_string();
        }
    }
    default.to_string()
}

pub fn detect(s: &str) -> Lang {
    let ar = s.chars().filter(|c| is_arabic(*c)).count();
    let lat = s.chars().filter(|c| is_latin(*c)).count();
    if ar > lat {
        Lang::Ar
    } else {
        Lang::En
    }
}

static URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+|www\.\S+").unwrap());
static HYPHEN_BREAK: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\w)-\n(\w)").unwrap());
static LIST_MARK: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*(?:[#>]+|[-*•▪●◦·]|\d+[.)])\s+").unwrap());
static SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t\u{00A0}\u{2000}-\u{200B}]+").unwrap());
static MD_EMPHASIS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\*\*|__|`+|~~)").unwrap());

/// Cleans text copied from web pages, PDFs and OCR so it reads like prose:
/// hard-wrapped lines are re-joined, URLs and markdown noise are dropped.
pub fn normalize(raw: &str) -> String {
    let t = raw.replace("\r\n", "\n").replace('\r', "\n").replace('\u{00AD}', "");
    let t = HYPHEN_BREAK.replace_all(&t, "$1$2");
    let t = URL.replace_all(&t, " link ");
    let t = MD_EMPHASIS.replace_all(&t, "");

    let mut out = String::new();
    let mut prev: Option<String> = None;
    for line in t.lines() {
        let line = SPACES.replace_all(LIST_MARK.replace(line, "").trim(), " ").to_string();
        if line.is_empty() {
            if prev.is_some() {
                out.push('\n');
            }
            prev = None;
            continue;
        }
        match &prev {
            // A long line that doesn't end a sentence was wrapped by the source, so join it
            Some(p) if p.chars().count() >= 40 && !ends_sentence(p) => out.push(' '),
            Some(_) => out.push('\n'),
            None => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        out.push_str(&line);
        prev = Some(line);
    }
    out.trim().to_string()
}

fn ends_sentence(s: &str) -> bool {
    s.trim_end()
        .chars()
        .last()
        .map(|c| ".!?:;؟؛…。\"'”»)".contains(c))
        .unwrap_or(false)
}

const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "no", "fig", "inc", "ltd", "approx", "dept",
];

fn is_abbreviation(buf: &str) -> bool {
    let word = buf
        .trim_end_matches('.')
        .rsplit(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("")
        .to_lowercase();
    // Single-letter initials like "J. Smith"
    (word.chars().count() == 1 && word.chars().all(|c| c.is_alphabetic())) || ABBREVIATIONS.contains(&word.as_str())
}

fn split_sentences(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        buf.push(c);
        if ".!?؟…。".contains(c) {
            // Swallow repeated terminators and closing quotes/brackets
            while i + 1 < chars.len() && ".!?؟…\"'”»)]".contains(chars[i + 1]) {
                i += 1;
                buf.push(chars[i]);
            }
            let at_boundary = i + 1 >= chars.len() || chars[i + 1].is_whitespace();
            if at_boundary && !(c == '.' && is_abbreviation(&buf)) {
                out.push(buf.trim().to_string());
                buf.clear();
            }
        }
        i += 1;
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    out
}

fn split_long(s: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s.trim().to_string();
    while rest.chars().count() > max {
        let head: String = rest.chars().take(max).collect();
        let cut = [", ", "، ", "; ", "؛ ", ": ", " - ", " — "]
            .iter()
            .filter_map(|p| head.rfind(p).map(|i| i + p.len()))
            .filter(|i| *i > max / 3)
            .max()
            .or_else(|| head.rfind(' ').map(|i| i + 1))
            .unwrap_or(head.len());
        out.push(rest[..cut].trim().to_string());
        rest = rest[cut..].trim().to_string();
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// Splits a sentence into runs of one script, then names each run's
/// language. Digits, spaces and punctuation stay with the current run, and a
/// minority run of one short word is folded back in since switching voices
/// for it sounds worse.
fn script_runs(s: &str, main: Lang, known: &[String]) -> Vec<(String, String)> {
    let main_script = if main == Lang::Ar { Script::Arabic } else { Script::Latin };
    let mut runs: Vec<(Script, String)> = Vec::new();
    for word in s.split_inclusive(char::is_whitespace) {
        let script = match script_of(word) {
            Script::Neutral => runs.last().map(|r| r.0).unwrap_or(main_script),
            other => other,
        };
        match runs.last_mut() {
            Some(last) if last.0 == script => last.1.push_str(word),
            _ => runs.push((script, word.to_string())),
        }
    }
    let mut merged: Vec<(Script, String)> = Vec::new();
    for (script, text) in runs {
        let short = script != main_script && text.split_whitespace().count() == 1 && text.trim().chars().count() <= 3;
        let script = if short { main_script } else { script };
        match merged.last_mut() {
            Some(last) if last.0 == script => last.1.push_str(&text),
            _ => merged.push((script, text)),
        }
    }
    merged
        .into_iter()
        .map(|(script, t)| (language_of(t.trim(), script, known), t.trim().to_string()))
        .filter(|(_, t)| t.chars().any(|c| c.is_alphanumeric()))
        .collect()
}

/// Splits text into sentences using only the built-in languages
#[cfg(test)]
pub fn split(text: &str) -> Vec<Sentence> {
    let known: Vec<String> = BUILT_IN_LANGUAGES.iter().map(|s| s.to_string()).collect();
    split_for(text, &known)
}

/// Splits text into sentences, detecting among the given languages
pub fn split_for(text: &str, known: &[String]) -> Vec<Sentence> {
    let mut pieces: Vec<String> = normalize(text)
        .lines()
        .flat_map(split_sentences)
        .flat_map(|s| split_long(&s, MAX_CHUNK))
        .filter(|s| s.chars().any(|c| c.is_alphanumeric()))
        .collect();
    if pieces.first().is_some_and(|p| p.chars().count() > FIRST_CHUNK) {
        let first = split_long(&pieces.remove(0), FIRST_CHUNK);
        let rest = first[1..].join(" ");
        pieces.insert(0, first[0].clone());
        if !rest.is_empty() {
            pieces.insert(1, rest);
        }
    }
    pieces
        .into_iter()
        .map(|text| {
            let lang = detect(&text);
            let segments = script_runs(&text, lang, known);
            Sentence { text, lang, segments }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_wrapped_lines_and_splits_sentences() {
        let s = split("This is a long line that was hard wrapped by a PDF viewer and\ncontinues here. Dr. Smith said hi! Next?");
        let texts: Vec<_> = s.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "This is a long line that was hard wrapped by a PDF viewer and continues here.",
                "Dr. Smith said hi!",
                "Next?"
            ]
        );
    }

    #[test]
    fn detects_arabic_and_mixed_runs() {
        let s = split("مرحبا بكم في تطبيق Sayzo Reader الجديد.");
        assert_eq!(s[0].lang, Lang::Ar);
        assert_eq!(s[0].segments.len(), 3);
        assert_eq!(s[0].segments[1], ("en".to_string(), "Sayzo Reader".to_string()));
    }

    #[test]
    fn detects_other_languages() {
        let langs = |t: &str| split(t).iter().flat_map(|s| s.segments.iter().map(|(l, _)| l.clone())).collect::<Vec<_>>();
        assert_eq!(langs("Hoy vamos a la playa con todos mis amigos de la escuela."), ["es"]);
        assert_eq!(langs("Aujourd'hui nous allons visiter le musée avec toute la famille."), ["fr"]);
        assert_eq!(langs("Привет, как у тебя дела сегодня?"), ["ru"]);
        assert_eq!(langs("Heute gehen wir mit der ganzen Familie in den Park."), ["en"]); // no German voice yet
        let with_de = |t: &str| split_for(t, &["en".into(), "de".into()]).iter().flat_map(|s| s.segments.iter().map(|(l, _)| l.clone())).collect::<Vec<_>>();
        assert_eq!(with_de("Heute gehen wir mit der ganzen Familie in den Park."), ["de"]);
        assert_eq!(langs("I love reading books every night."), ["en"]);
        assert_eq!(langs("من امروز به مدرسه رفتم و دوستانم را دیدم"), ["fa"]);
        assert_eq!(langs("ذهب الولد إلى المدرسة صباحا مع أصدقائه"), ["ar"]);
    }

    #[test]
    fn breaks_very_long_sentences() {
        let long = "word, ".repeat(120);
        assert!(split(&long).iter().all(|s| s.text.chars().count() <= MAX_CHUNK));
    }
}

