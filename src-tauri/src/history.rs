use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const MAX_ITEMS: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: u64,
    pub text: String,
    pub source: String,
    pub at: u64,
}

pub struct History {
    items: Mutex<Vec<HistoryItem>>,
    path: PathBuf,
}

impl History {
    pub fn load(app: &AppHandle) -> Self {
        let path = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("sayzo"))
            .join("history.json");
        let items = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { items: Mutex::new(items), path }
    }

    fn persist(&self, items: &[HistoryItem]) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&self.path, serde_json::to_string(items).unwrap_or_default());
    }

    pub fn add(&self, text: &str, source: &str) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let mut items = self.items.lock().unwrap();
        // Re-reading the same text moves it to the top instead of duplicating it
        items.retain(|i| i.text != text);
        items.insert(0, HistoryItem { id: now, text: text.to_string(), source: source.to_string(), at: now });
        items.truncate(MAX_ITEMS);
        self.persist(&items);
    }

    pub fn list(&self) -> Vec<HistoryItem> {
        self.items.lock().unwrap().clone()
    }

    pub fn delete(&self, id: u64) {
        let mut items = self.items.lock().unwrap();
        items.retain(|i| i.id != id);
        self.persist(&items);
    }

    pub fn clear(&self) {
        let mut items = self.items.lock().unwrap();
        items.clear();
        self.persist(&items);
    }
}
