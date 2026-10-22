//! Arabic text preparation for speech.
//!
//! The Arabic voices were trained on fully vowelled (diacritized) text, but
//! almost all real Arabic is written without short vowels, so on plain text
//! they guess and mispronounce. Before synthesis every Arabic chunk is
//! normalized, its numbers are spelled out, and the missing vowels are
//! restored with the CATT diacritizer (Abjad AI, Apache-2.0), run in pure
//! Rust through tract so it doesn't clash with the speech engine's runtime.

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use tract_onnx::prelude::*;
use unicode_normalization::UnicodeNormalization;

/// Arabic letter -> CATT input id (Buckwalter order)
fn letter_id(c: char) -> Option<i64> {
    Some(match c {
        'ش' => 4,
        'ؤ' => 5,
        'ء' => 6,
        'ذ' => 7,
        'إ' => 8,
        'أ' => 9,
        'ا' => 10,
        'ض' => 11,
        'ع' => 12,
        'ح' => 13,
        'ص' => 14,
        'ط' => 15,
        'ى' => 16,
        'ظ' => 17,
        'ب' => 18,
        'د' => 19,
        'ف' => 20,
        'غ' => 21,
        'ه' => 22,
        'ج' => 23,
        'ك' => 24,
        'ل' => 25,
        'م' => 26,
        'ن' => 27,
        'ة' => 28,
        'ق' => 29,
        'ر' => 30,
        'س' => 31,
        'ت' => 32,
        'ث' => 33,
        'و' => 34,
        'خ' => 35,
        'ي' => 36,
        'ز' => 37,
        'آ' => 38,
        'ئ' => 39,
        _ => return None,
    })
}

const SPACE_ID: i64 = 3;

/// CATT output tag -> marks written after the letter
const TAGS: [&str; 18] = [
    "", "", "", "", "\u{651}\u{64f}", "\u{651}\u{64c}", "\u{651}\u{64e}", "\u{651}\u{64b}", "\u{651}\u{650}", "\u{651}\u{64d}",
    "\u{64b}", "\u{64d}", "\u{64c}", "\u{64e}", "\u{650}", "\u{652}", "\u{64f}", "\u{651}",
];

fn is_harakah(c: char) -> bool {
    matches!(c as u32, 0x064B..=0x0652 | 0x0670)
}

/// The model's positional table has 1024 slots
const MAX_LEN: usize = 1000;

type Plan = std::sync::Arc<TypedRunnableModel>;

pub struct Diacritizer {
    plan: Plan,
}

impl Diacritizer {
    pub fn load(model: &Path) -> Result<Self> {
        let plan = tract_onnx::onnx()
            .model_for_path(model)?
            .into_optimized()?
            .into_runnable()?;
        Ok(Self { plan })
    }

    /// Restores short vowels, shadda and tanween. Text that is already mostly
    /// vowelled is trusted as written.
    pub fn diacritize(&self, text: &str) -> Result<String> {
        let letters = text.chars().filter(|c| letter_id(*c).is_some()).count();
        let marks = text.chars().filter(|c| is_harakah(*c)).count();
        if letters == 0 || marks * 2 >= letters {
            return Ok(text.to_string());
        }
        let bare: String = text.chars().filter(|c| !is_harakah(*c)).collect();
        let mut out = String::with_capacity(bare.len() * 2);
        for piece in chunks(&bare, MAX_LEN) {
            out.push_str(&self.run(&piece)?);
        }
        Ok(out)
    }

