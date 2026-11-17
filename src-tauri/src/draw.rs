//! Screen drawing: a transparent full-screen canvas for ink. Its tools live
//! in the voice bar, a separate window, so the canvas can turn click-through
//! ("mouse mode") while the tools stay usable.

use crate::{ocr, reader::Reader, shortcuts};
use anyhow::{anyhow, Result};
use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Emitter, Manager, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const CANVAS: &str = "draw-canvas";
/// The voice bar was hidden when drawing started and was shown for its tools
static BAR_SHOWN_FOR_DRAWING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawState {
    pub open: bool,
    /// Clicks go to the apps underneath; the ink stays on screen
    pub passthrough: bool,
}

fn canvas(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(CANVAS)
}

/// While drawing is open, the main bar is owned by the canvas so Windows
/// keeps it above the drawing layer (clicking the canvas would otherwise
/// cover it). The owner must be cleared before the canvas closes, because
/// Windows destroys owned windows together with their owner.
#[cfg(windows)]
fn set_bar_owner(app: &AppHandle, owner: Option<&WebviewWindow>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_HWNDPARENT};
    let Some(bar) = crate::window::main_window(app) else { return };
    let (Ok(bar_hwnd), owner_hwnd) = (bar.hwnd(), owner.and_then(|o| o.hwnd().ok())) else { return };
    unsafe {
        SetWindowLongPtrW(HWND(bar_hwnd.0 as _), GWLP_HWNDPARENT, owner_hwnd.map(|h| h.0 as isize).unwrap_or(0));
    }
}

#[cfg(not(windows))]
fn set_bar_owner(_app: &AppHandle, _owner: Option<&WebviewWindow>) {}

pub fn is_open(app: &AppHandle) -> bool {
    canvas(app).is_some()
}

fn emit_state(app: &AppHandle, passthrough: bool) {
    let _ = app.emit("draw://state", DrawState { open: is_open(app), passthrough });
}

/// Opens drawing, recovering if its windows can't be created
pub fn start(app: &AppHandle) {
    if let Err(e) = open(app) {
        crate::window::recover(app, "draw", &e.to_string());
    }
}

/// Opens the canvas over the monitor under the cursor; the voice bar grows its drawing tools
pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(c) = canvas(app) {
        let _ = c.show();
        return Ok(());
    }
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| anyhow!("no monitor found"))?;
    let (pos, size) = (*monitor.position(), *monitor.size());

    let c = WebviewWindowBuilder::new(app, CANVAS, WebviewUrl::App("index.html".into()))
        .title("Sayzo – drawing")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        // Transparent from the first frame: no white flash while the page loads
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .content_protected(crate::window::hide_from_capture(app))
        .build()?;
    let _ = c.set_position(pos);
    let _ = c.set_size(size);

    // The canvas shows itself once its page has painted (no white box);
    // this is only a fallback in case that signal never arrives
    let c2 = c.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        if !c2.is_visible().unwrap_or(true) {
            let _ = c2.show();
        }
    });
    // The drawing tools are in the voice bar, so it has to be on screen;
    // a bar the user had hidden goes away again when drawing stops
    let bar_hidden = crate::window::main_window(app).is_some_and(|b| !b.is_visible().unwrap_or(true));
    BAR_SHOWN_FOR_DRAWING.store(bar_hidden, std::sync::atomic::Ordering::SeqCst);
    crate::window::show_main(app);
    set_bar_owner(app, Some(&c));
    // Starts in mouse mode: the screen stays live and usable until a tool is picked
    set_passthrough(app, true);
    Ok(())
}

pub fn close(app: &AppHandle) {
    app.state::<DrawMode>().0.store(true, std::sync::atomic::Ordering::SeqCst);
    // Release the main bar first, or closing the canvas would take it with it
    set_bar_owner(app, None);
    if let Some(c) = canvas(app) {
        let _ = c.close();
    }
    if BAR_SHOWN_FOR_DRAWING.swap(false, std::sync::atomic::Ordering::SeqCst) {
        if let Some(bar) = crate::window::main_window(app) {
            let _ = bar.hide();
        }
    }
    let _ = app.emit("draw://state", DrawState { open: false, passthrough: false });
}

/// Mouse mode: the canvas ignores the mouse so you can use the apps below
pub fn set_passthrough(app: &AppHandle, on: bool) {
    app.state::<DrawMode>().0.store(on, std::sync::atomic::Ordering::SeqCst);
    if let Some(c) = canvas(app) {
        let _ = c.set_ignore_cursor_events(on);
        if !on {
            let _ = c.set_focus();
        }
    }
    emit_state(app, on);
}

/// The drawing hotkey: start drawing, or flip between drawing and mouse mode
pub fn hotkey(app: &AppHandle) {
    if !is_open(app) {
        start(app);
        return;
    }
    let passthrough = app.state::<DrawMode>().0.load(std::sync::atomic::Ordering::SeqCst);
    set_passthrough(app, !passthrough);
}

