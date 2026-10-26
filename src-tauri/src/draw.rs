//! Screen drawing: a transparent full-screen canvas for ink plus a floating
//! toolbar, in two windows so the canvas can turn click-through ("mouse
//! mode") while the toolbar stays usable.

use crate::{ocr, reader::Reader, shortcuts};
use anyhow::{anyhow, Result};
use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const CANVAS: &str = "draw-canvas";
pub const TOOLBAR: &str = "draw-toolbar";
/// Gap between the toolbar and the bottom of the screen
const TOOLBAR_BOTTOM: f64 = 36.0;
pub const TOOLBAR_HEIGHT: f64 = 54.0;
pub const TOOLBAR_WIDTH: f64 = 820.0;

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

fn toolbar(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(TOOLBAR)
}

/// Focusing the full-screen canvas can lift it over the toolbar; re-insert
/// the toolbar at the top of the always-on-top band so it stays reachable
fn raise_toolbar(app: &AppHandle) {
    if let Some(t) = toolbar(app) {
        let _ = t.set_always_on_top(false);
        let _ = t.set_always_on_top(true);
    }
}

pub fn is_open(app: &AppHandle) -> bool {
    canvas(app).is_some()
}

fn emit_state(app: &AppHandle, passthrough: bool) {
    let _ = app.emit("draw://state", DrawState { open: is_open(app), passthrough });
}

/// Opens the canvas over the monitor under the cursor, with the toolbar at its bottom
pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(c) = canvas(app) {
        set_passthrough(app, false);
        let _ = c.set_focus();
        return Ok(());
    }
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| anyhow!("no monitor found"))?;
    let (pos, size, scale) = (*monitor.position(), *monitor.size(), monitor.scale_factor());

    let c = WebviewWindowBuilder::new(app, CANVAS, WebviewUrl::App("index.html".into()))
        .title("Sayzo – drawing")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build()?;
    let _ = c.set_position(pos);
    let _ = c.set_size(size);

    let t = WebviewWindowBuilder::new(app, TOOLBAR, WebviewUrl::App("index.html".into()))
        .title("Sayzo – drawing tools")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .inner_size(TOOLBAR_WIDTH, TOOLBAR_HEIGHT)
        .build()?;
    let width = (TOOLBAR_WIDTH * scale) as i32;
    let x = pos.x + (size.width as i32 - width) / 2;
    let y = pos.y + size.height as i32 - ((TOOLBAR_HEIGHT + TOOLBAR_BOTTOM) * scale) as i32;
    let _ = t.set_position(PhysicalPosition::new(x, y));

    let _ = c.show();
    let _ = t.show();
    let _ = c.set_focus();
    raise_toolbar(app);
    emit_state(app, false);
    Ok(())
}

pub fn close(app: &AppHandle) {
    for w in [toolbar(app), canvas(app)].into_iter().flatten() {
        let _ = w.close();
    }
    let _ = app.emit("draw://state", DrawState { open: false, passthrough: false });
}

/// Mouse mode: the canvas ignores the mouse so you can use the apps below
pub fn set_passthrough(app: &AppHandle, on: bool) {
    if let Some(c) = canvas(app) {
        let _ = c.set_ignore_cursor_events(on);
        if !on {
            let _ = c.set_focus();
            raise_toolbar(app);
        }
    }
    emit_state(app, on);
}

/// The drawing hotkey: start drawing, or flip between drawing and mouse mode
pub fn hotkey(app: &AppHandle) {
    if !is_open(app) {
        if let Err(e) = open(app) {
            shortcuts::toast(app, &e.to_string());
        }
        return;
    }
    let passthrough = app.state::<DrawMode>().0.load(std::sync::atomic::Ordering::SeqCst);
    app.state::<DrawMode>().0.store(!passthrough, std::sync::atomic::Ordering::SeqCst);
    set_passthrough(app, !passthrough);
}

/// Remembers whether the canvas is click-through, so the hotkey can flip it
#[derive(Default)]
pub struct DrawMode(pub std::sync::atomic::AtomicBool);

// Window-creating commands must be async: building a window from a
// synchronous command deadlocks the app on Windows
#[tauri::command]
pub async fn draw_open(app: AppHandle) -> Result<(), String> {
    app.state::<DrawMode>().0.store(false, std::sync::atomic::Ordering::SeqCst);
    open(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn draw_close(app: AppHandle) {
    app.state::<DrawMode>().0.store(false, std::sync::atomic::Ordering::SeqCst);
    close(&app);
}

#[tauri::command]
pub fn draw_set_passthrough(app: AppHandle, on: bool) {
    app.state::<DrawMode>().0.store(on, std::sync::atomic::Ordering::SeqCst);
    set_passthrough(&app, on);
}

#[tauri::command]
pub fn draw_get_state(app: AppHandle) -> DrawState {
    DrawState { open: is_open(&app), passthrough: app.state::<DrawMode>().0.load(std::sync::atomic::Ordering::SeqCst) }
}

/// Resizes the toolbar (collapsing it, or making room for a popover) while
/// keeping its bottom-centre anchored, so it grows upwards from where it sits
#[tauri::command]
pub fn draw_toolbar_size(window: WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let old = window.outer_size().map_err(|e| e.to_string())?;
    let (nw, nh) = ((width * scale).round() as i32, (height * scale).round() as i32);
    let x = pos.x + (old.width as i32 - nw) / 2;
    let y = pos.y + old.height as i32 - nh;
    window.set_size(Size::Logical(LogicalSize::new(width, height))).map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())
}

/// Hides both drawing windows for a moment, runs `f` on the bare screen, then shows them again
fn without_overlay<T>(app: &AppHandle, f: impl FnOnce(&WebviewWindow) -> Result<T>) -> Result<T> {
    let c = canvas(app).ok_or_else(|| anyhow!("drawing isn't open"))?;
    let t = toolbar(app);
    let _ = c.hide();
    if let Some(t) = &t {
        let _ = t.hide();
    }
    std::thread::sleep(std::time::Duration::from_millis(180));
    let result = f(&c);
    let _ = c.show();
    if let Some(t) = &t {
        let _ = t.show();
    }
    let _ = c.set_focus();
    raise_toolbar(app);
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