    fn run(&self, text: &str) -> Result<String> {
        // Letters go to the model; everything else (spaces, punctuation,
        // Latin) is passed through and seen by the model as a word break
        let ids: Vec<i64> = text.chars().map(|c| letter_id(c).unwrap_or(SPACE_ID)).collect();
        if ids.iter().all(|&i| i == SPACE_ID) {
            return Ok(text.to_string());
        }
        let n = ids.len();
        let src = tract_ndarray::Array2::from_shape_vec((1, n), ids.clone())?;
        let mask = tract_ndarray::Array4::from_elem((1, 1, n, n), true);
        let result = self.plan.run(tvec!(src.into_tensor().into(), mask.into_tensor().into()))?;
        let logits = result[0].to_plain_array_view::<f32>()?;
        let tags: Vec<usize> = (0..n)
            .map(|i| {
                let row = logits.slice(tract_ndarray::s![0, i, ..]);
                row.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(k, _)| k).unwrap_or(3)
            })
            .collect();

        let mut out = String::with_capacity(text.len() * 2);
        for (c, tag) in text.chars().zip(tags) {
            out.push(c);
            if letter_id(c).is_some() {
                out.push_str(TAGS[tag]);
            }
        }
        Ok(out)
    }
}

/// Splits at spaces so no piece exceeds `max` characters
fn chunks(text: &str, max: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for word in text.split_inclusive(' ') {
        if out.last().unwrap().chars().count() + word.chars().count() > max {
            out.push(String::new());
        }
        out.last_mut().unwrap().push_str(word);
    }
    out
}

static NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(-?)(\d{1,3}(?:,\d{3})+|\d+)(?:[.٫](\d+))?(\s*[%٪])?").unwrap());
static SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

/// Normalizes Arabic text so the diacritizer and voice see standard letters,
/// and spells out numbers in Arabic words.
pub fn prepare(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    // NFKC folds presentation forms (ﻻ, ﷲ, isolated/final glyphs) into plain letters
    for c in text.nfkc() {
        match c {
            '\u{0640}' => {} // tatweel
            '٠'..='٩' => s.push(char::from(b'0' + (c as u32 - '٠' as u32) as u8)),
            '۰'..='۹' => s.push(char::from(b'0' + (c as u32 - '۰' as u32) as u8)),
            // Persian/Urdu letter variants the Arabic voice doesn't know
            'ک' => s.push('ك'),
            'ی' | 'ې' => s.push('ي'),
            'ە' | 'ہ' => s.push('ه'),
            'ٱ' => s.push('ا'),
            'گ' => s.push('ك'),
            'پ' => s.push('ب'),
            'چ' => s.push_str("تش"),
            'ڤ' => s.push('ف'),
            _ => s.push(c),
        }
    }
    let s = NUMBER.replace_all(&s, |m: &regex::Captures| {
        let mut words = String::new();
        if !m[1].is_empty() {
            words.push_str("سالب ");
        }
        let int: String = m[2].chars().filter(|c| c.is_ascii_digit()).collect();
        words.push_str(&match int.parse::<u64>() {
            Ok(n) if int.len() <= 15 => number_words(n),
            _ => int.chars().map(|d| number_words(d.to_digit(10).unwrap() as u64)).collect::<Vec<_>>().join(" "),
        });
        if let Some(frac) = m.get(3) {
            words.push_str(" فاصلة ");
            let f = frac.as_str().trim_start_matches('0');
            let zeros = frac.as_str().len() - f.len();
            for _ in 0..zeros {
                words.push_str("صفر ");
            }
            if !f.is_empty() {
                words.push_str(&number_words(f.parse::<u64>().unwrap_or(0)));
            }
        }
        if m.get(4).is_some() {
            words.push_str(" بالمئة");
        }
        format!(" {} ", words.trim())
    });
    SPACES.replace_all(s.trim(), " ").to_string()
}

const ONES: [&str; 20] = [
    "صفر", "واحد", "اثنان", "ثلاثة", "أربعة", "خمسة", "ستة", "سبعة", "ثمانية", "تسعة", "عشرة", "أحد عشر", "اثنا عشر", "ثلاثة عشر",
    "أربعة عشر", "خمسة عشر", "ستة عشر", "سبعة عشر", "ثمانية عشر", "تسعة عشر",
];
const TENS: [&str; 10] = ["", "", "عشرون", "ثلاثون", "أربعون", "خمسون", "ستون", "سبعون", "ثمانون", "تسعون"];
const HUNDREDS: [&str; 10] = ["", "مئة", "مئتان", "ثلاثمئة", "أربعمئة", "خمسمئة", "ستمئة", "سبعمئة", "ثمانمئة", "تسعمئة"];

