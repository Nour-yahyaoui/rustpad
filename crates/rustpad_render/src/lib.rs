//! Pure-Rust TrueType (`.ttf`) Anti-Aliased Font Engine, "Paper & Daylight" Light Theme,
//! Smooth 2D SDF Rounded-Rect/Shadow/Icon Painter, and PNG Encoder.

pub mod canvas;
pub mod png_codec;
pub mod theme;
pub mod ttf;

pub use canvas::{Rect, Surface};
pub use png_codec::encode_surface_png;
pub use theme::{Color, DaylightTheme};
pub use ttf::{FontBook, FontSlot};
