//! Pure-Rust Lossless PNG Encoder & RFC 2083 Scanline Decoder (`IHDR`/`IDAT` Sub/Up/Average/Paeth)
//! for `Surface` screenshots, exports, and live embedded `.docx` image rendering.

use crate::canvas::Surface;
use rustpad_zip::{crc32, zlib_compress, zlib_decompress};
use std::path::Path;

const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

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
    out.extend_from_slice(&PNG_MAGIC);

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

/// Decodes a standard 8-bit non-interlaced PNG image (`Grayscale`, `RGB`, `GrayAlpha`, or `RGBA`)
/// into `(width, height, rgba_bytes)` with zero external crate dependencies.
pub fn decode_png_rgba(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if png_bytes.len() < 33 || png_bytes[..8] != PNG_MAGIC {
        return Err("Invalid PNG signature".into());
    }

    let mut cursor = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut color_type = 0u8;
    let mut seen_ihdr = false;
    let mut idat_stream: Vec<u8> = Vec::new();

    while cursor + 12 <= png_bytes.len() {
        let len = u32::from_be_bytes([
            png_bytes[cursor],
            png_bytes[cursor + 1],
            png_bytes[cursor + 2],
            png_bytes[cursor + 3],
        ]) as usize;
        let ctype = &png_bytes[cursor + 4..cursor + 8];
        let data_start = cursor + 8;
        let data_end = data_start.saturating_add(len);
        if data_end + 4 > png_bytes.len() {
            return Err("Truncated PNG chunk".into());
        }
        let chunk_data = &png_bytes[data_start..data_end];
        cursor = data_end + 4;

        match ctype {
            b"IHDR" => {
                if chunk_data.len() < 13 {
                    return Err("Malformed PNG IHDR".into());
                }
                width = u32::from_be_bytes([
                    chunk_data[0],
                    chunk_data[1],
                    chunk_data[2],
                    chunk_data[3],
                ]);
                height = u32::from_be_bytes([
                    chunk_data[4],
                    chunk_data[5],
                    chunk_data[6],
                    chunk_data[7],
                ]);
                let bit_depth = chunk_data[8];
                color_type = chunk_data[9];
                let interlace = chunk_data[12];
                if width == 0 || height == 0 || width > 8192 || height > 8192 {
                    return Err("PNG dimensions out of supported bounds (1..=8192)".into());
                }
                if bit_depth != 8 {
                    return Err(format!("Unsupported PNG bit depth {bit_depth} (expected 8-bit)"));
                }
                if interlace != 0 {
                    return Err("Interlaced Adam7 PNG not supported".into());
                }
                seen_ihdr = true;
            }
            b"IDAT" => {
                idat_stream.extend_from_slice(chunk_data);
            }
            b"IEND" => break,
            _ => {}
        }
    }

    if !seen_ihdr || idat_stream.is_empty() {
        return Err("Missing IHDR or IDAT in PNG".into());
    }

    let bpp: usize = match color_type {
        0 => 1, // Grayscale
        2 => 3, // RGB
        4 => 2, // Grayscale + Alpha
        6 => 4, // RGBA
        other => return Err(format!("Unsupported PNG color type {other}")),
    };

    let row_bytes = (width as usize) * bpp;
    let stride = row_bytes + 1;
    let expected_raw = stride
        .checked_mul(height as usize)
        .ok_or_else(|| "PNG stride overflow".to_string())?;
    if expected_raw > 64 * 1024 * 1024 {
        return Err("Decoded PNG exceeds 64 MiB safety limit".into());
    }

    let raw = zlib_decompress(&idat_stream).map_err(|e| format!("PNG zlib error: {}", e.0))?;
    if raw.len() < expected_raw {
        return Err("Decompressed PNG IDAT shorter than expected scanlines".into());
    }

    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    let mut prev_row = vec![0u8; row_bytes];
    let mut cur_row = vec![0u8; row_bytes];

    for y in 0..height as usize {
        let base = y * stride;
        let filter = raw[base];
        let src = &raw[base + 1..base + 1 + row_bytes];

        for i in 0..row_bytes {
            let left = if i >= bpp { cur_row[i - bpp] } else { 0 };
            let up = prev_row[i];
            let up_left = if i >= bpp { prev_row[i - bpp] } else { 0 };
            cur_row[i] = match filter {
                0 => src[i],
                1 => src[i].wrapping_add(left),
                2 => src[i].wrapping_add(up),
                3 => src[i].wrapping_add(((left as u16 + up as u16) >> 1) as u8),
                4 => src[i].wrapping_add(paeth_predictor(left, up, up_left)),
                _ => src[i],
            };
        }

        match color_type {
            6 => rgba.extend_from_slice(&cur_row),
            2 => {
                for px in cur_row.chunks_exact(3) {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
            4 => {
                for px in cur_row.chunks_exact(2) {
                    rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
                }
            }
            _ => {
                for &g in &cur_row {
                    rgba.extend_from_slice(&[g, g, g, 255]);
                }
            }
        }

        prev_row.copy_from_slice(&cur_row);
    }

    Ok((width, height, rgba))
}

#[inline]
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = (a as i32) + (b as i32) - (c as i32);
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
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

        let (dw, dh, decoded_rgba) = decode_png_rgba(&png).expect("decode PNG lossless roundtrip");
        assert_eq!((dw, dh), (640, 240));
        assert_eq!(decoded_rgba.len(), 640 * 240 * 4);
    }
}
