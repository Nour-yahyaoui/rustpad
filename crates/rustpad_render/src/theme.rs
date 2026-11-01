//! Dual-Mode Architectural Studio Theme Engine (`v3.0.0`):
//! - `ThemeMode::Daylight` — Warm Architectural "Paper & Linen" Studio (`#EFECE7` desk, `#FFFFFF` sheet)
//! - `ThemeMode::Midnight` — Obsidian Slate & Warm Charcoal Sheet (`#111318` desk, `#1E222B` sheet, `#ECE9E2` ink)
//! - `ThemeMode::HybridDarkDesk` — Obsidian Dark Studio Chrome + Warm Cotton Paper Sheet (night print-fidelity mode)

use doc_model::RgbColor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn from_doc(c: RgbColor) -> Self {
        Self::rgb(c.r, c.g, c.b)
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    #[inline]
    pub const fn pack_argb(self) -> u32 {
        ((self.a as u32) << 24)
            | ((self.r as u32) << 16)
            | ((self.g as u32) << 8)
            | (self.b as u32)
    }
}

/// Studio appearance mode (`Ctrl+Shift+D` cycles or sets directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThemeMode {
    /// Warm architectural linen desk + pure cotton white sheet.
    Daylight = 0,
    /// Full dark mode: obsidian slate chrome + low-glare charcoal sheet + warm parchment ink.
    Midnight = 1,
    /// Hybrid dark desk: obsidian dark chrome + warm cotton paper sheet for print preview at night.
    HybridDarkDesk = 2,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Daylight
    }
}

impl ThemeMode {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Midnight,
            2 => Self::HybridDarkDesk,
            _ => Self::Daylight,
        }
    }

    pub const fn is_dark_chrome(self) -> bool {
        matches!(self, Self::Midnight | Self::HybridDarkDesk)
    }

    pub const fn is_dark_sheet(self) -> bool {
        matches!(self, Self::Midnight)
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Daylight => Self::Midnight,
            Self::Midnight => Self::HybridDarkDesk,
            Self::HybridDarkDesk => Self::Daylight,
        }
    }

    pub const fn toggle_dark_light(self) -> Self {
        match self {
            Self::Daylight => Self::Midnight,
            Self::Midnight | Self::HybridDarkDesk => Self::Daylight,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Daylight => "Daylight (Paper & Linen)",
            Self::Midnight => "Midnight (Full Dark Mode)",
            Self::HybridDarkDesk => "Dark Desk + Warm Paper",
        }
    }

    pub const fn short_badge(self) -> &'static str {
        match self {
            Self::Daylight => "Light",
            Self::Midnight => "Dark",
            Self::HybridDarkDesk => "Hybrid",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Daylight => "daylight",
            Self::Midnight => "midnight",
            Self::HybridDarkDesk => "hybrid",
        }
    }

    pub fn from_slug(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "dark" | "midnight" | "night" => Self::Midnight,
            "2" | "hybrid" | "dark-desk" | "darkdesk" => Self::HybridDarkDesk,
            _ => Self::Daylight,
        }
    }
}

static ACTIVE_THEME_MODE: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn active_theme_mode() -> ThemeMode {
    ThemeMode::from_u8(ACTIVE_THEME_MODE.load(Ordering::Relaxed))
}

#[inline]
pub fn set_active_theme_mode(mode: ThemeMode) {
    ACTIVE_THEME_MODE.store(mode as u8, Ordering::Relaxed);
}

#[inline]
pub fn cycle_theme_mode() -> ThemeMode {
    let next = active_theme_mode().next();
    set_active_theme_mode(next);
    next
}

fn theme_pref_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("RustPad");
        let _ = std::fs::create_dir_all(&dir);
        return Some(dir.join("theme.conf"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".rustpad_theme"));
    }
    None
}

/// Loads persisted user theme preference, `RUSTPAD_THEME` env override, or OS dark-mode setting.
pub fn load_saved_or_system_theme() -> ThemeMode {
    if let Ok(env_val) = std::env::var("RUSTPAD_THEME") {
        if !env_val.trim().is_empty() {
            let mode = ThemeMode::from_slug(&env_val);
            set_active_theme_mode(mode);
            return mode;
        }
    }
    if let Some(p) = theme_pref_path() {
        if let Ok(saved) = std::fs::read_to_string(p) {
            let mode = ThemeMode::from_slug(&saved);
            set_active_theme_mode(mode);
            return mode;
        }
    }
    let sys = detect_os_dark_preference();
    set_active_theme_mode(sys);
    sys
}

