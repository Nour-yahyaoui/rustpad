//! Pure-Rust Lossless PNG Encoder/Decoder (RFC 2083: 1/2/4/8/16-bit, Grayscale, RGB, Indexed
//! `PLTE`+`tRNS`, GrayAlpha, RGBA, Non-Interlaced & Adam7 Interlaced) + Pure-Rust BMP/DIB Decoder.

use crate::canvas::Surface;
use rustpad_zip::{crc32, zlib_compress, zlib_decompress};
use std::path::Path;

pub const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

pub fn encode_surface_png(surface: &Surface) -> Vec<u8> {
    let w = surface.width;
    let h = surface.height;
    let mut rgba = Vec::with_capacity((w as usize) * (h as usize) * 4);
    for &p in &surface.pixels {
        rgba.push(((p >> 16) & 0xFF) as u8);
        rgba.push(((p >> 8) & 0xFF) as u8);
        rgba.push((p & 0xFF) as u8);
        rgba.push(((p >> 24) & 0xFF) as u8);
    }
    encode_rgba_png(w, h, &rgba)
}

/// Encode raw 8-bit `RGBA` pixels (`width * height * 4` bytes) into a lossless RFC 2083 PNG buffer.
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let w = width.max(1);
    let h = height.max(1);
    let row_len = (w as usize) * 4;
    let mut raw = Vec::with_capacity((row_len + 1) * (h as usize));

    for y in 0..h as usize {
        raw.push(0); // Filter 0 (None)
        let start = y * row_len;
        let end = start + row_len;
        if end <= rgba.len() {
            raw.extend_from_slice(&rgba[start..end]);
        } else {
            raw.resize(raw.len() + row_len, 0);
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

/// Decodes any RFC 2083 PNG (`1/2/4/8/16`-bit, color types `0/2/3/4/6`, `PLTE`+`tRNS`, normal or Adam7)
/// into `(width, height, rgba_bytes)` with zero external crate dependencies.
pub fn decode_png_rgba(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if png_bytes.len() < 33 || png_bytes[..8] != PNG_MAGIC {
        return Err("Invalid PNG signature".into());
    }

    let mut cursor = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut interlace = 0u8;
    let mut seen_ihdr = false;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
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
            break;
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
                bit_depth = chunk_data[8];
                color_type = chunk_data[9];
                interlace = chunk_data[12];
                if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
                    return Err("PNG dimensions out of supported bounds (1..=32768)".into());
                }
                if !matches!(bit_depth, 1 | 2 | 4 | 8 | 16) {
                    return Err(format!("Unsupported PNG bit depth {bit_depth}"));
                }
                if !matches!(color_type, 0 | 2 | 3 | 4 | 6) {
                    return Err(format!("Unsupported PNG color type {color_type}"));
                }
                if interlace > 1 {
                    return Err("Unsupported PNG interlace method".into());
                }
                seen_ihdr = true;
            }
            b"PLTE" => {
                palette.clear();
                for rgb in chunk_data.chunks_exact(3) {
                    palette.push([rgb[0], rgb[1], rgb[2]]);
                }
            }
            b"tRNS" => {
                trns = chunk_data.to_vec();
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
    if color_type == 3 && palette.is_empty() {
        return Err("Indexed PNG missing PLTE palette".into());
    }
    if (width as usize).saturating_mul(height as usize) > 64 * 1024 * 1024 {
        return Err("PNG pixel count exceeds 64MP safety ceiling".into());
    }

    let raw = zlib_decompress(&idat_stream).map_err(|e| format!("PNG zlib error: {}", e.0))?;
    let channels: usize = match color_type {
        0 | 3 => 1,
        4 => 2,
        2 => 3,
        6 => 4,
        _ => 1,
    };
    let bits_per_pixel = channels * (bit_depth as usize);
    let filter_bpp = bits_per_pixel.div_ceil(8).max(1);

    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];

    if interlace == 0 {
        let mut raw_pos = 0usize;
        decode_png_pass(
            &raw,
            &mut raw_pos,
            width as usize,
            height as usize,
            0,
            0,
            1,
            1,
            width as usize,
            bit_depth,
            color_type,
            filter_bpp,
            bits_per_pixel,
            &palette,
            &trns,
            &mut rgba,
        )?;
    } else {
        // RFC 2083 §2.6 Adam7 7-pass progressive interlacing
        const X_ORIG: [usize; 7] = [0, 4, 0, 2, 0, 1, 0];
        const Y_ORIG: [usize; 7] = [0, 0, 4, 0, 2, 0, 1];
        const X_STEP: [usize; 7] = [8, 8, 4, 4, 2, 2, 1];
        const Y_STEP: [usize; 7] = [8, 8, 8, 4, 4, 2, 2];

        let mut raw_pos = 0usize;
        for pass in 0..7 {
            let pw = ((width as usize).saturating_sub(X_ORIG[pass]) + X_STEP[pass] - 1)
                / X_STEP[pass];
            let ph = ((height as usize).saturating_sub(Y_ORIG[pass]) + Y_STEP[pass] - 1)
                / Y_STEP[pass];
            if pw == 0 || ph == 0 {
                continue;
            }
            decode_png_pass(
                &raw,
                &mut raw_pos,
                pw,
                ph,
                X_ORIG[pass],
                Y_ORIG[pass],
                X_STEP[pass],
                Y_STEP[pass],
                width as usize,
                bit_depth,
                color_type,
                filter_bpp,
                bits_per_pixel,
                &palette,
                &trns,
                &mut rgba,
            )?;
        }
    }

    Ok((width, height, rgba))
}

