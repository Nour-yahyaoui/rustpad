//! Warm Architectural "Paper & Linen" Studio Theme Tokens.
//! Derived from fine cotton drafting paper, stone linen board, carbon ink, and terracotta wax.

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
    /// Warm architect's linen desk behind A4/Letter sheets (`#EFECE7`)
    pub const DESK_BG: Color = Color::rgb(239, 236, 231);
    /// Quiet stone-linen sidebar & dock surface (`#F7F5F0`)
    pub const SIDEBAR: Color = Color::rgb(247, 245, 240);
    /// Calm archival top chrome & status strip (`#FAF9F6`)
    pub const CHROME: Color = Color::rgb(250, 249, 246);
    /// Pure cotton document sheet & elevated menu surface (`#FFFFFF`)
    pub const PAPER: Color = Color::rgb(255, 255, 255);
    /// Tactile warm hover highlight (`#EAE6DF`)
    pub const HOVER: Color = Color::rgb(234, 230, 223);
    /// Active pressed surface (`#E1DCD2`)
    pub const ACTIVE_BG: Color = Color::rgb(225, 220, 210);
    /// Subtle stone hairline rule (`#DFDBD2`)
    pub const BORDER: Color = Color::rgb(223, 219, 210);
    /// Defined control border & splitter grip (`#C8C2B6`)
    pub const BORDER_STRONG: Color = Color::rgb(200, 194, 182);

    /// Fountain-pen carbon ink (`#181816`)
    pub const INK: Color = Color::rgb(24, 24, 22);
    /// High-contrast warm graphite secondary ink (`#52504A`)
    pub const INK_SUB: Color = Color::rgb(82, 80, 74);
    /// Editorial metadata & shortcut ink (`#7A766E`)
    pub const INK_MUTED: Color = Color::rgb(122, 118, 110);

    /// Prussian editorial ink for Word Studio (`.docx`) (`#1F4B7A`)
    pub const WORD_ACCENT: Color = Color::rgb(31, 75, 122);
    pub const WORD_SOFT: Color = Color::rgb(233, 239, 246);

    /// Archival vermilion seal for PDF Studio (`.pdf`) (`#A63626`)
    pub const PDF_ACCENT: Color = Color::rgb(166, 54, 38);
    pub const PDF_SOFT: Color = Color::rgb(248, 236, 233);

    /// Signature Rust-Terracotta studio mark & active splitter (`#B84A1C`)
    pub const RUST_ORANGE: Color = Color::rgb(184, 74, 28);
    pub const RUST_SOFT: Color = Color::rgb(250, 239, 232);

    /// Botanical forest ink for AI Agent Studio (`#2D5A47`)
    pub const AI_ACCENT: Color = Color::rgb(45, 90, 71);
    pub const AI_SOFT: Color = Color::rgb(234, 242, 238);

    /// Emerald status & checklist completion (`#1D6F4A`)
    pub const SUCCESS: Color = Color::rgb(29, 111, 74);
    pub const SUCCESS_SOFT: Color = Color::rgb(234, 243, 238);
    /// Warm amber notice (`#B45309`)
    pub const WARNING: Color = Color::rgb(180, 83, 9);
    /// Calm modal backdrop scrim
    pub const SCRIM: Color = Color::rgba(24, 24, 22, 108);
}