pub fn save_theme_preference(mode: ThemeMode) {
    set_active_theme_mode(mode);
    if let Some(p) = theme_pref_path() {
        let _ = std::fs::write(p, format!("{}\n", mode.slug()));
    }
}

#[cfg(target_os = "windows")]
fn detect_os_dark_preference() -> ThemeMode {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut isize,
        ) -> i32;
        fn RegQueryValueExW(
            hKey: isize,
            lpValueName: *const u16,
            lpReserved: *const u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
        fn RegCloseKey(hKey: isize) -> i32;
    }

    unsafe {
        let subkey: Vec<u16> =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
                .encode_utf16()
                .collect();
        let val_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let mut hkey: isize = 0;
        // HKEY_CURRENT_USER = 0x80000001_u32 as isize, KEY_READ = 0x20019
        if RegOpenKeyExW(
            0x8000_0001u32 as i32 as isize,
            subkey.as_ptr(),
            0,
            0x20019,
            &mut hkey,
        ) == 0
        {
            let mut val: u32 = 1;
            let mut sz: u32 = 4;
            let mut reg_type: u32 = 0;
            let status = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                std::ptr::null(),
                &mut reg_type,
                (&mut val as *mut u32) as *mut u8,
                &mut sz,
            );
            RegCloseKey(hkey);
            if status == 0 && val == 0 {
                return ThemeMode::Midnight;
            }
        }
    }
    ThemeMode::Daylight
}

#[cfg(not(target_os = "windows"))]
fn detect_os_dark_preference() -> ThemeMode {
    if let Ok(gtk) = std::env::var("GTK_THEME") {
        if gtk.to_ascii_lowercase().contains("dark") {
            return ThemeMode::Midnight;
        }
    }
    if let Ok(cs) = std::env::var("COLORFGBG") {
        if cs.ends_with(";0") {
            return ThemeMode::Midnight;
        }
    }
    ThemeMode::Daylight
}

/// Static Daylight constants preserved for lossless test & baseline compatibility.
pub struct DaylightTheme;

impl DaylightTheme {
    pub const DESK_BG: Color = Color::rgb(239, 236, 231);
    pub const SIDEBAR: Color = Color::rgb(247, 245, 240);
    pub const CHROME: Color = Color::rgb(250, 249, 246);
    pub const PAPER: Color = Color::rgb(255, 255, 255);
    pub const HOVER: Color = Color::rgb(234, 230, 223);
    pub const ACTIVE_BG: Color = Color::rgb(225, 220, 210);
    pub const BORDER: Color = Color::rgb(223, 219, 210);
    pub const BORDER_STRONG: Color = Color::rgb(200, 194, 182);
    pub const INK: Color = Color::rgb(24, 24, 22);
    pub const INK_SUB: Color = Color::rgb(82, 80, 74);
    pub const INK_MUTED: Color = Color::rgb(122, 118, 110);
    pub const WORD_ACCENT: Color = Color::rgb(31, 75, 122);
    pub const WORD_SOFT: Color = Color::rgb(233, 239, 246);
    pub const PDF_ACCENT: Color = Color::rgb(166, 54, 38);
    pub const PDF_SOFT: Color = Color::rgb(248, 236, 233);
    pub const RUST_ORANGE: Color = Color::rgb(184, 74, 28);
    pub const RUST_SOFT: Color = Color::rgb(250, 239, 232);
    pub const AI_ACCENT: Color = Color::rgb(45, 90, 71);
    pub const AI_SOFT: Color = Color::rgb(234, 242, 238);
    pub const SUCCESS: Color = Color::rgb(29, 111, 74);
    pub const SUCCESS_SOFT: Color = Color::rgb(234, 243, 238);
    pub const WARNING: Color = Color::rgb(180, 83, 9);
    pub const SCRIM: Color = Color::rgba(24, 24, 22, 108);
}

/// Obsidian Midnight Dark Mode constants (WCAG AAA/AA calibrated warm-slate hierarchy).
pub struct MidnightTheme;