fn below_thousand(n: u64) -> String {
    let (h, r) = (n / 100, n % 100);
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(HUNDREDS[h as usize].to_string());
    }
    if r > 0 {
        parts.push(if r < 20 {
            ONES[r as usize].to_string()
        } else if r % 10 == 0 {
            TENS[(r / 10) as usize].to_string()
        } else {
            format!("{} و{}", ONES[(r % 10) as usize], TENS[(r / 10) as usize])
        });
    }
    parts.join(" و")
}

/// Scale word for a count: (1, 2, 3–10, 11+)
fn scale(count: u64, words: [&str; 4]) -> String {
    match count {
        1 => words[0].to_string(),
        2 => words[1].to_string(),
        3..=10 => format!("{} {}", below_thousand(count), words[2]),
        _ => format!("{} {}", below_thousand(count), words[3]),
    }
}

/// Spells a whole number in Modern Standard Arabic
pub fn number_words(n: u64) -> String {
    if n == 0 {
        return ONES[0].to_string();
    }
    let groups = [
        (1_000_000_000_000, ["تريليون", "تريليونان", "تريليونات", "تريليون"]),
        (1_000_000_000, ["مليار", "ملياران", "مليارات", "مليار"]),
        (1_000_000, ["مليون", "مليونان", "ملايين", "مليون"]),
        (1_000, ["ألف", "ألفان", "آلاف", "ألف"]),
    ];
    let mut rest = n;
    let mut parts = Vec::new();
    for (size, words) in groups {
        let count = rest / size;
        if count > 0 {
            parts.push(scale(count, words));
            rest %= size;
        }
    }
    if rest > 0 {
        parts.push(below_thousand(rest));
    }
    parts.join(" و")
}

/// Loads the diacritizer if its model is present
pub fn load_diacritizer(models: &Path) -> Option<Diacritizer> {
    let path = models.join("catt").join("model.onnx");
    match Diacritizer::load(&path) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("Arabic diacritizer unavailable ({}): {e}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn spells_numbers() {
        assert_eq!(number_words(7), "سبعة");
        assert_eq!(number_words(21), "واحد وعشرون");
        assert_eq!(number_words(100), "مئة");
        assert_eq!(number_words(2026), "ألفان وستة وعشرون");
        assert_eq!(number_words(3000), "ثلاثة آلاف");
        assert_eq!(number_words(1_500_000), "مليون وخمسمئة ألف");
    }

    #[test]
    fn normalizes_digits_forms_and_percent() {
        assert_eq!(prepare("عام ٢٠٢٦"), "عام ألفان وستة وعشرون");
        assert_eq!(prepare("ﻻ"), "لا");
        assert_eq!(prepare("نسبة 12.5%"), "نسبة اثنا عشر فاصلة خمسة بالمئة");
        assert_eq!(prepare("كـــتاب"), "كتاب");
    }

    #[test]
    #[ignore = "needs the CATT model; run with --ignored"]
    fn diacritizes_plain_arabic() {
        let d = load_diacritizer(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models")).expect("model");
        let t = std::time::Instant::now();
        let out = d.diacritize("ذهب الولد إلى المدرسة صباحا").unwrap();
        println!("{out}  ({:?})", t.elapsed());
        assert_eq!(out, "ذَهَبَ الْوَلَدُ إِلَى الْمَدْرَسَةِ صَبَاحًا");
        // Already vowelled text is left alone
        assert_eq!(d.diacritize("كَتَبَ").unwrap(), "كَتَبَ");
    }
}
