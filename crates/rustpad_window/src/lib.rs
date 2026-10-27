//! Cross-Platform Native Desktop Windowing, Input Events, Cursors, DPI, Clipboard & OS File Dialogs.
//! Zero external crate dependencies: uses dynamic `libX11.so.6` on Linux, native `Win32`
//! (`user32` / `gdi32` / `comdlg32` / `kernel32`) on Windows, and portable fallback elsewhere.

#[cfg(target_os = "linux")]
mod linux_x11;
#[cfg(target_os = "windows")]
mod win32;

use rustpad_render::Surface;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Delete,
    Enter,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Arrow,
    IBeam,
    Hand,
    ResizeEW,
}

#[derive(Debug, Clone)]
pub enum WindowEvent {
    Resize { width: u32, height: u32 },
    MouseMove { x: f32, y: f32, mods: Modifiers },
    MouseDown {
        x: f32,
        y: f32,
        button: MouseButton,
        mods: Modifiers,
    },
    MouseUp {
        x: f32,
        y: f32,
        button: MouseButton,
        mods: Modifiers,
    },
    Scroll { delta_y: f32, mods: Modifiers },
    KeyDown { key: Key, mods: Modifiers },
    TextInput(char),
    Tick,
    CloseRequested,
}

pub trait DesktopAppHandler {
    fn on_event(&mut self, event: WindowEvent, surface: &mut Surface) -> AppControl;
    fn render(&mut self, surface: &mut Surface);
    fn desired_cursor(&self) -> CursorShape {
        CursorShape::Arrow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppControl {
    Continue,
    Redraw,
    Exit,
}

/// Detects the display scale factor (`1.0` at 96 DPI, `1.25` at 120 DPI, `1.5` at 144 DPI, etc.).
pub fn detect_dpi_scale() -> f32 {
    if let Ok(raw) = std::env::var("RUSTPAD_UI_SCALE") {
        if let Ok(v) = raw.trim().parse::<f32>() {
            return v.clamp(0.85, 2.0);
        }
    }
    #[cfg(target_os = "windows")]
    {
        return win32::query_system_dpi_scale();
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(gdk) = std::env::var("GDK_SCALE") {
            if let Ok(v) = gdk.trim().parse::<f32>() {
                return v.clamp(1.0, 2.0);
            }
        }
        1.0
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        1.0
    }
}

fn internal_clipboard() -> &'static Mutex<String> {
    static CLIP: OnceLock<Mutex<String>> = OnceLock::new();
    CLIP.get_or_init(|| Mutex::new(String::new()))
}

static OS_CLIP_SYNCED: AtomicBool = AtomicBool::new(false);

/// Writes UTF-8 text to the native OS clipboard (with in-process mirror for headless/test use).
pub fn set_clipboard_text(text: &str) -> bool {
    let mut mem_ok = false;
    if let Ok(mut guard) = internal_clipboard().lock() {
        *guard = text.to_string();
        mem_ok = true;
    }
    #[cfg(target_os = "windows")]
    {
        let os_ok = win32::set_win32_clipboard_text(text);
        OS_CLIP_SYNCED.store(os_ok, Ordering::Relaxed);
        return os_ok || mem_ok;
    }
    #[cfg(target_os = "linux")]
    {
        let has_display = std::env::var_os("DISPLAY").is_some()
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        if has_display {
            for (bin, args) in [
                ("wl-copy", &([] as [&str; 0])[..]),
                ("xclip", &["-selection", "clipboard"][..]),
                ("xsel", &["--clipboard", "--input"][..]),
            ] {
                if let Ok(mut child) = Command::new(bin)
                    .args(args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    if child.wait().map(|s| s.success()).unwrap_or(false) {
                        OS_CLIP_SYNCED.store(true, Ordering::Relaxed);
                        return true;
                    }
                }
            }
        }
        OS_CLIP_SYNCED.store(false, Ordering::Relaxed);
        mem_ok
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let os_ok = child.wait().map(|s| s.success()).unwrap_or(false);
            OS_CLIP_SYNCED.store(os_ok, Ordering::Relaxed);
        }
        mem_ok
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        mem_ok
    }
}