#[allow(clippy::too_many_arguments)]
fn decode_png_pass(
    raw: &[u8],
    raw_pos: &mut usize,
    pass_w: usize,
    pass_h: usize,
    x_orig: usize,
    y_orig: usize,
    x_step: usize,
    y_step: usize,
    full_w: usize,
    bit_depth: u8,
    color_type: u8,
    filter_bpp: usize,
    bits_per_pixel: usize,
    palette: &[[u8; 3]],
    trns: &[u8],
    rgba_out: &mut [u8],
) -> Result<(), String> {
    let row_bytes = (pass_w * bits_per_pixel).div_ceil(8);
    let stride = row_bytes + 1;
    let needed = stride
        .checked_mul(pass_h)
        .ok_or_else(|| "PNG pass stride overflow".to_string())?;
    if *raw_pos + needed > raw.len() {
        return Err("Decompressed PNG IDAT shorter than expected scanlines".into());
    }

    let mut prev_row = vec![0u8; row_bytes];
    let mut cur_row = vec![0u8; row_bytes];

    let trns_gray_16 = if color_type == 0 && trns.len() >= 2 {
        Some(u16::from_be_bytes([trns[0], trns[1]]))
    } else {
        None
    };
    let trns_rgb_16 = if color_type == 2 && trns.len() >= 6 {
        Some((
            u16::from_be_bytes([trns[0], trns[1]]),
            u16::from_be_bytes([trns[2], trns[3]]),
            u16::from_be_bytes([trns[4], trns[5]]),
        ))
    } else {
        None
    };

    for py in 0..pass_h {
        let base = *raw_pos + py * stride;
        let filter = raw[base];
        let src = &raw[base + 1..base + 1 + row_bytes];

        for i in 0..row_bytes {
            let left = if i >= filter_bpp {
                cur_row[i - filter_bpp]
            } else {
                0
            };
            let up = prev_row[i];
            let up_left = if i >= filter_bpp {
                prev_row[i - filter_bpp]
            } else {
                0
            };
            cur_row[i] = match filter {
                0 => src[i],
                1 => src[i].wrapping_add(left),
                2 => src[i].wrapping_add(up),
                3 => src[i].wrapping_add(((left as u16 + up as u16) >> 1) as u8),
                4 => src[i].wrapping_add(paeth_predictor(left, up, up_left)),
                _ => src[i],
            };
        }

        let dst_y = y_orig + py * y_step;
        for px in 0..pass_w {
            let dst_x = x_orig + px * x_step;
            let out_idx = (dst_y * full_w + dst_x) * 4;
            let pixel = unpack_png_pixel(
                &cur_row,
                px,
                bit_depth,
                color_type,
                palette,
                trns,
                trns_gray_16,
                trns_rgb_16,
            );
            rgba_out[out_idx..out_idx + 4].copy_from_slice(&pixel);
        }

        prev_row.copy_from_slice(&cur_row);
    }

    *raw_pos += needed;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn unpack_png_pixel(
    row: &[u8],
    px: usize,
    bit_depth: u8,
    color_type: u8,
    palette: &[[u8; 3]],
    trns: &[u8],
    trns_gray_16: Option<u16>,
    trns_rgb_16: Option<(u16, u16, u16)>,
) -> [u8; 4] {
    match (color_type, bit_depth) {
        (6, 8) => {
            let o = px * 4;
            [row[o], row[o + 1], row[o + 2], row[o + 3]]
        }
        (2, 8) => {
            let o = px * 3;
            let (r, g, b) = (row[o], row[o + 1], row[o + 2]);
            let a = if trns_rgb_16 == Some((r as u16, g as u16, b as u16)) {
                0
            } else {
                255
            };
            [r, g, b, a]
        }
        (4, 8) => {
            let o = px * 2;
            [row[o], row[o], row[o], row[o + 1]]
        }
        (0, 8) => {
            let g = row[px];
            let a = if trns_gray_16 == Some(g as u16) { 0 } else { 255 };
            [g, g, g, a]
        }
        (6, 16) => {
            let o = px * 8;
            [row[o], row[o + 2], row[o + 4], row[o + 6]]
        }
        (2, 16) => {
            let o = px * 6;
            let r16 = u16::from_be_bytes([row[o], row[o + 1]]);
            let g16 = u16::from_be_bytes([row[o + 2], row[o + 3]]);
            let b16 = u16::from_be_bytes([row[o + 4], row[o + 5]]);
            let a = if trns_rgb_16 == Some((r16, g16, b16)) {
                0
            } else {
                255
            };
            [row[o], row[o + 2], row[o + 4], a]
        }
        (4, 16) => {
            let o = px * 4;
            [row[o], row[o], row[o], row[o + 2]]
        }
        (0, 16) => {
            let o = px * 2;
            let g16 = u16::from_be_bytes([row[o], row[o + 1]]);
            let a = if trns_gray_16 == Some(g16) { 0 } else { 255 };
            [row[o], row[o], row[o], a]
        }
        (3, bd) => {
            let idx = read_subbyte_sample(row, px, bd);
            let rgb = palette.get(idx).copied().unwrap_or([0, 0, 0]);
            let a = trns.get(idx).copied().unwrap_or(255);
            [rgb[0], rgb[1], rgb[2], a]
        }
        (0, bd) => {
            let raw_val = read_subbyte_sample(row, px, bd);
            let max_v = (1usize << bd) - 1;
            let g = ((raw_val * 255 + max_v / 2) / max_v) as u8;
            let a = if trns_gray_16 == Some(raw_val as u16) {
                0
            } else {
                255
            };
            [g, g, g, a]
        }
        _ => [0, 0, 0, 255],
    }
}

#[inline]
fn read_subbyte_sample(row: &[u8], px: usize, bit_depth: u8) -> usize {
    match bit_depth {
        8 => row.get(px).copied().unwrap_or(0) as usize,
        4 => {
            let b = row.get(px >> 1).copied().unwrap_or(0);
            if px & 1 == 0 {
                (b >> 4) as usize
            } else {
                (b & 0x0F) as usize
            }
        }
        2 => {
            let b = row.get(px >> 2).copied().unwrap_or(0);
            let shift = 6 - ((px & 3) * 2);
            ((b >> shift) & 0x03) as usize
        }
        1 => {
            let b = row.get(px >> 3).copied().unwrap_or(0);
            let shift = 7 - (px & 7);
            ((b >> shift) & 0x01) as usize
        }
        _ => 0,
    }
}

/// Decodes a Windows BMP file (`"BM"` header + DIB) into `(width, height, rgba_bytes)`.
pub fn decode_bmp_rgba(bmp: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bmp.len() < 54 || &bmp[0..2] != b"BM" {
        return Err("Invalid BMP signature".into());
    }
    let pixel_offset = u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]) as usize;
    decode_dib_internal(&bmp[14..], Some((bmp, pixel_offset)))
}

