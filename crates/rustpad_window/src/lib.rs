//! Cross-Platform Native Desktop Windowing, Input Events, Clipboard & OS File Dialogs.
//! Zero external crate dependencies: uses dynamic `libX11.so.6` on Linux, native `Win32`
//! (`user32` / `gdi32` / `comdlg32`) on Windows, and portable fallback elsewhere.

#[cfg(target_os = "linux")]
mod linux_x11;
#[cfg(target_os = "windows")]
mod win32;

use rustpad_render::Surface;
use std::path::PathBuf;
use std::process::Command;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppControl {
    Continue,
    Redraw,
    Exit,
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

/// Attempts to invoke a native OS Open File dialog (`Explorer` on Windows, `zenity`/`kdialog` on Linux,
/// `osascript` on macOS). Returns `None` if cancelled or if no external dialog tool is installed
/// (allowing the caller to fall back cleanly to RustPad's built-in Daylight File Modal).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_event_types() {
        let ev = WindowEvent::KeyDown {
            key: Key::Char('k'),
            mods: Modifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
        };
        assert!(matches!(ev, WindowEvent::KeyDown { .. }));
    }
}
