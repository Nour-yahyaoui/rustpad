//! Offline OCR & Scanned Document Text Extractor (`v3.0.0`).
//!
//! Three-tier 100% offline text recovery pipeline:
//! 1. Native PDF stream text-layer extraction (`pdf_engine` + layout reconstruction).
//! 2. Local `tesseract` CLI acceleration when installed on the host workstation.
//! 3. Pure-Rust Offline Bitmap Glyph-Template OCR fallback (`PNG` / `JPEG` / `BMP` / `PGM`)
//!    powered by Otsu luminance binarization, horizontal/vertical projection segmentation,
//!    and `16×20` normalized correlation against embedded `Sans`/`Serif`/`Mono` TrueType glyphs.

use doc_model::Document;
use pdf_engine::load_pdf_file;
use rustpad_render::{decode_any_image_rgba, FontBook, FontSlot};
use std::path::Path;
use std::process::Command;

pub fn extract_text_ocr_or_pdf(path: impl AsRef<Path>) -> Result<(Document, String), String> {
    let p = path.as_ref();
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let title = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("OCR Scan");

    if ext == "pdf" {
        let pdf = load_pdf_file(p)?;
        let doc = crate::converter::pdf_to_word(&pdf);
        let words = doc.stats().words;
        if words > 0 {
            return Ok((
                doc,
                format!(
                    "Extracted {} words across {} page(s) into editable Word document",
                    words,
                    pdf.pages.len()
                ),
            ));
        }
    }

    // Fast-path: use local host `tesseract` binary if available
    if let Ok(output) = Command::new("tesseract").arg(p).arg("stdout").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !text.trim().is_empty() {
                let doc = crate::converter::markdown_to_word(&text, title);
                return Ok((
                    doc,
                    "Completed offline OCR via local Tesseract engine".into(),
                ));
            }
        }
    }

    // Pure-Rust fallback for bitmap scans (`.png`, `.jpg`, `.jpeg`, `.bmp`, `.pgm`)
    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "pgm") {
        let raw = std::fs::read(p).map_err(|e| e.to_string())?;
        if let Some(recognized) = pure_rust_bitmap_ocr_bytes(&raw) {
            if !recognized.trim().is_empty() {
                let doc = crate::converter::markdown_to_word(&recognized, title);
                let words = doc.stats().words;
                return Ok((
                    doc,
                    format!(
                        "Completed pure-Rust offline bitmap OCR ({words} word(s) recognized)"
                    ),
                ));
            }
        }
    }

    Err(
        "No recognizable text layer or bitmap scan glyphs found (clean printed scans or optional local 'tesseract' supported)"
            .into(),
    )
}

