//! Watches the mouse system-wide for gestures that select text: a drag, or a
//! double/triple click. Used by "active" mode to read selections automatically.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP,
};

/// Minimum pointer travel, in pixels, for a press-and-release to count as a drag-select
const DRAG_THRESHOLD: i32 = 8;
/// Wait this long after the last gesture so a double click that becomes a
/// triple click (word → paragraph) triggers one read, not two
const SETTLE: Duration = Duration::from_millis(260);

static EVENTS: OnceLock<Sender<()>> = OnceLock::new();

struct Clicks {
    down: Option<POINT>,
    last_up: Option<(POINT, Instant)>,
    count: u32,
}

static CLICKS: Mutex<Clicks> = Mutex::new(Clicks { down: None, last_up: None, count: 0 });

fn near(a: POINT, b: POINT, d: i32) -> bool {
    (a.x - b.x).abs() <= d && (a.y - b.y).abs() <= d
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let mut c = CLICKS.lock().unwrap();
        match wparam.0 as u32 {
            WM_LBUTTONDOWN => c.down = Some(info.pt),
            WM_LBUTTONUP => {
                let dragged = c.down.map(|d| !near(d, info.pt, DRAG_THRESHOLD)).unwrap_or(false);
                let double_ms = GetDoubleClickTime() as u128;
                c.count = match c.last_up {
                    Some((p, t)) if near(p, info.pt, 4) && t.elapsed().as_millis() <= double_ms => c.count + 1,
                    _ => 1,
                };
                c.last_up = Some((info.pt, Instant::now()));
                c.down = None;
                if dragged || c.count >= 2 {
                    if let Some(tx) = EVENTS.get() {
                        let _ = tx.send(());
                    }
                }
            }
            _ => {}
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub fn start(on_selection: impl Fn() + Send + 'static) {
    let (tx, rx) = mpsc::channel::<()>();
    if EVENTS.set(tx).is_err() {
        return;
    }
    thread::spawn(move || {
        while rx.recv().is_ok() {
            loop {
                match rx.recv_timeout(SETTLE) {
                    Ok(()) => continue,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            on_selection();
        }
    });
    thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).ok();
        if SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module.map(|m| m.into()), 0).is_err() {
            eprintln!("failed to install mouse hook; auto-read on select is unavailable");
            return;
        }
        // Low-level hooks are delivered through this thread's message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