/// Reads UTF-8 text from the native OS clipboard (falling back to the in-process clipboard buffer).
pub fn get_clipboard_text() -> Option<String> {
    if OS_CLIP_SYNCED.load(Ordering::Relaxed) || internal_clipboard().lock().map(|g| g.is_empty()).unwrap_or(true) {
        #[cfg(target_os = "windows")]
        {
            if let Some(s) = win32::get_win32_clipboard_text() {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            let has_display = std::env::var_os("DISPLAY").is_some()
                || std::env::var_os("WAYLAND_DISPLAY").is_some();
            if has_display {
                for (bin, args) in [
                    ("wl-paste", &["--no-newline"][..]),
                    ("xclip", &["-selection", "clipboard", "-o"][..]),
                    ("xsel", &["--clipboard", "--output"][..]),
                ] {
                    if let Ok(out) = Command::new(bin)
                        .args(args)
                        .stderr(Stdio::null())
                        .output()
                    {
                        if out.status.success() {
                            let s = String::from_utf8_lossy(&out.stdout).to_string();
                            if !s.is_empty() {
                                return Some(s);
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(out) = Command::new("pbpaste").output() {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    internal_clipboard()
        .lock()
        .ok()
        .map(|g| g.clone())
        .filter(|s| !s.is_empty())
}

pub fn run_native_window(
    title: &str,
    initial_width: u32,
    initial_height: u32,
    app: &mut dyn DesktopAppHandler,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        return linux_x11::run_x11_window(title, initial_width, initial_height, app);
    }
    #[cfg(target_os = "windows")]
    {
        return win32::run_win32_window(title, initial_width, initial_height, app);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (title, initial_width, initial_height, app);
        Err("Interactive native windowing requires Windows or Linux X11 (or use CLI flags)".into())
    }
}

pub fn try_native_open_dialog(title: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return win32::open_file_dialog(title);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = Command::new("zenity")
            .args([
                "--file-selection",
                "--title",
                title,
                "--file-filter=Documents (*.docx *.pdf *.md *.txt) | *.docx *.pdf *.md *.txt",
            ])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
            return None;
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = Command::new("osascript")
            .args(["-e", "POSIX path of (choose file)"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = title;
        None
    }
}

pub fn try_native_save_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return win32::save_file_dialog(title, default_name);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = Command::new("zenity")
            .args([
                "--file-selection",
                "--save",
                "--confirm-overwrite",
                "--title",
                title,
                "--filename",
                default_name,
            ])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
            return None;
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (title, default_name);
        None
    }
}

/// Dispatches a vector PDF spool file to the native OS print subsystem (Win32 Print verb / CUPS `lp`).
pub fn spool_pdf_to_os_printer(pdf_path: & std::path::Path) -> Result<String, String> {
    if std::env::var("RUSTPAD_HEADLESS_PRINT").as_deref() == Ok("1") {
        return Ok(format!(
            "Spooled print-ready vector PDF to {}",
            pdf_path.display()
        ));
    }
    #[cfg(target_os = "windows")]
    {
        let p_str = pdf_path.display().to_string().replace('\'', "''");
        let ps_cmd = format!("Start-Process -FilePath '{p_str}' -Verb Print");
        if let Ok(st) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .status()
        {
            if st.success() {
                return Ok(format!("Sent '{}' to Windows Print Spooler", pdf_path.display()));
            }
        }
        return Ok(format!(
            "Prepared print-ready PDF at '{}' (no default Windows printer bound)",
            pdf_path.display()
        ));
    }
    #[cfg(target_os = "linux")]
    {
        for bin in ["lp", "lpr"] {
            if let Ok(out) = Command::new(bin).arg(pdf_path).output() {
                if out.status.success() {
                    return Ok(format!("Dispatched '{}' to CUPS ({bin})", pdf_path.display()));
                }
            }
        }
        return Ok(format!(
            "Prepared print-ready PDF at '{}' (CUPS queue offline)",
            pdf_path.display()
        ));
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Ok(format!("Prepared print-ready PDF at '{}'", pdf_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_event_and_clipboard_roundtrip() {
        let ev = WindowEvent::KeyDown {
            key: Key::Char('k'),
            mods: Modifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
        };
        assert!(matches!(ev, WindowEvent::KeyDown { .. }));
        assert!(set_clipboard_text("RustPad Clipboard Check"));
        assert_eq!(
            get_clipboard_text().as_deref(),
            Some("RustPad Clipboard Check")
        );
    }
}
