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

#[cfg(test)]
mod tests {
    use super::*;
    use doc_model::FontFamily;

    #[test]
    fn test_styled_font_slots_and_calibrated_width_drawing() {
        let mut fonts = FontBook::new();
        assert_eq!(
            FontSlot::select_styled(FontFamily::Sans, true, true),
            FontSlot::SansBoldItalic
        );
        assert_eq!(
            FontSlot::select_styled(FontFamily::Serif, true, false),
            FontSlot::SerifBold
        );
        assert_eq!(
            FontSlot::select_styled(FontFamily::Mono, false, true),
            FontSlot::MonoItalic
        );

        let serif_reg = fonts.measure_text(FontSlot::SerifRegular, "RustPad Typography", 16.0);
        let serif_bold = fonts.measure_text(FontSlot::SerifBold, "RustPad Typography", 16.0);
        assert!(serif_bold > serif_reg);

        let g_italic = fonts.glyph(FontSlot::SansItalic, 'I', 20.0);
        assert!(g_italic.width > 0 && g_italic.height > 0);

        let mut surf = Surface::new(360, 48, Color::rgb(255, 255, 255));
        let target_w = 88.0;
        let end_x = surf.draw_text_to_width(
            &mut fonts,
            FontSlot::SansBold,
            14.0,
            12.0,
            28.0,
            target_w,
            Color::rgb(15, 23, 42),
            "nour yahyaoui",
        );
        assert!((end_x - (12.0 + target_w)).abs() < 0.05);
    }
}
