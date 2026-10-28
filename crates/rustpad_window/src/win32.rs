//! Pure-Rust Win32 (`user32` / `gdi32` / `comdlg32` / `kernel32`) Native Desktop Window,
//! HiDPI Scale Detection, Native Cursors, UTF-16 Clipboard & Explorer File Dialogs.

#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use crate::{AppControl, CursorShape, DesktopAppHandler, Key, Modifiers, MouseButton, WindowEvent};
use rustpad_render::{DaylightTheme, Surface};
use std::ffi::c_void;
use std::path::PathBuf;

type HWND = *mut c_void;
type HINSTANCE = *mut c_void;
type HDC = *mut c_void;
type HICON = *mut c_void;
type HCURSOR = *mut c_void;
type HBRUSH = *mut c_void;
type HMENU = *mut c_void;
type HGLOBAL = *mut c_void;
type UINT = u32;
type DWORD = u32;
type WORD = u16;
type LONG = i32;
type BOOL = i32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;

#[repr(C)]
struct WNDCLASSW {
    style: UINT,
    lpfnWndProc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
}

#[repr(C)]
struct POINT {
    x: LONG,
    y: LONG,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint: [LONG; 4],
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [u8; 32],
}

#[repr(C)]
struct BITMAPINFOHEADER {
    biSize: DWORD,
    biWidth: LONG,
    biHeight: LONG,
    biPlanes: WORD,
    biBitCount: WORD,
    biCompression: DWORD,
    biSizeImage: DWORD,
    biXPelsPerMeter: LONG,
    biYPelsPerMeter: LONG,
    biClrUsed: DWORD,
    biClrImportant: DWORD,
}

#[repr(C)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

#[repr(C)]
struct OPENFILENAMEW {
    lStructSize: DWORD,
    hwndOwner: HWND,
    hInstance: HINSTANCE,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: DWORD,
    nFilterIndex: DWORD,
    lpstrFile: *mut u16,
    nMaxFile: DWORD,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: DWORD,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    Flags: DWORD,
    nFileOffset: WORD,
    nFileExtension: WORD,
    lpstrDefExt: *const u16,
    lCustData: LPARAM,
    lpfnHook: *const c_void,
    lpTemplateName: *const u16,
    pvReserved: *mut c_void,
    dwReserved: DWORD,
    FlagsEx: DWORD,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    fn GlobalAlloc(uFlags: UINT, dwBytes: usize) -> HGLOBAL;
    fn GlobalLock(hMem: HGLOBAL) -> *mut c_void;
    fn GlobalUnlock(hMem: HGLOBAL) -> BOOL;
    fn GlobalSize(hMem: HGLOBAL) -> usize;
    fn GlobalFree(hMem: HGLOBAL) -> HGLOBAL;
}

#[link(name = "user32")]
extern "system" {
    fn SetProcessDPIAware() -> BOOL;
    fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> WORD;
    fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn InvalidateRect(hWnd: HWND, lpRect: *const c_void, bErase: BOOL) -> BOOL;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: UINT, lpTimerFunc: *const c_void) -> usize;
    fn GetKeyState(nVirtKey: i32) -> i16;
    fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    fn SetCapture(hWnd: HWND) -> HWND;
    fn ReleaseCapture() -> BOOL;
    fn GetDC(hWnd: HWND) -> HDC;
    fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    fn OpenClipboard(hWndNewOwner: HWND) -> BOOL;
    fn CloseClipboard() -> BOOL;
    fn EmptyClipboard() -> BOOL;
    fn GetClipboardData(uFormat: UINT) -> HGLOBAL;
    fn SetClipboardData(uFormat: UINT, hMem: HGLOBAL) -> HGLOBAL;
    fn RegisterClipboardFormatW(lpszFormat: *const u16) -> UINT;
    fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
    fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> isize;
}