/// Whether the canvas is click-through (mouse mode); the one source of truth
pub struct DrawMode(pub std::sync::atomic::AtomicBool);

impl Default for DrawMode {
    fn default() -> Self {
        Self(std::sync::atomic::AtomicBool::new(true))
    }
}

// Window-creating commands must be async: building a window from a
// synchronous command deadlocks the app on Windows
#[tauri::command]
pub async fn draw_open(app: AppHandle) {
    start(&app);
}

/// The bar's pen button: opens drawing, or closes it if it's already open
#[tauri::command]
pub async fn draw_toggle(app: AppHandle) {
    if is_open(&app) {
        close(&app);
    } else {
        start(&app);
    }
}

#[tauri::command]
pub async fn draw_close(app: AppHandle) {
    close(&app);
}

#[tauri::command]
pub fn draw_set_passthrough(app: AppHandle, on: bool) {
    set_passthrough(&app, on);
}

#[tauri::command]
pub fn draw_get_state(app: AppHandle) -> DrawState {
    DrawState { open: is_open(&app), passthrough: app.state::<DrawMode>().0.load(std::sync::atomic::Ordering::SeqCst) }
}

/// Hides the drawing and the voice bar for a moment, runs `f` on the bare
/// screen, then shows them again
fn without_overlay<T>(app: &AppHandle, f: impl FnOnce(&WebviewWindow) -> Result<T>) -> Result<T> {
    let c = canvas(app).ok_or_else(|| anyhow!("drawing isn't open"))?;
    let bar = crate::window::main_window(app).filter(|b| b.is_visible().unwrap_or(false));
    let _ = c.hide();
    if let Some(b) = &bar {
        let _ = b.hide();
    }
    std::thread::sleep(std::time::Duration::from_millis(180));
    let result = f(&c);
    let _ = c.show();
    if let Some(b) = &bar {
        let _ = b.show();
    }
    let _ = c.set_focus();
    result
}

/// Screen behind the drawing as a PNG, so the canvas can paint its ink on top
#[tauri::command]
pub async fn draw_capture_screen(app: AppHandle) -> Result<Response, String> {
    without_overlay(&app, |c| {
        let pos = c.outer_position()?;
        let size: PhysicalSize<u32> = c.outer_size()?;
        let bgra = ocr::capture(pos.x, pos.y, size.width as i32, size.height as i32)?;
        encode_png(&bgra, size.width, size.height)
    })
    .map(Response::new)
    .map_err(|e| e.to_string())
}

fn encode_png(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let mut rgba = bgra.to_vec();
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
        px[3] = 255;
    }
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&rgba)?;
    Ok(out)
}

/// Saves an annotated screenshot (PNG bytes in the request body) to
/// Pictures\Sayzo and copies it to the clipboard. Returns the file path.
#[tauri::command]
pub async fn draw_save_image(app: AppHandle, request: Request<'_>) -> Result<String, String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected PNG bytes".into());
    };
    let dir = app.path().picture_dir().map_err(|e| e.to_string())?.join("Sayzo");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = chrono_name();
    let path = dir.join(format!("Sayzo drawing {name}.png"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;

    // Clipboard wants raw RGBA
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes.as_slice()));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.color_type == png::ColorType::Rgba {
        let image = arboard::ImageData { width: info.width as usize, height: info.height as usize, bytes: buf[..info.buffer_size()].into() };
        if let Err(e) = arboard::Clipboard::new().and_then(|mut c| c.set_image(image)) {
            eprintln!("couldn't copy the screenshot: {e}");
        }
    }
    Ok(path.display().to_string())
}

/// Local date and time for file names, without pulling in a date crate
fn chrono_name() -> String {
    #[cfg(windows)]
    unsafe {
        let t = windows::Win32::System::SystemInformation::GetLocalTime();
        return format!("{:04}-{:02}-{:02} {:02}.{:02}.{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond);
    }
    #[allow(unreachable_code)]
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default()
}

/// The Read tool: reads aloud the text inside a box drawn on the canvas
/// (coordinates in the canvas' CSS pixels)
#[tauri::command]
pub async fn draw_read_region(app: AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let text = without_overlay(&app, |c| {
        let scale = c.scale_factor()?;
        let origin = c.outer_position()?;
        let (px, py) = (origin.x + (x * scale).round() as i32, origin.y + (y * scale).round() as i32);
        ocr::recognize_region(px, py, (width * scale).round() as i32, (height * scale).round() as i32)
    });
    match text {
        Ok(text) => app.state::<Reader>().speak(&text, "screen", None).map_err(|e| e.to_string()),
        Err(e) => {
            shortcuts::toast(&app, &e.to_string());
            Err(e.to_string())
        }
    }
}
