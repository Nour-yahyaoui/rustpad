//! Pure-Rust Lossless PNG Encoder for `Surface` screenshots & image exports.

use crate::canvas::Surface;
use rustpad_zip::{crc32, zlib_compress};
use std::path::Path;

pub fn encode_surface_png(surface: &Surface) -> Vec<u8> {
    let w = surface.width;
    let h = surface.height;
    let mut raw = Vec::with_capacity(((w * 4 + 1) * h) as usize);

    for y in 0..h {
        raw.push(0); // Filter type 0 (None)
        let row_off = (y * w) as usize;
        for x in 0..w as usize {
            let p = surface.pixels[row_off + x];
            raw.push(((p >> 16) & 0xFF) as u8);
            raw.push(((p >> 8) & 0xFF) as u8);
            raw.push((p & 0xFF) as u8);
            raw.push(((p >> 24) & 0xFF) as u8);
        }
    }

    let idat = zlib_compress(&raw);
    let mut out = Vec::with_capacity(idat.len() + 64);
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]); // PNG magic

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // RGBA
    ihdr.push(0);
    ihdr.push(0);
    ihdr.push(0);
    write_png_chunk(&mut out, b"IHDR", &ihdr);
    write_png_chunk(&mut out, b"IDAT", &idat);
    write_png_chunk(&mut out, b"IEND", &[]);
    out
}

pub fn save_surface_png(path: impl AsRef<Path>, surface: &Surface) -> Result<(), String> {
    let bytes = encode_surface_png(surface);
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let crc_start = out.len();
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    let c = crc32(&out[crc_start..]);
    out.extend_from_slice(&c.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DaylightTheme, FontBook, FontSlot, Rect, Surface};

    #[test]
    fn test_truetype_and_sdf_daylight_render_to_png() {
        let mut fonts = FontBook::new();
        let mut surf = Surface::new(640, 240, DaylightTheme::DESK_BG);
        let sheet = Rect::new(32.0, 24.0, 576.0, 192.0);
        surf.draw_paper_shadow(sheet, 10.0, 14.0, 22);
        surf.fill_rounded_rect(sheet, 10.0, DaylightTheme::PAPER);
        surf.stroke_rounded_rect(sheet, 10.0, 1.0, DaylightTheme::BORDER);
        surf.draw_logo_64(52, 48);
        let adv = surf.draw_text(
            &mut fonts,
            FontSlot::SansBold,
            24.0,
            132.0,
            82.0,
            DaylightTheme::INK,
            "RustPad — Paper & Daylight UI",
        );
        assert!(adv > 240.0);
        let png = encode_surface_png(&surf);
        assert!(png.starts_with(&[137, 80, 78, 71]));
        assert!(png.len() > 2000);
    }
}