#[link(name = "shell32")]
extern "system" {
    fn DragAcceptFiles(hWnd: HWND, fAccept: BOOL);
    fn DragQueryFileW(hDrop: *mut c_void, iFile: UINT, lpszFile: *mut u16, cch: UINT) -> UINT;
    fn DragFinish(hDrop: *mut c_void);
}

#[link(name = "gdi32")]
extern "system" {
    fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    fn StretchDIBits(
        hdc: HDC,
        xDest: i32,
        yDest: i32,
        DestWidth: i32,
        DestHeight: i32,
        xSrc: i32,
        ySrc: i32,
        SrcWidth: i32,
        SrcHeight: i32,
        lpBits: *const c_void,
        lpbmi: *const BITMAPINFO,
        iUsage: UINT,
        rop: DWORD,
    ) -> i32;
}

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
    fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

struct Win32Context<'a> {
    app: &'a mut dyn DesktopAppHandler,
    surface: Surface,
    needs_render: bool,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn query_system_dpi_scale() -> f32 {
    unsafe {
        SetProcessDPIAware();
        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            return 1.0;
        }
        let dpi_x = GetDeviceCaps(hdc, 88 /* LOGPIXELSX */);
        ReleaseDC(std::ptr::null_mut(), hdc);
        if dpi_x <= 0 {
            1.0
        } else {
            (dpi_x as f32 / 96.0).clamp(1.0, 2.0)
        }
    }
}

pub fn set_win32_clipboard_text(text: &str) -> bool {
    const CF_UNICODETEXT: UINT = 13;
    const GMEM_MOVEABLE: UINT = 0x0002;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();
        let wide: Vec<u16> = text
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let byte_len = wide.len() * 2;
        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len);
        if hmem.is_null() {
            CloseClipboard();
            return false;
        }
        let dst = GlobalLock(hmem) as *mut u16;
        if dst.is_null() {
            GlobalFree(hmem);
            CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        GlobalUnlock(hmem);
        let ok = !SetClipboardData(CF_UNICODETEXT, hmem).is_null();
        if !ok {
            GlobalFree(hmem);
        }
        CloseClipboard();
        ok
    }
}

pub fn get_win32_clipboard_text() -> Option<String> {
    const CF_UNICODETEXT: UINT = 13;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let hmem = GetClipboardData(CF_UNICODETEXT);
        if hmem.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(hmem) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut len = 0usize;
        while len < 524_288 && *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let decoded = String::from_utf16_lossy(slice).replace("\r\n", "\n");
        GlobalUnlock(hmem);
        CloseClipboard();
        Some(decoded)
    }
}

pub fn get_win32_clipboard_image_bytes() -> Option<Vec<u8>> {
    const CF_DIB: UINT = 8;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let png_fmt1 = RegisterClipboardFormatW(to_wide("PNG").as_ptr());
        let png_fmt2 = RegisterClipboardFormatW(to_wide("image/png").as_ptr());
        for fmt in [png_fmt1, png_fmt2, CF_DIB] {
            if fmt == 0 {
                continue;
            }
            let hmem = GetClipboardData(fmt);
            if hmem.is_null() {
                continue;
            }
            let sz = GlobalSize(hmem);
            if !(16..=64 * 1024 * 1024).contains(&sz) {
                continue;
            }
            let ptr = GlobalLock(hmem) as *const u8;
            if ptr.is_null() {
                continue;
            }
            let data = std::slice::from_raw_parts(ptr, sz).to_vec();
            GlobalUnlock(hmem);
            CloseClipboard();
            return Some(data);
        }
        CloseClipboard();
        None
    }
}

fn current_mods() -> Modifiers {
    unsafe {
        Modifiers {
            shift: (GetKeyState(0x10) as u16 & 0x8000) != 0,
            ctrl: (GetKeyState(0x11) as u16 & 0x8000) != 0,
            alt: (GetKeyState(0x12) as u16 & 0x8000) != 0,
        }
    }
}

