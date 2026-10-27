//! Pure-Rust TrueType (`.ttf`) Anti-Aliased Font Engine, "Paper & Daylight" Light Theme,
//! SIMD-Friendly 2D SDF Rounded-Rect/Shadow/Icon Painter, and Lossless PNG Encoder/Decoder.

pub mod canvas;
pub mod png_codec;
pub mod theme;
pub mod ttf;

pub use canvas::{Rect, Surface, UiIcon};
pub use png_codec::{decode_png_rgba, encode_surface_png};
pub use theme::{Color, DaylightTheme};
pub use ttf::{FontBook, FontSlot};