/// Runs pure-Rust projection-profile segmentation + `16×20` multi-font TrueType template matching
/// on PNG/JPEG/BMP/PGM image bytes.
pub fn pure_rust_bitmap_ocr_bytes(bytes: &[u8]) -> Option<String> {
    let (w, h, luma) = decode_grayscale_scan(bytes)?;
    if w < 8 || h < 8 {
        return None;
    }

    let threshold = otsu_threshold(&luma);
    // Determine polarity: ink should have minority coverage (< 50%)
    let dark_count = luma.iter().filter(|&&v| v <= threshold).count();
    let invert = dark_count * 2 > luma.len();
    let ink: Vec<u8> = luma
        .iter()
        .map(|&v| {
            let is_ink = if invert { v > threshold } else { v <= threshold };
            if is_ink { 1 } else { 0 }
        })
        .collect();

    let w_us = w as usize;
    let h_us = h as usize;

    // 1. Segment horizontal text lines
    let mut row_counts = vec![0usize; h_us];
    for y in 0..h_us {
        let row = &ink[y * w_us..(y + 1) * w_us];
        row_counts[y] = row.iter().map(|&b| b as usize).sum();
    }
    let lines = segment_1d_runs(&row_counts, 1, 2);
    if lines.is_empty() {
        return None;
    }

    let templates = build_glyph_templates();
    let mut output_lines: Vec<String> = Vec::new();

    for (ly0, ly1) in lines {
        let line_h = ly1.saturating_sub(ly0);
        if line_h < 5 {
            continue;
        }
        // 2. Vertical column projection inside this line
        let mut col_counts = vec![0usize; w_us];
        for x in 0..w_us {
            let mut sum = 0usize;
            for y in ly0..ly1 {
                sum += ink[y * w_us + x] as usize;
            }
            col_counts[x] = sum;
        }
        let boxes = segment_1d_runs(&col_counts, 1, 0);
        if boxes.is_empty() {
            continue;
        }

        let avg_char_w = (boxes.iter().map(|(x0, x1)| x1 - x0).sum::<usize>() as f32)
            / (boxes.len() as f32);
        let space_gap_px = (avg_char_w * 0.52).max(3.0);

        let mut line_str = String::new();
        let mut prev_x1: Option<usize> = None;

        for (bx0, bx1) in boxes {
            if let Some(px1) = prev_x1 {
                if (bx0.saturating_sub(px1) as f32) >= space_gap_px {
                    line_str.push(' ');
                }
            }
            prev_x1 = Some(bx1);

            // Tight vertical trim inside `[bx0..bx1, ly0..ly1]`
            let mut ty0 = ly1;
            let mut ty1 = ly0;
            for y in ly0..ly1 {
                if (bx0..bx1).any(|x| ink[y * w_us + x] != 0) {
                    ty0 = ty0.min(y);
                    ty1 = ty1.max(y + 1);
                }
            }
            if ty0 >= ty1 {
                continue;
            }
            let cw = bx1 - bx0;
            let ch_h = ty1 - ty0;

            // Punctuation heuristics for tiny dots / dashes
            if ch_h <= (line_h / 3).max(2) && cw <= (line_h / 3).max(2) {
                if ty0 > ly0 + line_h / 2 {
                    line_str.push('.');
                } else {
                    line_str.push('\'');
                }
                continue;
            }
            if ch_h <= (line_h / 4).max(2) && cw > ch_h * 2 {
                line_str.push('-');
                continue;
            }

            let patch = resample_patch_16x20(&ink, w_us, bx0, ty0, cw, ch_h);
            let aspect = (cw as f32) / (ch_h.max(1) as f32);
            let best_ch = match_template(&patch, aspect, &templates);
            line_str.push(best_ch);
        }

        let trimmed = line_str.trim();
        if !trimmed.is_empty() {
            output_lines.push(trimmed.to_string());
        }
    }

    if output_lines.is_empty() {
        None
    } else {
        Some(output_lines.join("\n\n"))
    }
}

struct GlyphTemplate {
    ch: char,
    aspect: f32,
    grid: [u8; 320], // 16 * 20 binary mask
}

fn build_glyph_templates() -> Vec<GlyphTemplate> {
    let mut book = FontBook::new();
    let mut out = Vec::with_capacity(280);
    for slot in [
        FontSlot::SansRegular,
        FontSlot::SansBold,
        FontSlot::SerifRegular,
        FontSlot::MonoRegular,
    ] {
        for b in 33u8..=126u8 {
            let ch = b as char;
            let g = book.glyph(slot, ch, 28.0);
            if g.width == 0 || g.height == 0 {
                continue;
            }
            let bin: Vec<u8> = g.alpha.iter().map(|&a| if a >= 96 { 1 } else { 0 }).collect();
            let grid = resample_patch_16x20(
                &bin,
                g.width as usize,
                0,
                0,
                g.width as usize,
                g.height as usize,
            );
            out.push(GlyphTemplate {
                ch,
                aspect: (g.width as f32) / (g.height.max(1) as f32),
                grid,
            });
        }
    }
    out
}

fn match_template(patch: &[u8; 320], aspect: f32, templates: &[GlyphTemplate]) -> char {
    let mut best_score = f32::NEG_INFINITY;
    let mut best_ch = '?';
    for tpl in templates {
        let mut inter = 0u32;
        let mut union = 0u32;
        let mut eq = 0u32;
        for i in 0..320 {
            let a = patch[i];
            let b = tpl.grid[i];
            if a == b {
                eq += 1;
            }
            if (a & b) != 0 {
                inter += 1;
            }
            if (a | b) != 0 {
                union += 1;
            }
        }
        let iou = if union > 0 {
            (inter as f32) / (union as f32)
        } else {
            0.0
        };
        let acc = (eq as f32) / 320.0;
        let aspect_penalty = (aspect - tpl.aspect).abs() * 0.22;
        let score = iou * 0.65 + acc * 0.35 - aspect_penalty;
        if score > best_score {
            best_score = score;
            best_ch = tpl.ch;
        }
    }
    best_ch
}

