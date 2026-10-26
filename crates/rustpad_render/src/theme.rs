//! "Paper & Daylight" Light Theme Design Tokens (`DaylightTheme`).

use doc_model::RgbColor;

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

pub struct DaylightTheme;

impl DaylightTheme {
    /// Soft Alabaster workspace desk backdrop (`#F4F6F9`)
    pub const DESK_BG: Color = Color::rgb(0xF4, 0xF6, 0xF9);
    /// Pure white document paper & popover surface (`#FFFFFF`)
    pub const PAPER: Color = Color::rgb(0xFF, 0xFF, 0xFF);
    /// Clean header bar & toolbar surface (`#FAFBFC`)
    pub const CHROME: Color = Color::rgb(0xFA, 0xFB, 0xFC);
    /// Subtle rail / secondary panel surface (`#F8FAFC`)
    pub const SIDEBAR: Color = Color::rgb(0xF8, 0xFA, 0xFC);
    /// Subtle tactile button hover (`#EEF2F6`)
    pub const HOVER: Color = Color::rgb(0xEE, 0xF2, 0xF6);
    /// Hairline crisp border (`#E2E8F0`)
    pub const BORDER: Color = Color::rgb(0xE2, 0xE8, 0xF0);
    /// Stronger control outline (`#CBD5E1`)
    pub const BORDER_STRONG: Color = Color::rgb(0xCB, 0xD5, 0xE1);
    /// Deep Slate primary ink (`#0F172A`)
    pub const INK: Color = Color::rgb(0x0F, 0x17, 0x2A);
    /// Secondary UI text & icons (`#475569`)
    pub const INK_SUB: Color = Color::rgb(0x47, 0x55, 0x69);
    /// Muted hints & shortcut badges (`#94A3B8`)
    pub const INK_MUTED: Color = Color::rgb(0x94, 0xA3, 0xB8);
    /// Royal Cobalt Word accent (`#2563EB`)
    pub const WORD_ACCENT: Color = Color::rgb(0x25, 0x63, 0xEB);
    /// Soft Cobalt active pill fill (`#EFF6FF`)
    pub const WORD_SOFT: Color = Color::rgb(0xEF, 0xF6, 0xFF);
    /// Warm Coral PDF Studio accent (`#E11D48`)
    pub const PDF_ACCENT: Color = Color::rgb(0xE1, 0x1D, 0x48);
    /// Soft Coral active pill fill (`#FFF1F2`)
    pub const PDF_SOFT: Color = Color::rgb(0xFF, 0xF1, 0xF2);
    /// RustPad warm brand orange (`#EA580C`)
    pub const RUST_ORANGE: Color = Color::rgb(0xEA, 0x58, 0x0C);
    /// Saved / success emerald (`#059669`)
    pub const SUCCESS: Color = Color::rgb(0x05, 0x96, 0x69);
    /// Soft Emerald badge background (`#ECFDF5`)
    pub const SUCCESS_SOFT: Color = Color::rgb(0xEC, 0xFD, 0xF5);
    /// Translucent modal backdrop
    pub const SCRIM: Color = Color::rgba(15, 23, 42, 110);
}
