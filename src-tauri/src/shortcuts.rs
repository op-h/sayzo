use crate::reader::Reader;
use crate::settings::SettingsState;
use crate::{selection, window};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// Maps registered shortcut ids back to the action names in settings
#[derive(Default)]
pub struct Registered(pub Mutex<HashMap<u32, String>>);

/// Registers every shortcut from settings and returns the actions whose key
/// combination is invalid or already taken by another app
pub fn register_all(app: &AppHandle) -> Vec<String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut map = HashMap::new();
    let mut failed = Vec::new();
    for (action, accel) in app.state::<SettingsState>().get().shortcuts {
        if accel.trim().is_empty() {
            continue;
        }
        match accel.parse::<Shortcut>() {
            Ok(sc) if gs.register(sc).is_ok() => {
                map.insert(sc.id(), action);
            }
            _ => failed.push(action),
        }
    }
    *app.state::<Registered>().0.lock().unwrap() = map;
    failed
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let action = app.state::<Registered>().0.lock().unwrap().get(&shortcut.id()).cloned();
    if let Some(action) = action {
        run_action(app, &action);
    }
}

pub fn toast(app: &AppHandle, message: &str) {
    let _ = app.emit("toast", message);
}

pub fn set_auto_read(app: &AppHandle, on: bool) {
    let s = app.state::<SettingsState>().update(|s| s.auto_read = on);
    crate::sync_tray(app, &s);
    let _ = app.emit("settings://changed", s);
    toast(app, if on { "Auto-read on: select any text to hear it" } else { "Auto-read off" });
}

pub fn run_action(app: &AppHandle, action: &str) {
    let reader = app.state::<Reader>().inner().clone();
    match action {
        "read_selection" => {
            let app = app.clone();
            // Grabbing the selection can wait on other apps, so keep it off the event loop
            std::thread::spawn(move || {
                let restore = app.state::<SettingsState>().get().restore_clipboard;
                match selection::selected_text(true, restore) {
                    Some(text) => {
                        if let Err(e) = reader.speak(&text, "selection", None) {
                            toast(&app, &e.to_string());
                        }
                    }
                    // Pressing the hotkey again with nothing selected stops reading
                    None if reader.is_active() => reader.stop(),
                    None => toast(&app, "Select some text first"),
                }
            });
        }
        "toggle_active" => {
            let on = !app.state::<SettingsState>().get().auto_read;
            set_auto_read(app, on);
        }
        "pause_resume" => reader.toggle_pause(),
        "stop" => reader.stop(),
        "next" => {
            let _ = reader.jump(1);
        }
        "previous" => {
            let _ = reader.jump(-1);
        }
        "ocr" => {
            window::open_ocr(app.clone());
        }
        "toggle_window" => window::toggle_main(app),
        "draw" => crate::draw::hotkey(app),
        "dashboard" => {
            let _ = window::toggle_dashboard(app.clone());
        }
        _ => {}
    }
}
