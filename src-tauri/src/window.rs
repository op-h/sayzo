use crate::settings::Settings;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const BAR_WIDTH: f64 = 660.0;
/// Distance from the top of the screen to the floating bar
const TOP_OFFSET: i32 = 54;

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

pub fn position_main(app: &AppHandle) {
    let Some(window) = main_window(app) else { return };
    if let (Ok(Some(monitor)), Ok(size)) = (window.primary_monitor(), window.outer_size()) {
        let x = monitor.position().x + (monitor.size().width as i32 - size.width as i32) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, monitor.position().y + TOP_OFFSET));
    }
}

pub fn apply_settings(app: &AppHandle, s: &Settings) {
    for window in app.webview_windows().values() {
        let _ = window.set_content_protected(s.hide_from_capture);
    }
    if let Some(window) = main_window(app) {
        let _ = window.set_always_on_top(s.always_on_top);
    }
}

/// Current logical width of the bar: full, or the small pill when minimized
static BAR_CURRENT_WIDTH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(BAR_WIDTH as u32);

#[tauri::command]
pub fn set_window_height(window: WebviewWindow, height: u32) -> Result<(), String> {
    let width = BAR_CURRENT_WIDTH.load(std::sync::atomic::Ordering::SeqCst) as f64;
    window.set_size(Size::Logical(LogicalSize::new(width, height as f64))).map_err(|e| e.to_string())
}

/// Switches the bar between full width and the minimized pill, keeping it
/// centred on the same spot
#[tauri::command]
pub fn set_bar_width(window: WebviewWindow, width: u32) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let old = window.outer_size().map_err(|e| e.to_string())?;
    let new_w = (width as f64 * scale).round() as i32;
    let x = pos.x + (old.width as i32 - new_w) / 2;
    BAR_CURRENT_WIDTH.store(width, std::sync::atomic::Ordering::SeqCst);
    window.set_size(Size::Logical(LogicalSize::new(width as f64, old.height as f64 / scale))).map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new(x, pos.y)).map_err(|e| e.to_string())
}

pub fn toggle_main(app: &AppHandle) {
    let Some(window) = main_window(app) else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.unminimize();
    }
}

fn dashboard(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window("dashboard") {
        return Ok(w);
    }
    let window = WebviewWindowBuilder::new(app, "dashboard", WebviewUrl::App("index.html".into()))
        .title("Sayzo")
        .inner_size(900.0, 640.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .visible(false)
        .build()?;
    // Closing only hides it, so reopening is instant and keeps its state
    let w = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = w.hide();
        }
    });
    Ok(window)
}

pub fn create_dashboard(app: &AppHandle) {
    if let Err(e) = dashboard(app) {
        eprintln!("failed to create dashboard: {e}");
    }
}

pub fn open_dashboard(app: AppHandle) -> Result<(), String> {
    let w = dashboard(&app).map_err(|e| e.to_string())?;
    let _ = w.show();
    let _ = w.unminimize();
    w.set_focus().map_err(|e| e.to_string())
}

pub fn toggle_dashboard(app: AppHandle) -> Result<(), String> {
    let w = dashboard(&app).map_err(|e| e.to_string())?;
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        w.hide().map_err(|e| e.to_string())
    } else {
        open_dashboard(app)
    }
}

// Commands that may create a window must be async: building a window from a
// synchronous command deadlocks the whole app on Windows
#[tauri::command]
pub async fn open_dashboard_command(app: AppHandle) -> Result<(), String> {
    open_dashboard(app)
}

#[tauri::command]
pub async fn toggle_dashboard_command(app: AppHandle) -> Result<(), String> {
    toggle_dashboard(app)
}

/// Opens a transparent full-screen layer on the monitor under the cursor
/// where the user drags a rectangle around the text to read
#[tauri::command]
pub async fn open_ocr_overlay_command(app: AppHandle) -> Result<(), String> {
    open_ocr_overlay(app)
}

/// Covers the monitor under the cursor with the area picker
pub fn open_ocr_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("ocr-overlay") {
        let _ = w.set_focus();
        return Ok(());
    }
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("no monitor found")?;
    let window = WebviewWindowBuilder::new(&app, "ocr-overlay", WebviewUrl::App("index.html".into()))
        .title("Sayzo – select text area")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = window.set_position(*monitor.position());
    let _ = window.set_size(PhysicalSize::new(monitor.size().width, monitor.size().height));
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}
