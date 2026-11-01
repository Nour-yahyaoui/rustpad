//! Pure-Rust TrueType (`.ttf`) Anti-Aliased Font Engine, "Paper & Daylight" Light Theme,
//! SIMD-Friendly 2D SDF Rounded-Rect/Shadow/Icon Painter, and Universal PNG/JPEG/BMP Codecs.

pub mod canvas;
pub mod jpeg_codec;
pub mod png_codec;
pub mod theme;
pub mod ttf;

pub use canvas::{Rect, Surface, UiIcon};
pub use jpeg_codec::{
    decode_any_image_rgba, decode_any_image_thumbnail_rgba, decode_jpeg_rgba,
    detect_image_extension, hash_image_bytes,
};
pub use png_codec::{
    decode_bmp_rgba, decode_dib_rgba, decode_png_rgba, encode_rgba_png, encode_surface_png,
};
pub use theme::{
    active_theme_mode, cycle_theme_mode, load_saved_or_system_theme, save_theme_preference,
    set_active_theme_mode, Color, DaylightTheme, MidnightTheme, StudioTheme, ThemeMode,
};
pub use ttf::{FontBook, FontSlot};
