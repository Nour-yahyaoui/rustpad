//! Pure-Rust Dynamic X11 (`libX11.so.6`) Desktop Window Backend for Linux.
//! Uses `dlopen` so zero `-dev` header packages are needed to compile!

#![allow(non_camel_case_types, non_snake_case, clippy::too_many_arguments)]

use crate::{
    AppControl, CursorShape, DesktopAppHandler, Key, Modifiers, MouseButton, WindowEvent,
};
use rustpad_render::canvas::LOGO_64_RGBA;
use rustpad_render::{active_theme_mode, StudioTheme, Surface};
use std::ffi::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void, CString};
use std::time::{Duration, Instant};

type Display = c_void;
type Window = c_ulong;
type Atom = c_ulong;
type GC = *mut c_void;
type Visual = c_void;
type KeySym = c_ulong;

#[repr(C)]
struct XImage {
    width: c_int,
    height: c_int,
    xoffset: c_int,
    format: c_int,
    data: *mut c_char,
    byte_order: c_int,
    bitmap_unit: c_int,
    bitmap_bit_order: c_int,
    bitmap_pad: c_int,
    depth: c_int,
    bytes_per_line: c_int,
    bits_per_pixel: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    obdata: *mut c_char,
    f: [*mut c_void; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XKeyEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    root: Window,
    subwindow: Window,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    keycode: c_uint,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XButtonEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    root: Window,
    subwindow: Window,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    button: c_uint,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XMotionEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    root: Window,
    subwindow: Window,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    is_hint: c_char,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XConfigureEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    event: Window,
    window: Window,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    above: Window,
    override_redirect: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    message_type: Atom,
    format: c_int,
    data: [c_long; 5],
}

#[repr(C)]
union XEvent {
    type_: c_int,
    xkey: XKeyEvent,
    xbutton: XButtonEvent,
    xmotion: XMotionEvent,
    xconfigure: XConfigureEvent,
    xclient: XClientMessageEvent,
    pad: [c_long; 24],
}

#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

unsafe fn load_sym<T>(lib: *mut c_void, name: &[u8]) -> Result<T, String> {
    let ptr = dlsym(lib, name.as_ptr() as *const c_char);
    if ptr.is_null() {
        return Err(format!(
            "Missing X11 symbol: {}",
            String::from_utf8_lossy(name)
        ));
    }
    Ok(std::mem::transmute_copy(&ptr))
}

pub fn run_x11_window(
    title: &str,
    initial_width: u32,
    initial_height: u32,
    app: &mut dyn DesktopAppHandler,
) -> Result<(), String> {
    unsafe {
        let lib_name = CString::new("libX11.so.6").unwrap();
        let lib = dlopen(lib_name.as_ptr(), 1 /* RTLD_LAZY */);
        if lib.is_null() {
            return Err("Could not load libX11.so.6 (X11 runtime library)".into());
        }

        let XOpenDisplay: unsafe extern "C" fn(*const c_char) -> *mut Display =
            load_sym(lib, b"XOpenDisplay\0")?;
        let XDefaultScreen: unsafe extern "C" fn(*mut Display) -> c_int =
            load_sym(lib, b"XDefaultScreen\0")?;
        let XRootWindow: unsafe extern "C" fn(*mut Display, c_int) -> Window =
            load_sym(lib, b"XRootWindow\0")?;
        let XDefaultVisual: unsafe extern "C" fn(*mut Display, c_int) -> *mut Visual =
            load_sym(lib, b"XDefaultVisual\0")?;
        let XDefaultDepth: unsafe extern "C" fn(*mut Display, c_int) -> c_int =
            load_sym(lib, b"XDefaultDepth\0")?;
        let XDefaultGC: unsafe extern "C" fn(*mut Display, c_int) -> GC =
            load_sym(lib, b"XDefaultGC\0")?;
        let XCreateSimpleWindow: unsafe extern "C" fn(
            *mut Display,
            Window,
            c_int,
            c_int,
            c_uint,
            c_uint,
            c_uint,
            c_ulong,
            c_ulong,
        ) -> Window = load_sym(lib, b"XCreateSimpleWindow\0")?;
        let XStoreName: unsafe extern "C" fn(*mut Display, Window, *const c_char) -> c_int =
            load_sym(lib, b"XStoreName\0")?;
        let XSelectInput: unsafe extern "C" fn(*mut Display, Window, c_long) -> c_int =
            load_sym(lib, b"XSelectInput\0")?;
        let XMapWindow: unsafe extern "C" fn(*mut Display, Window) -> c_int =
            load_sym(lib, b"XMapWindow\0")?;
        let XInternAtom: unsafe extern "C" fn(*mut Display, *const c_char, c_int) -> Atom =
            load_sym(lib, b"XInternAtom\0")?;
        let XSetWMProtocols: unsafe extern "C" fn(*mut Display, Window, *mut Atom, c_int) -> c_int =
            load_sym(lib, b"XSetWMProtocols\0")?;
        let XChangeProperty: unsafe extern "C" fn(
            *mut Display,
            Window,
            Atom,
            Atom,
            c_int,
            c_int,
            *const c_uchar,
            c_int,
        ) -> c_int = load_sym(lib, b"XChangeProperty\0")?;
        let XPending: unsafe extern "C" fn(*mut Display) -> c_int = load_sym(lib, b"XPending\0")?;
        let XNextEvent: unsafe extern "C" fn(*mut Display, *mut XEvent) -> c_int =
            load_sym(lib, b"XNextEvent\0")?;
        let XLookupString: unsafe extern "C" fn(
            *mut XKeyEvent,
            *mut c_char,
            c_int,
            *mut KeySym,
            *mut c_void,
        ) -> c_int = load_sym(lib, b"XLookupString\0")?;
        let XCreateImage: unsafe extern "C" fn(
            *mut Display,
            *mut Visual,
            c_uint,
            c_int,
            c_int,
            *mut c_char,
            c_uint,
            c_uint,
            c_int,
            c_int,
        ) -> *mut XImage = load_sym(lib, b"XCreateImage\0")?;
        let XPutImage: unsafe extern "C" fn(
            *mut Display,
            Window,
            GC,
            *mut XImage,
            c_int,
            c_int,
            c_int,
            c_int,
            c_uint,
            c_uint,
        ) -> c_int = load_sym(lib, b"XPutImage\0")?;
        let XFlush: unsafe extern "C" fn(*mut Display) -> c_int = load_sym(lib, b"XFlush\0")?;
        let XDestroyWindow: unsafe extern "C" fn(*mut Display, Window) -> c_int =
            load_sym(lib, b"XDestroyWindow\0")?;
        let XCloseDisplay: unsafe extern "C" fn(*mut Display) -> c_int =
            load_sym(lib, b"XCloseDisplay\0")?;
        let x_create_font_cursor: Option<unsafe extern "C" fn(*mut Display, c_uint) -> c_ulong> =
            load_sym(lib, b"XCreateFontCursor\0").ok();
        let x_define_cursor: Option<unsafe extern "C" fn(*mut Display, Window, c_ulong) -> c_int> =
            load_sym(lib, b"XDefineCursor\0").ok();

        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return Err("No active X11 DISPLAY found (set DISPLAY or run on a desktop session)".into());
        }

        let screen = XDefaultScreen(dpy);
        let root = XRootWindow(dpy, screen);
        let visual = XDefaultVisual(dpy, screen);
        let depth = XDefaultDepth(dpy, screen);
        let gc = XDefaultGC(dpy, screen);

        let win = XCreateSimpleWindow(
            dpy,
            root,
            60,
            40,
            initial_width,
            initial_height,
            0,
            0x00E2_E8F0,
            0x00F4_F6F9,
        );

        let c_title = CString::new(title).unwrap_or_else(|_| CString::new("RustPad").unwrap());
        XStoreName(dpy, win, c_title.as_ptr());

        // Set _NET_WM_NAME (UTF-8) & _NET_WM_ICON (64x64 RustPad logo!)
        let net_wm_name = XInternAtom(dpy, b"_NET_WM_NAME\0".as_ptr() as *const c_char, 0);
        let utf8_atom = XInternAtom(dpy, b"UTF8_STRING\0".as_ptr() as *const c_char, 0);
        XChangeProperty(
            dpy,
            win,
            net_wm_name,
            utf8_atom,
            8,
            0, /* PropModeReplace */
            title.as_ptr(),
            title.len() as c_int,
        );

        let net_wm_icon = XInternAtom(dpy, b"_NET_WM_ICON\0".as_ptr() as *const c_char, 0);
        let cardinal_atom = XInternAtom(dpy, b"CARDINAL\0".as_ptr() as *const c_char, 0);
        let mut icon_data = Vec::with_capacity(2 + 64 * 64);
        icon_data.push(64 as c_ulong);
        icon_data.push(64 as c_ulong);
        for px in LOGO_64_RGBA.chunks_exact(4) {
            let argb = ((px[3] as c_ulong) << 24)
                | ((px[0] as c_ulong) << 16)
                | ((px[1] as c_ulong) << 8)
                | (px[2] as c_ulong);
            icon_data.push(argb);
        }
        XChangeProperty(
            dpy,
            win,
            net_wm_icon,
            cardinal_atom,
            32,
            0,
            icon_data.as_ptr() as *const c_uchar,
            icon_data.len() as c_int,
        );

        let mut wm_delete = XInternAtom(dpy, b"WM_DELETE_WINDOW\0".as_ptr() as *const c_char, 0);
        XSetWMProtocols(dpy, win, &mut wm_delete, 1);

        let gtk_theme_atom =
            XInternAtom(dpy, b"_GTK_THEME_VARIANT\0".as_ptr() as *const c_char, 0);
        let mut applied_dark_variant = active_theme_mode().is_dark_chrome();
        let initial_variant: &[u8] = if applied_dark_variant {
            b"dark"
        } else {
            b"light"
        };
        XChangeProperty(
            dpy,
            win,
            gtk_theme_atom,
            utf8_atom,
            8,
            0,
            initial_variant.as_ptr(),
            initial_variant.len() as c_int,
        );

        // KeyPressMask | ButtonPressMask | ButtonReleaseMask | PointerMotionMask | ExposureMask | StructureNotifyMask
        let event_mask: c_long =
            (1 << 0) | (1 << 2) | (1 << 3) | (1 << 6) | (1 << 15) | (1 << 17);
        XSelectInput(dpy, win, event_mask);
        XMapWindow(dpy, win);

        let mut surface = Surface::new(initial_width, initial_height, StudioTheme::desk_bg());
        app.render(&mut surface);
        blit_surface_x11(
            dpy,
            win,
            gc,
            visual,
            depth as c_uint,
            &mut surface,
            XCreateImage,
            XPutImage,
            XFlush,
        );

        let mut last_tick = Instant::now();
        let mut running = true;
        let mut active_cursor = CursorShape::Arrow;
        let cursors = if let Some(create_cur) = x_create_font_cursor {
            Some((
                create_cur(dpy, 68),  // XC_left_ptr
                create_cur(dpy, 152), // XC_xterm
                create_cur(dpy, 60),  // XC_hand2
                create_cur(dpy, 108), // XC_sb_h_double_arrow
            ))
        } else {
            None
        };

        while running {
            let mut needs_redraw = false;
            let mut pending_motion: Option<(f32, f32, Modifiers)> = None;

            while XPending(dpy) > 0 {
                let mut xev: XEvent = std::mem::zeroed();
                XNextEvent(dpy, &mut xev);
                if xev.type_ != 6 {
                    if let Some((mx, my, mmods)) = pending_motion.take() {
                        let ctrl = app.on_event(
                            WindowEvent::MouseMove {
                                x: mx,
                                y: my,
                                mods: mmods,
                            },
                            &mut surface,
                        );
                        handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                    }
                }
                match xev.type_ {
                    12 /* Expose */ => {
                        needs_redraw = true;
                    }
                    22 /* ConfigureNotify */ => {
                        let cfg = xev.xconfigure;
                        let nw = cfg.width.max(320) as u32;
                        let nh = cfg.height.max(240) as u32;
                        if nw != surface.width || nh != surface.height {
                            surface.resize(nw, nh, StudioTheme::desk_bg());
                            let _ = app.on_event(
                                WindowEvent::Resize {
                                    width: nw,
                                    height: nh,
                                },
                                &mut surface,
                            );
                            needs_redraw = true;
                        }
                    }
                    33 /* ClientMessage */ => {
                        if xev.xclient.data[0] as Atom == wm_delete {
                            let _ = app.on_event(WindowEvent::CloseRequested, &mut surface);
                            running = false;
                            break;
                        }
                    }
                    6 /* MotionNotify */ => {
                        // Coalesce high-DPI pointer motion bursts into a single per-frame coordinate
                        let m = xev.xmotion;
                        pending_motion = Some((m.x as f32, m.y as f32, decode_x11_mods(m.state)));
                    }
                    4 /* ButtonPress */ => {
                        let b = xev.xbutton;
                        let mods = decode_x11_mods(b.state);
                        let ev = match b.button {
                            1 => Some(WindowEvent::MouseDown {
                                x: b.x as f32,
                                y: b.y as f32,
                                button: MouseButton::Left,
                                mods,
                            }),
                            3 => Some(WindowEvent::MouseDown {
                                x: b.x as f32,
                                y: b.y as f32,
                                button: MouseButton::Right,
                                mods,
                            }),
                            4 => Some(WindowEvent::Scroll {
                                delta_y: -56.0,
                                mods,
                            }),
                            5 => Some(WindowEvent::Scroll {
                                delta_y: 56.0,
                                mods,
                            }),
                            _ => None,
                        };
                        if let Some(w_ev) = ev {
                            let ctrl = app.on_event(w_ev, &mut surface);
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        }
                    }
                    5 /* ButtonRelease */ => {
                        let b = xev.xbutton;
                        let btn = match b.button {
                            1 => Some(MouseButton::Left),
                            3 => Some(MouseButton::Right),
                            _ => None,
                        };
                        if let Some(button) = btn {
                            let ctrl = app.on_event(
                                WindowEvent::MouseUp {
                                    x: b.x as f32,
                                    y: b.y as f32,
                                    button,
                                    mods: decode_x11_mods(b.state),
                                },
                                &mut surface,
                            );
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        }
                    }
                    2 /* KeyPress */ => {
                        let mut kev = xev.xkey;
                        let mods = decode_x11_mods(kev.state);
                        let mut buf = [0i8; 32];
                        let mut keysym: KeySym = 0;
                        let n = XLookupString(
                            &mut kev,
                            buf.as_mut_ptr(),
                            buf.len() as c_int,
                            &mut keysym,
                            std::ptr::null_mut(),
                        );

                        if let Some(mapped) = map_x11_keysym(keysym) {
                            let ctrl =
                                app.on_event(WindowEvent::KeyDown { key: mapped, mods }, &mut surface);
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        } else if mods.ctrl && (0x61..=0x7A).contains(&keysym) {
                            let ch = (keysym as u8) as char;
                            let ctrl = app.on_event(
                                WindowEvent::KeyDown {
                                    key: Key::Char(ch),
                                    mods,
                                },
                                &mut surface,
                            );
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        } else if mods.ctrl && (0x41..=0x5A).contains(&keysym) {
                            let ch = ((keysym as u8) + 32) as char;
                            let ctrl = app.on_event(
                                WindowEvent::KeyDown {
                                    key: Key::Char(ch),
                                    mods,
                                },
                                &mut surface,
                            );
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        } else if mods.ctrl && matches!(keysym, 0x5C | 0x3D | 0x2B | 0x2D | 0x30 | 0x5B | 0x5D) {
                            let ch = match keysym {
                                0x5C => '\\',
                                0x3D | 0x2B => '=',
                                0x2D => '-',
                                0x30 => '0',
                                0x5B => '[',
                                _ => ']',
                            };
                            let ctrl = app.on_event(
                                WindowEvent::KeyDown {
                                    key: Key::Char(ch),
                                    mods,
                                },
                                &mut surface,
                            );
                            handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                        } else if n > 0 && !mods.ctrl {
                            let bytes: Vec<u8> =
                                buf[..n as usize].iter().map(|&c| c as u8).collect();
                            if let Ok(s) = std::str::from_utf8(&bytes) {
                                for ch in s.chars() {
                                    if !ch.is_control() {
                                        let ctrl =
                                            app.on_event(WindowEvent::TextInput(ch), &mut surface);
                                        handle_ctrl(ctrl, &mut running, &mut needs_redraw);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if let Some((mx, my, mmods)) = pending_motion.take() {
                let ctrl = app.on_event(
                    WindowEvent::MouseMove {
                        x: mx,
                        y: my,
                        mods: mmods,
                    },
                    &mut surface,
                );
                handle_ctrl(ctrl, &mut running, &mut needs_redraw);
            }

            let next_cur = app.desired_cursor();
            if next_cur != active_cursor {
                active_cursor = next_cur;
                if let (Some(def_cur), Some((c_arr, c_ibeam, c_hand, c_ew))) =
                    (x_define_cursor, cursors)
                {
                    let cid = match next_cur {
                        CursorShape::Arrow => c_arr,
                        CursorShape::IBeam => c_ibeam,
                        CursorShape::Hand => c_hand,
                        CursorShape::ResizeEW => c_ew,
                    };
                    def_cur(dpy, win, cid);
                    XFlush(dpy);
                }
            }

            if last_tick.elapsed() >= Duration::from_millis(16) {
                last_tick = Instant::now();
                let ctrl = app.on_event(WindowEvent::Tick, &mut surface);
                handle_ctrl(ctrl, &mut running, &mut needs_redraw);
            }

            if needs_redraw && running {
                let want_dark = active_theme_mode().is_dark_chrome();
                if want_dark != applied_dark_variant {
                    applied_dark_variant = want_dark;
                    let variant: &[u8] = if want_dark { b"dark" } else { b"light" };
                    XChangeProperty(
                        dpy,
                        win,
                        gtk_theme_atom,
                        utf8_atom,
                        8,
                        0,
                        variant.as_ptr(),
                        variant.len() as c_int,
                    );
                }
                app.render(&mut surface);
                blit_surface_x11(
                    dpy,
                    win,
                    gc,
                    visual,
                    depth as c_uint,
                    &mut surface,
                    XCreateImage,
                    XPutImage,
                    XFlush,
                );
            } else {
                std::thread::sleep(Duration::from_millis(6)); // ~120-165 Hz low-latency idle poll
            }
        }

        XDestroyWindow(dpy, win);
        XCloseDisplay(dpy);
    }
    Ok(())
}

fn handle_ctrl(ctrl: AppControl, running: &mut bool, needs_redraw: &mut bool) {
    match ctrl {
        AppControl::Continue => {}
        AppControl::Redraw => *needs_redraw = true,
        AppControl::Exit => *running = false,
    }
}

fn decode_x11_mods(state: c_uint) -> Modifiers {
    Modifiers {
        shift: (state & 0x01) != 0,
        ctrl: (state & 0x04) != 0,
        alt: (state & 0x08) != 0,
    }
}

fn map_x11_keysym(ks: KeySym) -> Option<Key> {
    match ks {
        0xFF08 => Some(Key::Backspace),
        0xFFFF => Some(Key::Delete),
        0xFF0D | 0xFF8D => Some(Key::Enter),
        0xFF09 | 0xFE20 => Some(Key::Tab),
        0xFF1B => Some(Key::Escape),
        0xFF51 => Some(Key::Left),
        0xFF52 => Some(Key::Up),
        0xFF53 => Some(Key::Right),
        0xFF54 => Some(Key::Down),
        0xFF50 => Some(Key::Home),
        0xFF57 => Some(Key::End),
        0xFF55 => Some(Key::PageUp),
        0xFF56 => Some(Key::PageDown),
        _ => None,
    }
}

unsafe fn blit_surface_x11(
    dpy: *mut Display,
    win: Window,
    gc: GC,
    visual: *mut Visual,
    depth: c_uint,
    surface: &mut Surface,
    XCreateImage: unsafe extern "C" fn(
        *mut Display,
        *mut Visual,
        c_uint,
        c_int,
        c_int,
        *mut c_char,
        c_uint,
        c_uint,
        c_int,
        c_int,
    ) -> *mut XImage,
    XPutImage: unsafe extern "C" fn(
        *mut Display,
        Window,
        GC,
        *mut XImage,
        c_int,
        c_int,
        c_int,
        c_int,
        c_uint,
        c_uint,
    ) -> c_int,
    XFlush: unsafe extern "C" fn(*mut Display) -> c_int,
) {
    let ximg = XCreateImage(
        dpy,
        visual,
        depth,
        2, /* ZPixmap */
        0,
        surface.pixels.as_mut_ptr() as *mut c_char,
        surface.width,
        surface.height,
        32,
        (surface.width * 4) as c_int,
    );
    if !ximg.is_null() {
        XPutImage(
            dpy,
            win,
            gc,
            ximg,
            0,
            0,
            0,
            0,
            surface.width,
            surface.height,
        );
        // Detach borrowed Vec pointer before freeing XImage struct shell
        (*ximg).data = std::ptr::null_mut();
        libc_free(ximg as *mut c_void);
    }
    XFlush(dpy);
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(ptr: *mut c_void);
}
