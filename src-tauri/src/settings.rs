use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "Active" mode: any text selected with the mouse is read automatically
    pub auto_read: bool,
    pub english_voice: String,
    pub arabic_voice: String,
    /// Voice for every other language, keyed by ISO 639-1 code
    pub language_voices: BTreeMap<String, String>,
    pub speed: f32,
    pub volume: f32,
    pub always_on_top: bool,
    /// Open the reading panel (full text with the current sentence highlighted) when a read starts
    pub show_panel: bool,
    pub hide_from_capture: bool,
    pub restore_clipboard: bool,
    /// The floating bar is shrunk to a small pill
    pub compact_bar: bool,
    pub shortcuts: BTreeMap<String, String>,
}

pub fn default_shortcuts() -> BTreeMap<String, String> {
    [
        ("read_selection", "ctrl+shift+space"),
        ("toggle_active", "ctrl+alt+space"),
        ("pause_resume", "ctrl+alt+p"),
        ("stop", "ctrl+alt+s"),
        ("next", "ctrl+alt+period"),
        ("previous", "ctrl+alt+comma"),
        ("ocr", "ctrl+alt+o"),
        ("toggle_window", "ctrl+backslash"),
        ("dashboard", "ctrl+shift+d"),
        ("draw", "ctrl+alt+d"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_read: false,
            english_voice: "en-3".into(),
            arabic_voice: "ar-miro".into(),
            language_voices: BTreeMap::new(),
            speed: 1.0,
            volume: 1.0,
            always_on_top: true,
            show_panel: false,
            hide_from_capture: false,
            restore_clipboard: true,
            compact_bar: false,
            shortcuts: default_shortcuts(),
        }
    }
}

pub struct SettingsState {
    inner: Mutex<Settings>,
    path: PathBuf,
}

impl Settings {
    /// Which voice reads which language, optionally forcing one voice for everything
    pub fn voice_plan(&self, force: Option<String>) -> crate::tts::VoicePlan {
        crate::tts::VoicePlan {
            english: self.english_voice.clone(),
            arabic: self.arabic_voice.clone(),
            others: self.language_voices.clone(),
            force,
        }
    }
}

impl SettingsState {
    pub fn load(app: &AppHandle) -> Self {
        let dir = app.path().app_config_dir().unwrap_or_else(|_| std::env::temp_dir().join("sayzo"));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        let mut settings: Settings = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // Shortcuts added in newer versions get their defaults
        for (k, v) in default_shortcuts() {
            settings.shortcuts.entry(k).or_insert(v);
        }
        Self { inner: Mutex::new(settings), path }
    }

    pub fn get(&self) -> Settings {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, settings: Settings) {
        let _ = fs::write(&self.path, serde_json::to_string_pretty(&settings).unwrap_or_default());
        *self.inner.lock().unwrap() = settings;
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = self.get();
        f(&mut s);
        self.set(s.clone());
        s
    }
}