impl MidnightTheme {
    pub const DESK_BG: Color = Color::rgb(16, 18, 23);
    pub const SIDEBAR: Color = Color::rgb(22, 25, 32);
    pub const CHROME: Color = Color::rgb(27, 31, 39);
    /// Elevated menus, modals, popovers & midnight document sheet (`#212631`)
    pub const PAPER: Color = Color::rgb(33, 38, 49);
    pub const HOVER: Color = Color::rgb(42, 48, 62);
    pub const ACTIVE_BG: Color = Color::rgb(52, 60, 78);
    pub const BORDER: Color = Color::rgb(46, 52, 66);
    pub const BORDER_STRONG: Color = Color::rgb(70, 79, 98);
    /// Warm archival parchment primary ink (`#ECE9E2`) — zero harsh pure-white eye strain
    pub const INK: Color = Color::rgb(236, 233, 226);
    pub const INK_SUB: Color = Color::rgb(180, 176, 168);
    pub const INK_MUTED: Color = Color::rgb(132, 130, 124);
    pub const WORD_ACCENT: Color = Color::rgb(114, 168, 242);
    pub const WORD_SOFT: Color = Color::rgb(31, 47, 72);
    pub const PDF_ACCENT: Color = Color::rgb(240, 112, 94);
    pub const PDF_SOFT: Color = Color::rgb(64, 32, 31);
    pub const RUST_ORANGE: Color = Color::rgb(238, 126, 76);
    pub const RUST_SOFT: Color = Color::rgb(62, 38, 28);
    pub const AI_ACCENT: Color = Color::rgb(96, 198, 152);
    pub const AI_SOFT: Color = Color::rgb(26, 54, 44);
    pub const SUCCESS: Color = Color::rgb(78, 196, 138);
    pub const SUCCESS_SOFT: Color = Color::rgb(25, 55, 42);
    pub const WARNING: Color = Color::rgb(245, 166, 62);
    pub const SCRIM: Color = Color::rgba(6, 8, 12, 168);
}

/// Runtime-adaptive theme dispatcher (`O(1)` relaxed atomic load — zero heap allocation).
pub struct StudioTheme;