fn resample_patch_16x20(
    src: &[u8],
    stride: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
) -> [u8; 320] {
    let mut out = [0u8; 320];
    if w == 0 || h == 0 {
        return out;
    }
    for ty in 0..20 {
        let sy = y0 + (ty * h) / 20;
        for tx in 0..16 {
            let sx = x0 + (tx * w) / 16;
            out[ty * 16 + tx] = src[sy * stride + sx];
        }
    }
    out
}

fn segment_1d_runs(counts: &[usize], min_ink: usize, merge_gap: usize) -> Vec<(usize, usize)> {
    let mut raw: Vec<(usize, usize)> = Vec::new();
    let mut in_run = false;
    let mut start = 0usize;
    for (i, &c) in counts.iter().enumerate() {
        if c >= min_ink {
            if !in_run {
                in_run = true;
                start = i;
            }
        } else if in_run {
            raw.push((start, i));
            in_run = false;
        }
    }
    if in_run {
        raw.push((start, counts.len()));
    }
    if merge_gap == 0 || raw.len() <= 1 {
        return raw;
    }
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(raw.len());
    for (s, e) in raw {
        if let Some(last) = merged.last_mut() {
            if s.saturating_sub(last.1) <= merge_gap {
                last.1 = e;
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

fn decode_grayscale_scan(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.starts_with(b"P5") {
        return parse_pgm_p5(bytes);
    }
    let (w, h, rgba) = decode_any_image_rgba(bytes).ok()?;
    let mut luma = Vec::with_capacity((w * h) as usize);
    for px in rgba.chunks_exact(4) {
        let a = px[3] as u32;
        // Composite transparent pixels onto white sheet
        let r = ((px[0] as u32 * a) + 255 * (255 - a)) / 255;
        let g = ((px[1] as u32 * a) + 255 * (255 - a)) / 255;
        let b = ((px[2] as u32 * a) + 255 * (255 - a)) / 255;
        let y = (r * 77 + g * 150 + b * 29) >> 8;
        luma.push(y as u8);
    }
    Some((w, h, luma))
}

fn parse_pgm_p5(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut idx = 2usize;
    let mut next_token = |data: &[u8], pos: &mut usize| -> Option<u32> {
        while *pos < data.len() {
            if data[*pos] == b'#' {
                while *pos < data.len() && data[*pos] != b'\n' {
                    *pos += 1;
                }
            } else if data[*pos].is_ascii_whitespace() {
                *pos += 1;
            } else {
                break;
            }
        }
        let start = *pos;
        while *pos < data.len() && data[*pos].is_ascii_digit() {
            *pos += 1;
        }
        std::str::from_utf8(&data[start..*pos]).ok()?.parse().ok()
    };
    let w = next_token(bytes, &mut idx)?;
    let h = next_token(bytes, &mut idx)?;
    let _maxv = next_token(bytes, &mut idx)?;
    if idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    let need = (w as usize).checked_mul(h as usize)?;
    if idx + need > bytes.len() {
        return None;
    }
    Some((w, h, bytes[idx..idx + need].to_vec()))
}

fn otsu_threshold(luma: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in luma {
        hist[v as usize] += 1;
    }
    let total = luma.len() as f64;
    let mut sum_all = 0.0f64;
    for (i, &c) in hist.iter().enumerate() {
        sum_all += (i as f64) * (c as f64);
    }
    let mut sum_bg = 0.0f64;
    let mut w_bg = 0.0f64;
    let mut max_var = -1.0f64;
    let mut best_t = 128u8;
    for (t, &c) in hist.iter().enumerate() {
        w_bg += c as f64;
        if w_bg == 0.0 {
            continue;
        }
        let w_fg = total - w_bg;
        if w_fg == 0.0 {
            break;
        }
        sum_bg += (t as f64) * (c as f64);
        let m_bg = sum_bg / w_bg;
        let m_fg = (sum_all - sum_bg) / w_fg;
        let var_between = w_bg * w_fg * (m_bg - m_fg) * (m_bg - m_fg);
        if var_between > max_var {
            max_var = var_between;
            best_t = t as u8;
        }
    }
    best_t
}
