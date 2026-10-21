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
    /// Script runs synthesized separately so an English word inside Arabic
    /// (or the reverse) is spoken by the right voice instead of spelled out
    #[serde(skip)]
    pub segments: Vec<(Lang, String)>,
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

/// Splits a sentence into runs of Arabic and Latin script. Digits, spaces and
/// punctuation stay with the current run, and a minority run of one short
/// word is folded back in since switching voices for it sounds worse.
fn script_runs(s: &str, main: Lang) -> Vec<(Lang, String)> {
    let mut runs: Vec<(Lang, String)> = Vec::new();
    for word in s.split_inclusive(char::is_whitespace) {
        let lang = if word.chars().any(is_arabic) {
            Lang::Ar
        } else if word.chars().any(is_latin) {
            Lang::En
        } else {
            runs.last().map(|r| r.0).unwrap_or(main)
        };
        match runs.last_mut() {
            Some(last) if last.0 == lang => last.1.push_str(word),
            _ => runs.push((lang, word.to_string())),
        }
    }
    let mut merged: Vec<(Lang, String)> = Vec::new();
    for (lang, text) in runs {
        let short = lang != main && text.split_whitespace().count() == 1 && text.trim().chars().count() <= 3;
        let lang = if short { main } else { lang };
        match merged.last_mut() {
            Some(last) if last.0 == lang => last.1.push_str(&text),
            _ => merged.push((lang, text)),
        }
    }
    merged
        .into_iter()
        .map(|(l, t)| (l, t.trim().to_string()))
        .filter(|(_, t)| t.chars().any(|c| c.is_alphanumeric()))
        .collect()
}

pub fn split(text: &str) -> Vec<Sentence> {
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
            let segments = script_runs(&text, lang);
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
        assert_eq!(s[0].segments[1], (Lang::En, "Sayzo Reader".to_string()));
    }

    #[test]
    fn breaks_very_long_sentences() {
        let long = "word, ".repeat(120);
        assert!(split(&long).iter().all(|s| s.text.chars().count() <= MAX_CHUNK));
    }
}
