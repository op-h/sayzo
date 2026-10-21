//! Reads the text the user has selected in whatever app is in front.
//!
//! UI Automation is tried first because it doesn't touch the clipboard or send
//! keys. Apps that don't expose their selection that way get a simulated copy,
//! after which the user's clipboard is put back.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use uiautomation::patterns::UITextPattern;
use uiautomation::UIAutomation;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId};

/// Classic console windows: Ctrl+C there would interrupt the running program
const CONSOLE_CLASSES: &[&str] = &["ConsoleWindowClass", "PseudoConsoleWindow"];
/// Windows Terminal copies with Ctrl+Shift+C, which never sends an interrupt
const TERMINAL_CLASS: &str = "CASCADIA_HOSTING_WINDOW_CLASS";
/// File Explorer and the desktop: Ctrl+C would copy files, not text
const SHELL_CLASSES: &[&str] = &["CabinetWClass", "ExploreWClass", "Progman", "WorkerW", "Shell_TrayWnd"];

pub enum Uia {
    Selected(String),
    /// The control exposes text but nothing is selected
    Empty,
    /// The control doesn't expose text at all, so only copying can get it
    Unsupported,
}

struct Foreground {
    class: String,
    own: bool,
}

fn foreground() -> Option<Foreground> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut buf = [0u16; 256];
        let n = GetClassNameW(hwnd, &mut buf);
        Some(Foreground { class: String::from_utf16_lossy(&buf[..n.max(0) as usize]), own: pid == std::process::id() })
    }
}

fn uia_selection() -> Uia {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| -> uiautomation::Result<Uia> {
            let automation = UIAutomation::new()?;
            let walker = automation.get_control_view_walker()?;
            let mut element = automation.get_focused_element()?;
            // The focused element is often a child of the document that owns the text pattern
            for _ in 0..6 {
                if let Ok(pattern) = element.get_pattern::<UITextPattern>() {
                    let text = pattern
                        .get_selection()?
                        .iter()
                        .filter_map(|r| r.get_text(-1).ok())
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Ok(if text.trim().is_empty() { Uia::Empty } else { Uia::Selected(text) });
                }
                match walker.get_parent(&element) {
                    Ok(parent) => element = parent,
                    Err(_) => break,
                }
            }
            Ok(Uia::Unsupported)
        })();
        let _ = tx.send(result.unwrap_or(Uia::Unsupported));
    });
    // A hung app must not freeze reading, so give up on UIA after a moment
    rx.recv_timeout(Duration::from_millis(800)).unwrap_or(Uia::Unsupported)
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

/// The hotkey's modifiers may still be held; copying while Shift or Alt is
/// down would send a different shortcut (Ctrl+Shift+C opens DevTools in browsers)
fn wait_for_modifiers_released() {
    let mods = [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_CONTROL];
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline && mods.iter().any(|m| is_down(*m)) {
        thread::sleep(Duration::from_millis(15));
    }
    let still_held: Vec<INPUT> = [VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN]
        .into_iter()
        .filter(|m| is_down(*m))
        .map(|m| key(m, true))
        .collect();
    if !still_held.is_empty() {
        unsafe { SendInput(&still_held, std::mem::size_of::<INPUT>() as i32) };
    }
}

fn copy_via_clipboard(terminal: bool, restore: bool) -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let saved_text = clipboard.get_text().ok();
    let saved_image = if saved_text.is_none() { clipboard.get_image().ok() } else { None };

    wait_for_modifiers_released();
    let before = unsafe { GetClipboardSequenceNumber() };
    let mut inputs = vec![key(VK_CONTROL, false)];
    if terminal {
        inputs.push(key(VK_SHIFT, false));
    }
    inputs.push(key(VK_C, false));
    inputs.push(key(VK_C, true));
    if terminal {
        inputs.push(key(VK_SHIFT, true));
    }
    inputs.push(key(VK_CONTROL, true));
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };

    let deadline = Instant::now() + Duration::from_millis(500);
    let mut changed = false;
    while Instant::now() < deadline {
        if unsafe { GetClipboardSequenceNumber() } != before {
            changed = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !changed {
        return None;
    }
    // Some apps publish the clipboard in steps; give them a moment to finish
    thread::sleep(Duration::from_millis(40));
    let text = clipboard.get_text().ok();
    if restore {
        match (saved_text, saved_image) {
            (Some(t), _) => {
                let _ = clipboard.set_text(t);
            }
            (None, Some(img)) => {
                let _ = clipboard.set_image(img);
            }
            (None, None) => {
                let _ = clipboard.clear();
            }
        }
    }
    text
}

/// `eager` is true for the hotkey (the user explicitly asked, so copying is
/// worth trying even if UI Automation reported an empty selection) and false
/// for mouse-triggered auto-read, where only unsupported apps fall back.
pub fn selected_text(eager: bool, restore_clipboard: bool) -> Option<String> {
    let fg = foreground()?;
    if fg.own {
        return None;
    }
    let fallback_allowed = |fg: &Foreground| !CONSOLE_CLASSES.contains(&fg.class.as_str()) && !SHELL_CLASSES.contains(&fg.class.as_str());
    let text = match uia_selection() {
        Uia::Selected(t) => Some(t),
        Uia::Empty if !eager => None,
        _ if fallback_allowed(&fg) => copy_via_clipboard(fg.class == TERMINAL_CLASS, restore_clipboard),
        _ => None,
    };
    text.filter(|t| t.chars().any(|c| c.is_alphanumeric()))
}
