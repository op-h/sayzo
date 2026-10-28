mod arabic;
mod draw;
mod history;
mod mouse_hook;
mod ocr;
mod reader;
mod selection;
mod settings;
mod shortcuts;
mod text;
mod tts;
mod voices;
mod window;

use history::{History, HistoryItem};
use reader::{Reader, Snapshot};
use serde::Serialize;
use settings::{Settings, SettingsState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    ready: bool,
    error: Option<String>,
}

struct EngineState(Mutex<EngineStatus>);
struct TrayItems {
    auto_read: CheckMenuItem<Wry>,
}

fn models_dir(app: &AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        // sherpa-onnx appends "/file" to paths, which Windows rejects after a
        // verbatim "\\?\" prefix, so use the plain form
        let plain = dir.to_string_lossy().trim_start_matches(r"\\?\").to_string();
        let p = PathBuf::from(plain).join("models");
        if p.join("kokoro").exists() {
            return p;
        }
    }
    // Development builds read the models straight from the source tree
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models")
}

pub fn sync_tray(app: &AppHandle, s: &Settings) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.auto_read.set_checked(s.auto_read);
    }
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    app.state::<SettingsState>().get()
}

#[tauri::command]
fn update_settings(app: AppHandle, settings: Settings) -> Vec<String> {
    let old = app.state::<SettingsState>().get();
    app.state::<SettingsState>().set(settings.clone());
    window::apply_settings(&app, &settings);
    app.state::<Reader>().set_volume(settings.volume);
    sync_tray(&app, &settings);
    let failed = if old.shortcuts != settings.shortcuts { shortcuts::register_all(&app) } else { Vec::new() };
    let _ = app.emit("settings://changed", settings);
    failed
}

#[tauri::command]
fn set_auto_read(app: AppHandle, enabled: bool) {
    shortcuts::set_auto_read(&app, enabled);
}

#[tauri::command]
fn get_voices(app: AppHandle) -> Vec<tts::Voice> {
    app.state::<Arc<tts::Engines>>().voices()
}