/// Decodes a headerless Windows `CF_DIB` (`BITMAPINFOHEADER` + palette + pixel array) from clipboard!
pub fn decode_dib_rgba(dib: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    decode_dib_internal(dib, None)
}

fn decode_dib_internal(
    dib: &[u8],
    bmp_override: Option<(&[u8], usize)>,
) -> Result<(u32, u32, Vec<u8>), String> {
    if dib.len() < 40 {
        return Err("Truncated DIB header".into());
    }
    let hdr_size = u32::from_le_bytes([dib[0], dib[1], dib[2], dib[3]]) as usize;
    if hdr_size < 40 || hdr_size > dib.len() {
        return Err("Unsupported DIB header size".into());
    }
    let w_i32 = i32::from_le_bytes([dib[4], dib[5], dib[6], dib[7]]);
    let h_i32 = i32::from_le_bytes([dib[8], dib[9], dib[10], dib[11]]);
    let top_down = h_i32 < 0;
    let width = w_i32.unsigned_abs();
    let height = h_i32.unsigned_abs();
    if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
        return Err("BMP/DIB dimensions out of bounds".into());
    }
    if (width as usize).saturating_mul(height as usize) > 64 * 1024 * 1024 {
        return Err("BMP/DIB pixel count exceeds 64MP safety ceiling".into());
    }
    let bpp = u16::from_le_bytes([dib[14], dib[15]]);
    let compression = u32::from_le_bytes([dib[16], dib[17], dib[18], dib[19]]);
    let clr_used = u32::from_le_bytes([dib[32], dib[33], dib[34], dib[35]]) as usize;

    // BI_RGB = 0, BI_BITFIELDS = 3
    if compression != 0 && compression != 3 {
        return Err(format!("Compressed BMP mode {compression} not supported"));
    }

    let palette_entries = if bpp <= 8 {
        if clr_used > 0 {
            clr_used
        } else {
            1usize << bpp
        }
    } else if compression == 3 && hdr_size == 40 {
        3 // R, G, B 32-bit bitfield masks follow 40-byte BITMAPINFOHEADER
    } else {
        0
    };
    let palette_offset = hdr_size;
    let computed_pix_off = palette_offset + palette_entries * 4;

    let (src_buf, pix_offset) = match bmp_override {
        Some((full_bmp, off)) if off <= full_bmp.len() => (full_bmp, off),
        _ => (dib, computed_pix_off),
    };
    if pix_offset > src_buf.len() {
        return Err("BMP pixel offset out of range".into());
    }
    let pixels = &src_buf[pix_offset..];
    let row_stride = ((width as usize) * (bpp as usize) + 31) / 32 * 4;
    if row_stride.saturating_mul(height as usize) > pixels.len() {
        return Err("Truncated BMP/DIB pixel data".into());
    }

    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for row_idx in 0..height as usize {
        let dst_y = if top_down {
            row_idx
        } else {
            (height as usize) - 1 - row_idx
        };
        let row = &pixels[row_idx * row_stride..row_idx * row_stride + row_stride];
        for x in 0..width as usize {
            let out_o = (dst_y * (width as usize) + x) * 4;
            let (r, g, b, a) = match bpp {
                32 => {
                    let o = x * 4;
                    let a_raw = row[o + 3];
                    (row[o + 2], row[o + 1], row[o], if a_raw == 0 { 255 } else { a_raw })
                }
                24 => {
                    let o = x * 3;
                    (row[o + 2], row[o + 1], row[o], 255)
                }
                8 => {
                    let idx = row[x] as usize;
                    let po = palette_offset + idx * 4;
                    if po + 3 <= dib.len() {
                        (dib[po + 2], dib[po + 1], dib[po], 255)
                    } else {
                        (0, 0, 0, 255)
                    }
                }
                _ => return Err(format!("Unsupported BMP bit depth {bpp}")),
            };
            rgba[out_o] = r;
            rgba[out_o + 1] = g;
            rgba[out_o + 2] = b;
            rgba[out_o + 3] = a;
        }
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

    #[test]
    fn test_indexed_palette_png_and_bmp_decode() {
        // Build a valid 2x1 Indexed-Palette (color_type=3) PNG with PLTE + tRNS
        let mut png = Vec::new();
        png.extend_from_slice(&PNG_MAGIC);
        write_png_chunk(
            &mut png,
            b"IHDR",
            &[0, 0, 0, 2, 0, 0, 0, 1, 8, 3, 0, 0, 0],
        );
        write_png_chunk(&mut png, b"PLTE", &[255, 0, 0, 0, 255, 0]);
        write_png_chunk(&mut png, b"tRNS", &[255, 128]);
        let idat = zlib_compress(&[0, 0, 1]); // filter=0, idx 0, idx 1
        write_png_chunk(&mut png, b"IDAT", &idat);
        write_png_chunk(&mut png, b"IEND", &[]);

        let (pw, ph, prgba) = decode_png_rgba(&png).expect("indexed PNG");
        assert_eq!((pw, ph), (2, 1));
        assert_eq!(&prgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&prgba[4..8], &[0, 255, 0, 128]);

        // Build a minimal 2x1 24-bit BMP
        let mut bmp = vec![0u8; 54 + 8];
        bmp[0..2].copy_from_slice(b"BM");
        bmp[2..6].copy_from_slice(&(62u32.to_le_bytes()));
        bmp[10..14].copy_from_slice(&(54u32.to_le_bytes()));
        bmp[14..18].copy_from_slice(&(40u32.to_le_bytes()));
        bmp[18..22].copy_from_slice(&(2i32.to_le_bytes()));
        bmp[22..26].copy_from_slice(&(1i32.to_le_bytes()));
        bmp[26..28].copy_from_slice(&(1u16.to_le_bytes()));
        bmp[28..30].copy_from_slice(&(24u16.to_le_bytes()));
        // BGR pixels: blue (255,0,0 -> RGB 0,0,255), red (0,0,255 -> RGB 255,0,0)
        bmp[54..60].copy_from_slice(&[255, 0, 0, 0, 0, 255]);
        let (bw, bh, brgba) = decode_bmp_rgba(&bmp).expect("24-bit BMP");
        assert_eq!((bw, bh), (2, 1));
        assert_eq!(&brgba[0..4], &[0, 0, 255, 255]);
        assert_eq!(&brgba[4..8], &[255, 0, 0, 255]);
    }
}