impl StudioTheme {
    #[inline]
    pub fn desk_bg() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::DESK_BG
        } else {
            DaylightTheme::DESK_BG
        }
    }

    #[inline]
    pub fn sidebar() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::SIDEBAR
        } else {
            DaylightTheme::SIDEBAR
        }
    }

    #[inline]
    pub fn chrome() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::CHROME
        } else {
            DaylightTheme::CHROME
        }
    }

    /// Elevated UI card / modal / popover surface.
    #[inline]
    pub fn paper() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::PAPER
        } else {
            DaylightTheme::PAPER
        }
    }

    /// Physical A4 / Letter document sheet surface (dark in `Midnight`, warm paper in `Daylight` & `HybridDarkDesk`).
    #[inline]
    pub fn sheet_paper() -> Color {
        match active_theme_mode() {
            ThemeMode::Daylight => DaylightTheme::PAPER,
            ThemeMode::Midnight => Color::rgb(29, 33, 42),
            ThemeMode::HybridDarkDesk => Color::rgb(250, 248, 243),
        }
    }

    #[inline]
    pub fn sheet_border() -> Color {
        if active_theme_mode().is_dark_sheet() {
            MidnightTheme::BORDER_STRONG
        } else {
            DaylightTheme::BORDER
        }
    }

    #[inline]
    pub fn sheet_ink() -> Color {
        if active_theme_mode().is_dark_sheet() {
            MidnightTheme::INK
        } else {
            DaylightTheme::INK
        }
    }

    #[inline]
    pub fn sheet_ink_muted() -> Color {
        if active_theme_mode().is_dark_sheet() {
            MidnightTheme::INK_MUTED
        } else {
            DaylightTheme::INK_MUTED
        }
    }

    #[inline]
    pub fn sheet_ink_sub() -> Color {
        if active_theme_mode().is_dark_sheet() {
            MidnightTheme::INK_SUB
        } else {
            DaylightTheme::INK_SUB
        }
    }

    #[inline]
    pub fn hover() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::HOVER
        } else {
            DaylightTheme::HOVER
        }
    }

    #[inline]
    pub fn active_bg() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::ACTIVE_BG
        } else {
            DaylightTheme::ACTIVE_BG
        }
    }

    #[inline]
    pub fn border() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::BORDER
        } else {
            DaylightTheme::BORDER
        }
    }

    #[inline]
    pub fn border_strong() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::BORDER_STRONG
        } else {
            DaylightTheme::BORDER_STRONG
        }
    }

    #[inline]
    pub fn ink() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::INK
        } else {
            DaylightTheme::INK
        }
    }

    #[inline]
    pub fn ink_sub() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::INK_SUB
        } else {
            DaylightTheme::INK_SUB
        }
    }

    #[inline]
    pub fn ink_muted() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::INK_MUTED
        } else {
            DaylightTheme::INK_MUTED
        }
    }

    #[inline]
    pub fn word_accent() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::WORD_ACCENT
        } else {
            DaylightTheme::WORD_ACCENT
        }
    }

    #[inline]
    pub fn word_soft() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::WORD_SOFT
        } else {
            DaylightTheme::WORD_SOFT
        }
    }

    #[inline]
    pub fn pdf_accent() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::PDF_ACCENT
        } else {
            DaylightTheme::PDF_ACCENT
        }
    }

    #[inline]
    pub fn pdf_soft() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::PDF_SOFT
        } else {
            DaylightTheme::PDF_SOFT
        }
    }

    #[inline]
    pub fn rust_orange() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::RUST_ORANGE
        } else {
            DaylightTheme::RUST_ORANGE
        }
    }

    #[inline]
    pub fn rust_soft() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::RUST_SOFT
        } else {
            DaylightTheme::RUST_SOFT
        }
    }

    #[inline]
    pub fn ai_accent() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::AI_ACCENT
        } else {
            DaylightTheme::AI_ACCENT
        }
    }

    #[inline]
    pub fn ai_soft() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::AI_SOFT
        } else {
            DaylightTheme::AI_SOFT
        }
    }

    #[inline]
    pub fn success() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::SUCCESS
        } else {
            DaylightTheme::SUCCESS
        }
    }

    #[inline]
    pub fn success_soft() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::SUCCESS_SOFT
        } else {
            DaylightTheme::SUCCESS_SOFT
        }
    }

    #[inline]
    pub fn warning() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::WARNING
        } else {
            DaylightTheme::WARNING
        }
    }

    #[inline]
    pub fn scrim() -> Color {
        if active_theme_mode().is_dark_chrome() {
            MidnightTheme::SCRIM
        } else {
            DaylightTheme::SCRIM
        }
    }

    /// Adapts document span ink purely at canvas render time when `ThemeMode::Midnight` dark sheet
    /// is active — keeping saved `.docx` / `.pdf` colors 100% untouched on disk!
    #[inline]
    pub fn adapt_doc_ink(c: RgbColor, has_highlight: bool) -> Color {
        if !active_theme_mode().is_dark_sheet() || has_highlight {
            return Color::from_doc(c);
        }
        let lum = (c.r as u32 * 299 + c.g as u32 * 587 + c.b as u32 * 114) / 1000;
        if lum < 52 {
            // Default near-black / slate ink -> warm parchment `#ECE9E2`
            MidnightTheme::INK
        } else if c == RgbColor::MUTED_GRAY || (lum < 130 && c.r.abs_diff(c.b) < 28) {
            MidnightTheme::INK_SUB
        } else if c == RgbColor::COBALT || (c.b > c.r.saturating_add(60) && lum < 145) {
            Color::rgb(118, 174, 248)
        } else if c == RgbColor::CORAL || (c.r > c.b.saturating_add(70) && lum < 135) {
            Color::rgb(250, 120, 138)
        } else if c == RgbColor::EMERALD || (c.g > c.r.saturating_add(50) && lum < 140) {
            Color::rgb(82, 210, 152)
        } else {
            Color::from_doc(c)
        }
    }

    /// Adapts pastel table header / stripe tints when `ThemeMode::Midnight` dark sheet is active.
    #[inline]
    pub fn adapt_table_cell_bg(c: RgbColor) -> Color {
        if !active_theme_mode().is_dark_sheet() {
            return Color::from_doc(c);
        }
        let lum = (c.r as u32 * 299 + c.g as u32 * 587 + c.b as u32 * 114) / 1000;
        if lum >= 215 {
            // Map bright pastel table fills to subtle dark-slate equivalents preserving hue bias
            let dr = (36 + (c.r as i32 - 235).clamp(-12, 18)) as u8;
            let dg = (41 + (c.g as i32 - 235).clamp(-12, 18)) as u8;
            let db = (53 + (c.b as i32 - 235).clamp(-12, 22)) as u8;
            Color::rgb(dr, dg, db)
        } else {
            Color::from_doc(c)
        }
    }

    #[inline]
    pub fn adapt_table_border(c: RgbColor) -> Color {
        if !active_theme_mode().is_dark_sheet() {
            return Color::from_doc(c);
        }
        let lum = (c.r as u32 * 299 + c.g as u32 * 587 + c.b as u32 * 114) / 1000;
        if lum >= 150 {
            Color::rgb(74, 83, 102)
        } else {
            Color::from_doc(c)
        }
    }
}