/// Imports a voice package the user picked, then checks it actually speaks
#[tauri::command]
async fn import_voice(app: AppHandle, path: String) -> Result<Vec<tts::Voice>, String> {
    let engines = app.state::<Arc<tts::Engines>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let added = engines.store().import(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
        if let Err(e) = engines.test_voice(&added[0].id) {
            engines.unload(&added[0].id);
            let _ = engines.store().remove(&added[0].id);
            return Err(format!("The voice couldn't be loaded: {e}"));
        }
        let ids: Vec<String> = added.iter().map(|v| v.id.clone()).collect();
        Ok(engines.voices().into_iter().filter(|v| ids.contains(&v.id)).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn remove_voice(app: AppHandle, id: String) -> Result<(), String> {
    let engines = app.state::<Arc<tts::Engines>>();
    engines.unload(&id);
    engines.store().remove(&id).map_err(|e| e.to_string())?;
    // Anything that pointed at the removed voice falls back to the defaults
    let s = app.state::<SettingsState>().update(|s| {
        let d = Settings::default();
        if !engines.voices().iter().any(|v| v.id == s.english_voice) {
            s.english_voice = d.english_voice.clone();
        }
        if !engines.voices().iter().any(|v| v.id == s.arabic_voice) {
            s.arabic_voice = d.arabic_voice.clone();
        }
        let all = engines.voices();
        s.language_voices.retain(|_, v| all.iter().any(|x| x.id == *v));
    });
    let _ = app.emit("settings://changed", s);
    Ok(())
}

#[tauri::command]
fn set_voice_language(app: AppHandle, id: String, language: String) -> Result<(), String> {
    app.state::<Arc<tts::Engines>>().store().set_language(&id, &language).map_err(|e| e.to_string())
}

/// Opens the free voice collection in the browser. The URL is fixed so the
/// webview can't launch arbitrary programs.
#[tauri::command]
fn open_voice_library() {
    let _ = std::process::Command::new("explorer").arg("https://github.com/k2-fsa/sherpa-onnx/releases/tag/tts-models").spawn();
}

#[tauri::command]
fn get_engine_status(app: AppHandle) -> EngineStatus {
    app.state::<EngineState>().0.lock().unwrap().clone()
}

#[tauri::command]
async fn speak_text(app: AppHandle, text: String, source: Option<String>, voice: Option<String>) -> Result<(), String> {
    app.state::<Reader>().speak(&text, source.as_deref().unwrap_or("typed"), voice).map_err(|e| e.to_string())
}

#[tauri::command]
fn reader_control(app: AppHandle, action: String) {
    let reader = app.state::<Reader>();
    match action.as_str() {
        "pause" => reader.pause(),
        "resume" => reader.resume(),
        "toggle" => return reader.toggle_pause(),
        "stop" => return reader.stop(),
        "next" => {
            let _ = reader.jump(1);
        }
        "previous" => {
            let _ = reader.jump(-1);
        }
        _ => {}
    }
}

#[tauri::command]
async fn reader_seek(app: AppHandle, index: usize) -> Result<(), String> {
    app.state::<Reader>().seek(index).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_reader_state(app: AppHandle) -> Snapshot {
    app.state::<Reader>().snapshot()
}

#[tauri::command]
async fn export_audio(app: AppHandle, text: String) -> Result<String, String> {
    let path = app.state::<Reader>().export(&text).map_err(|e| e.to_string())?;
    // Show the new file in Explorer
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    Ok(path.display().to_string())
}

#[tauri::command]
async fn ocr_region(app: AppHandle, window: WebviewWindow, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window.outer_position().map_err(|e| e.to_string())?;
    let (px, py) = (origin.x + (x * scale).round() as i32, origin.y + (y * scale).round() as i32);
    let (pw, ph) = ((width * scale).round() as i32, (height * scale).round() as i32);
    let _ = window.close();
    // Let the overlay disappear before the screen is captured
    std::thread::sleep(std::time::Duration::from_millis(180));
    match ocr::recognize_region(px, py, pw, ph) {
        Ok(text) => app.state::<Reader>().speak(&text, "screen", None).map_err(|e| e.to_string()),
        Err(e) => {
            shortcuts::toast(&app, &e.to_string());
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn get_history(app: AppHandle) -> Vec<HistoryItem> {
    app.state::<History>().list()
}

#[tauri::command]
fn delete_history(app: AppHandle, id: Option<u64>) {
    match id {
        Some(id) => app.state::<History>().delete(id),
        None => app.state::<History>().clear(),
    }
}

#[tauri::command]
fn validate_shortcut(accel: String) -> bool {
    accel.parse::<tauri_plugin_global_shortcut::Shortcut>().is_ok()
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let l = app.autolaunch();
    if enabled { l.enable() } else { l.disable() }.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub const AUTHOR_URL: &str = "https://github.com/op-h";

/// Opens the author's GitHub page in the default browser. The URL is fixed so the webview can't launch arbitrary programs.
#[tauri::command]
fn open_author_page() {
    let _ = std::process::Command::new("explorer").arg(AUTHOR_URL).spawn();
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let s = app.state::<SettingsState>().get();
    let show = MenuItem::with_id(app, "show", "Show / hide bar", true, None::<&str>)?;
    let auto_read = CheckMenuItem::with_id(app, "auto_read", "Auto-read on select", true, s.auto_read, None::<&str>)?;
    let screen = MenuItem::with_id(app, "ocr", "Read text from screen area", true, None::<&str>)?;
    let draw_item = MenuItem::with_id(app, "draw", "Draw on screen", true, None::<&str>)?;
    let dashboard = MenuItem::with_id(app, "dashboard", "Open dashboard", true, None::<&str>)?;
    let author = MenuItem::with_id(app, "author", "Designed by OPH (GitHub)", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Sayzo", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show, &auto_read, &screen, &draw_item, &dashboard, &PredefinedMenuItem::separator(app)?, &author, &quit],
    )?;
    app.manage(TrayItems { auto_read });

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Sayzo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::toggle_main(app),
            "auto_read" => {
                let on = !app.state::<SettingsState>().get().auto_read;
                shortcuts::set_auto_read(app, on);
            }
            "ocr" => {
                window::open_ocr(app.clone());
            }
            "draw" => {
                draw::start(app);
            }
            "dashboard" => {
                let _ = window::open_dashboard(app.clone());
            }
            "author" => open_author_page(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                window::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn start_auto_read(app: &AppHandle) {
    let app = app.clone();
    let last = Mutex::new(String::new());
    mouse_hook::start(move || {
        let s = app.state::<SettingsState>().get();
        if !s.auto_read {
            return;
        }
        let Some(text) = selection::selected_text(false, s.restore_clipboard) else { return };
        let reader = app.state::<Reader>();
        let mut last = last.lock().unwrap();
        // Clicking inside text that is already being read shouldn't restart it
        if reader.is_active() && *last == text {
            return;
        }
        *last = text.clone();
        if let Err(e) = reader.speak(&text, "selection", None) {
            shortcuts::toast(&app, &e.to_string());
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    window::pin_webview_runtime();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        shortcuts::on_shortcut(app, shortcut);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .manage(shortcuts::Registered::default())
        .manage(draw::DrawMode::default())
        .manage(EngineState(Mutex::new(EngineStatus::default())))
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            set_auto_read,
            get_voices,
            import_voice,
            remove_voice,
            set_voice_language,
            open_voice_library,
            get_engine_status,
            speak_text,
            reader_control,
            reader_seek,
            get_reader_state,
            export_audio,
            ocr_region,
            get_history,
            delete_history,
            validate_shortcut,
            get_autostart,
            set_autostart,
            get_app_version,
            open_author_page,
            exit_app,
            window::set_window_height,
            window::open_dashboard_command,
            window::toggle_dashboard_command,
            window::open_ocr_overlay_command,
            window::set_bar_width,
            draw::draw_open,
            draw::draw_close,
            draw::draw_toggle,
            draw::draw_undock,
            draw::draw_set_passthrough,
            draw::draw_get_state,
            draw::draw_toolbar_size,
            draw::draw_capture_screen,
            draw::draw_save_image,
            draw::draw_read_region,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(SettingsState::load(&handle));
            app.manage(History::load(&handle));
            let voice_dir = handle.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("sayzo")).join("voices");
            let engines = Arc::new(tts::Engines::new(models_dir(&handle), voices::VoiceStore::new(voice_dir)));
            app.manage(engines.clone());
            app.manage(Reader::new(handle.clone(), engines.clone()));

            let s = app.state::<SettingsState>().get();
            window::position_main(&handle);
            // A docked drawing toolbar follows the voice bar around
            if let Some(bar) = window::main_window(&handle) {
                let h = handle.clone();
                bar.on_window_event(move |e| {
                    if matches!(e, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                        draw::follow_bar(&h);
                    }
                });
            }
            window::apply_settings(&handle, &s);
            window::create_dashboard(&handle);
            setup_tray(&handle)?;
            let failed = shortcuts::register_all(&handle);
            if !failed.is_empty() {
                eprintln!("shortcuts not registered: {failed:?}");
            }
            start_auto_read(&handle);
            window::open_after_restart(&handle);

            // Load the voice models in the background so the first read starts instantly
            std::thread::spawn(move || {
                let s = handle.state::<SettingsState>().get();
                let status = match engines.preload(&s.voice_plan(None)) {
                    Ok(()) => EngineStatus { ready: true, error: None },
                    Err(e) => EngineStatus { ready: false, error: Some(e.to_string()) },
                };
                *handle.state::<EngineState>().0.lock().unwrap() = status.clone();
                let _ = handle.emit("engine://status", status);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Sayzo");
}