unsafe fn apply_win32_cursor(shape: CursorShape) {
    let id: usize = match shape {
        CursorShape::Arrow => 32512,    // IDC_ARROW
        CursorShape::IBeam => 32513,    // IDC_IBEAM
        CursorShape::ResizeEW => 32644, // IDC_SIZEWE
        CursorShape::Hand => 32649,     // IDC_HAND
    };
    let hcur = LoadCursorW(std::ptr::null_mut(), id as *const u16);
    if !hcur.is_null() {
        SetCursor(hcur);
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, -21 /* GWLP_USERDATA */) as *mut Win32Context<'static>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let ctx = &mut *ptr;

    let mut dispatch = |ev: WindowEvent| match ctx.app.on_event(ev, &mut ctx.surface) {
        AppControl::Continue => {}
        AppControl::Redraw => {
            // Coalesce high-frequency mouse/scroll bursts into WM_PAINT for zero-lag 60-120Hz VSync
            ctx.needs_render = true;
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        AppControl::Exit => PostQuitMessage(0),
    };

    match msg {
        0x0014 /* WM_ERASEBKGND */ => 1,
        0x0020 /* WM_SETCURSOR */ => {
            if (lparam & 0xFFFF) == 1 /* HTCLIENT */ {
                apply_win32_cursor(ctx.app.desired_cursor());
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        0x000F /* WM_PAINT */ => {
            if ctx.needs_render {
                ctx.app.render(&mut ctx.surface);
                ctx.needs_render = false;
            }
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                    biWidth: ctx.surface.width as LONG,
                    biHeight: -(ctx.surface.height as LONG),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0],
            };
            StretchDIBits(
                hdc,
                0,
                0,
                ctx.surface.width as i32,
                ctx.surface.height as i32,
                0,
                0,
                ctx.surface.width as i32,
                ctx.surface.height as i32,
                ctx.surface.pixels.as_ptr() as *const c_void,
                &bmi,
                0,
                0x00CC_0020, /* SRCCOPY */
            );
            EndPaint(hwnd, &ps);
            0
        }
        0x0005 /* WM_SIZE */ => {
            let w = ((lparam as usize) & 0xFFFF).max(320) as u32;
            let h = (((lparam as usize) >> 16) & 0xFFFF).max(240) as u32;
            ctx.surface.resize(w, h, DaylightTheme::DESK_BG);
            let _ = ctx
                .app
                .on_event(WindowEvent::Resize { width: w, height: h }, &mut ctx.surface);
            ctx.needs_render = true;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        0x0200 /* WM_MOUSEMOVE */ => {
            let x = (lparam as i16) as f32;
            let y = ((lparam >> 16) as i16) as f32;
            dispatch(WindowEvent::MouseMove {
                x,
                y,
                mods: current_mods(),
            });
            apply_win32_cursor(ctx.app.desired_cursor());
            0
        }
        0x0201 /* WM_LBUTTONDOWN */ => {
            SetCapture(hwnd);
            let x = (lparam as i16) as f32;
            let y = ((lparam >> 16) as i16) as f32;
            dispatch(WindowEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
                mods: current_mods(),
            });
            0
        }
        0x0202 /* WM_LBUTTONUP */ => {
            ReleaseCapture();
            let x = (lparam as i16) as f32;
            let y = ((lparam >> 16) as i16) as f32;
            dispatch(WindowEvent::MouseUp {
                x,
                y,
                button: MouseButton::Left,
                mods: current_mods(),
            });
            0
        }
        0x0204 /* WM_RBUTTONDOWN */ => {
            let x = (lparam as i16) as f32;
            let y = ((lparam >> 16) as i16) as f32;
            dispatch(WindowEvent::MouseDown {
                x,
                y,
                button: MouseButton::Right,
                mods: current_mods(),
            });
            0
        }
        0x020A /* WM_MOUSEWHEEL */ => {
            // Positive WHEEL_DELTA = scroll up (decrease scroll_y by 56px per notch)
            let notches = ((wparam >> 16) as i16) as f32 / 120.0;
            let delta = -notches * 56.0;
            dispatch(WindowEvent::Scroll {
                delta_y: delta,
                mods: current_mods(),
            });
            0
        }
        0x0100 /* WM_KEYDOWN */ => {
            let mods = current_mods();
            let key = match wparam {
                0x08 => Some(Key::Backspace),
                0x2E => Some(Key::Delete),
                0x0D => Some(Key::Enter),
                0x09 => Some(Key::Tab),
                0x1B => Some(Key::Escape),
                0x25 => Some(Key::Left),
                0x26 => Some(Key::Up),
                0x27 => Some(Key::Right),
                0x28 => Some(Key::Down),
                0x24 => Some(Key::Home),
                0x23 => Some(Key::End),
                0x21 => Some(Key::PageUp),
                0x22 => Some(Key::PageDown),
                vk if mods.ctrl && (0x41..=0x5A).contains(&vk) => {
                    Some(Key::Char(((vk as u8) + 32) as char))
                }
                0xDC if mods.ctrl => Some(Key::Char('\\')),
                0xBB | 0x6B if mods.ctrl => Some(Key::Char('=')),
                0xBD | 0x6D if mods.ctrl => Some(Key::Char('-')),
                0x30 | 0x60 if mods.ctrl => Some(Key::Char('0')),
                0xDB if mods.ctrl => Some(Key::Char('[')),
                0xDD if mods.ctrl => Some(Key::Char(']')),
                _ => None,
            };
            if let Some(k) = key {
                dispatch(WindowEvent::KeyDown { key: k, mods });
            }
            0
        }
        0x0102 /* WM_CHAR */ => {
            let mods = current_mods();
            if !mods.ctrl {
                if let Some(ch) = char::from_u32(wparam as u32) {
                    if !ch.is_control() {
                        dispatch(WindowEvent::TextInput(ch));
                    }
                }
            }
            0
        }
        0x0113 /* WM_TIMER */ => {
            dispatch(WindowEvent::Tick);
            0
        }
        0x0233 /* WM_DROPFILES */ => {
            let hdrop = wparam as *mut c_void;
            if !hdrop.is_null() {
                let mut file_buf = vec![0u16; 1024];
                let copied =
                    DragQueryFileW(hdrop, 0, file_buf.as_mut_ptr(), file_buf.len() as UINT) as usize;
                DragFinish(hdrop);
                if copied > 0 && copied < file_buf.len() {
                    let path_str = String::from_utf16_lossy(&file_buf[..copied]);
                    dispatch(WindowEvent::DroppedFile(PathBuf::from(path_str)));
                }
            }
            0
        }
        0x0010 /* WM_CLOSE */ | 0x0002 /* WM_DESTROY */ => {
            let _ = ctx.app.on_event(WindowEvent::CloseRequested, &mut ctx.surface);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run_win32_window(
    title: &str,
    initial_width: u32,
    initial_height: u32,
    app: &mut dyn DesktopAppHandler,
) -> Result<(), String> {
    unsafe {
        let dpi_scale = query_system_dpi_scale();
        let scaled_w = ((initial_width as f32) * dpi_scale.clamp(1.0, 1.5)).round() as u32;
        let scaled_h = ((initial_height as f32) * dpi_scale.clamp(1.0, 1.5)).round() as u32;

        let hinst = GetModuleHandleW(std::ptr::null());
        let cls_name = to_wide("RustPadNativeStudioWindow");
        let wnd_title = to_wide(title);

        // Load embedded PE RT_GROUP_ICON ID `1` from `app/build.rs` (fallback to IDI_APPLICATION = 32512)
        let mut app_icon = LoadIconW(hinst, 1usize as *const u16);
        if app_icon.is_null() {
            app_icon = LoadIconW(std::ptr::null_mut(), 32512usize as *const u16);
        }

        let wc = WNDCLASSW {
            style: 0x0003,
            lpfnWndProc: wnd_proc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: app_icon,
            hCursor: std::ptr::null_mut(), // Managed dynamically via WM_SETCURSOR
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let mut ctx = Win32Context {
            app,
            surface: Surface::new(scaled_w, scaled_h, DaylightTheme::DESK_BG),
            needs_render: false,
        };
        ctx.app.render(&mut ctx.surface);

        let hwnd = CreateWindowExW(
            0,
            cls_name.as_ptr(),
            wnd_title.as_ptr(),
            0x00CF_0000, /* WS_OVERLAPPEDWINDOW */
            64,
            40,
            scaled_w as i32 + 16,
            scaled_h as i32 + 39,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW failed".into());
        }

        if !app_icon.is_null() {
            // WM_SETICON (0x0080): ICON_SMALL (0) for titlebar + ICON_BIG (1) for Taskbar / Alt-Tab
            SendMessageW(hwnd, 0x0080, 0, app_icon as isize);
            SendMessageW(hwnd, 0x0080, 1, app_icon as isize);
        }

        SetWindowLongPtrW(hwnd, -21, (&mut ctx as *mut Win32Context<'_>) as isize);
        DragAcceptFiles(hwnd, 1);
        // 16ms (~60Hz) tick drives smooth kinetic spring scroll, cursor blink & background AI worker polling
        SetTimer(hwnd, 1, 16, std::ptr::null());
        ShowWindow(hwnd, 5 /* SW_SHOW */);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

pub fn open_image_file_dialog(title: &str) -> Option<PathBuf> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        let filter: Vec<u16> =
            "Images (*.png;*.jpg;*.jpeg;*.bmp)\0*.png;*.jpg;*.jpeg;*.bmp\0All Files (*.*)\0*.*\0\0"
                .encode_utf16()
                .collect();
        let wtitle = to_wide(title);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as DWORD;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as DWORD;
        ofn.lpstrTitle = wtitle.as_ptr();
        ofn.Flags = 0x0008_0000 | 0x0000_1000; /* OFN_EXPLORER | OFN_FILEMUSTEXIST */
        if GetOpenFileNameW(&mut ofn) != 0 {
            let end = file_buf.iter().position(|&c| c == 0).unwrap_or(0);
            Some(PathBuf::from(String::from_utf16_lossy(&file_buf[..end])))
        } else {
            None
        }
    }
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        let filter: Vec<u16> =
            "Documents (*.docx;*.pdf;*.md)\0*.docx;*.pdf;*.md;*.txt\0All Files (*.*)\0*.*\0\0"
                .encode_utf16()
                .collect();
        let wtitle = to_wide(title);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as DWORD;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as DWORD;
        ofn.lpstrTitle = wtitle.as_ptr();
        ofn.Flags = 0x0008_0000 | 0x0000_1000; /* OFN_EXPLORER | OFN_FILEMUSTEXIST */
        if GetOpenFileNameW(&mut ofn) != 0 {
            let end = file_buf.iter().position(|&c| c == 0).unwrap_or(0);
            Some(PathBuf::from(String::from_utf16_lossy(&file_buf[..end])))
        } else {
            None
        }
    }
}

pub fn save_file_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        for (i, u) in default_name.encode_utf16().take(512).enumerate() {
            file_buf[i] = u;
        }
        let filter: Vec<u16> =
            "Word Document (*.docx)\0*.docx\0PDF Document (*.pdf)\0*.pdf\0Markdown (*.md)\0*.md\0\0"
                .encode_utf16()
                .collect();
        let wtitle = to_wide(title);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as DWORD;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as DWORD;
        ofn.lpstrTitle = wtitle.as_ptr();
        ofn.Flags = 0x0008_0000 | 0x0000_0002; /* OFN_EXPLORER | OFN_OVERWRITEPROMPT */
        if GetSaveFileNameW(&mut ofn) != 0 {
            let end = file_buf.iter().position(|&c| c == 0).unwrap_or(0);
            Some(PathBuf::from(String::from_utf16_lossy(&file_buf[..end])))
        } else {
            None
        }
    }
}
