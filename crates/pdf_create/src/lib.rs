//! Pure-Rust ISO 32000 Vector PDF 1.7 Document Generator.
//! Converts `doc_model::Document` (and custom PDF Studio pages) into crisp, multi-page vector PDFs.

use doc_model::{
    Alignment, Block, Document, FontFamily, ImageBlock, ListKind, Paragraph, ParagraphStyle,
    RevisionKind, RgbColor, SpanStyle, TableBlock,
};
use rustpad_zip::{zlib_compress, zlib_decompress};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PdfCreateOptions {
    pub compress_streams: bool,
    pub watermark_override: Option<String>,
    pub pdfa_conformance: bool,
    pub show_revisions: bool,
}

impl Default for PdfCreateOptions {
    fn default() -> Self {
        Self {
            compress_streams: true,
            watermark_override: None,
            pdfa_conformance: false,
            show_revisions: true,
        }
    }
}

pub fn export_document_to_pdf_file(
    path: impl AsRef<Path>,
    doc: &Document,
    opts: &PdfCreateOptions,
) -> Result<(), String> {
    let bytes = export_document_to_pdf_bytes(doc, opts);
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn export_document_to_pdf_bytes(doc: &Document, opts: &PdfCreateOptions) -> Vec<u8> {
    let (page_w, page_h) = doc
        .page_setup
        .paper_size
        .dimensions_pt(doc.page_setup.landscape);
    let m_top = doc.page_setup.margin_top_pt;
    let m_bot = doc.page_setup.margin_bottom_pt;
    let m_left = doc.page_setup.margin_left_pt;
    let m_right = doc.page_setup.margin_right_pt;
    let content_w = (page_w - m_left - m_right).max(120.0);

    let mut page_streams: Vec<String> = vec![String::new()];
    let mut pdf_images: Vec<PdfImageXObject> = Vec::new();
    let mut cursor_y = page_h - m_top;

    let start_new_page = |streams: &mut Vec<String>, y: &mut f32| {
        streams.push(String::new());
        *y = page_h - m_top;
    };

    for block in &doc.blocks {
        match block {
            Block::PageBreak => {
                start_new_page(&mut page_streams, &mut cursor_y);
            }
            Block::HorizontalRule => {
                if cursor_y - 18.0 < m_bot {
                    start_new_page(&mut page_streams, &mut cursor_y);
                }
                cursor_y -= 8.0;
                let s = page_streams.last_mut().unwrap();
                let _ = writeln!(
                    s,
                    "q 0.80 0.84 0.88 RG 0.8 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                    m_left,
                    cursor_y,
                    page_w - m_right,
                    cursor_y
                );
                cursor_y -= 10.0;
            }
            Block::Paragraph(p) => {
                render_paragraph_pdf(
                    p,
                    opts.show_revisions && doc.track_changes,
                    m_left,
                    content_w,
                    page_h,
                    m_top,
                    m_bot,
                    &mut cursor_y,
                    &mut page_streams,
                );
            }
            Block::Table(t) => {
                render_table_pdf(
                    t,
                    m_left,
                    content_w,
                    page_h,
                    m_top,
                    m_bot,
                    &mut cursor_y,
                    &mut page_streams,
                );
            }
            Block::Image(img) => {
                let aspect = img.aspect_ratio();
                let box_w = img.width_pt.clamp(48.0, content_w);
                let box_h = if img.width_pt > content_w {
                    (box_w / aspect).clamp(32.0, 560.0)
                } else {
                    img.height_pt.clamp(32.0, 560.0)
                };
                let caption_h = if img.caption.trim().is_empty() {
                    0.0
                } else {
                    16.0
                };
                if cursor_y - box_h - caption_h - 14.0 < m_bot {
                    start_new_page(&mut page_streams, &mut cursor_y);
                }
                let img_x = match img.alignment {
                    Alignment::Center => m_left + (content_w - box_w) * 0.5,
                    Alignment::Right => m_left + (content_w - box_w),
                    _ => m_left,
                };
                let s = page_streams.last_mut().unwrap();
                let y_bot = cursor_y - box_h;

                if let Some(xobj) = prepare_pdf_image_xobject(img) {
                    let alias = format!("Im{}", pdf_images.len() + 1);
                    pdf_images.push(xobj);
                    let _ = writeln!(
                        s,
                        "q {:.2} 0 0 {:.2} {:.2} {:.2} cm /{alias} Do Q",
                        box_w, box_h, img_x, y_bot
                    );
                } else {
                    let _ = writeln!(
                        s,
                        "q 0.95 0.96 0.98 rg 0.82 0.86 0.91 RG 0.75 w {:.2} {:.2} {:.2} {:.2} re B Q",
                        img_x, y_bot, box_w, box_h
                    );
                    write_pdf_text_cmd(
                        s,
                        "F1",
                        10.0,
                        img_x + 12.0,
                        y_bot + box_h * 0.5,
                        RgbColor::MUTED_GRAY,
                        &format!("[Image: {}]", img.name),
                    );
                }

                if !img.caption.trim().is_empty() {
                    let cap = img.caption.trim();
                    let cap_w = estimate_text_width(cap, 9.5, FontFamily::Sans);
                    let cap_x = match img.alignment {
                        Alignment::Center => m_left + ((content_w - cap_w) * 0.5).max(0.0),
                        Alignment::Right => m_left + (content_w - cap_w).max(0.0),
                        _ => img_x,
                    };
                    write_pdf_text_cmd(
                        s,
                        "F3",
                        9.5,
                        cap_x,
                        y_bot - 12.0,
                        RgbColor::MUTED_GRAY,
                        cap,
                    );
                }
                cursor_y = y_bot - caption_h - 12.0;
            }
        }
    }

    let total_pages = page_streams.len();
    let watermark = opts
        .watermark_override
        .as_ref()
        .or(doc.page_setup.watermark.as_ref());

    // Decorate each page with optional watermark, header, and footer
    let mut final_streams: Vec<String> = Vec::with_capacity(total_pages);
    for (idx, body_ops) in page_streams.into_iter().enumerate() {
        let mut full = String::new();
        if let Some(wm) = watermark {
            if !wm.trim().is_empty() {
                // Draw subtle diagonal light-slate watermark behind body text
                let _ = writeln!(
                    full,
                    "q 0.90 0.92 0.95 rg BT /F2 54 Tf 0.7071 0.7071 -0.7071 0.7071 {:.2} {:.2} Tm ({}) Tj ET Q",
                    page_w * 0.22,
                    page_h * 0.34,
                    escape_pdf_winansi(wm)
                );
            }
        }
        full.push_str(&body_ops);

        if !doc.page_setup.header_text.is_empty() {
            write_pdf_text_cmd(
                &mut full,
                "F1",
                9.0,
                m_left,
                page_h - 34.0,
                RgbColor::MUTED_GRAY,
                &doc.page_setup.header_text,
            );
            let _ = writeln!(
                full,
                "q 0.88 0.91 0.94 RG 0.5 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                m_left,
                page_h - 40.0,
                page_w - m_right,
                page_h - 40.0
            );
        }

        let page_num_str = if doc.page_setup.show_page_numbers {
            format!("Page {} of {}", idx + 1, total_pages)
        } else {
            String::new()
        };
        if !doc.page_setup.footer_text.is_empty() || !page_num_str.is_empty() {
            let _ = writeln!(
                full,
                "q 0.88 0.91 0.94 RG 0.5 w {:.2} 42.0 m {:.2} 42.0 l S Q",
                m_left,
                page_w - m_right
            );
            if !doc.page_setup.footer_text.is_empty() {
                write_pdf_text_cmd(
                    &mut full,
                    "F1",
                    9.0,
                    m_left,
                    28.0,
                    RgbColor::MUTED_GRAY,
                    &doc.page_setup.footer_text,
                );
            }
            if !page_num_str.is_empty() {
                let approx_w = estimate_text_width(&page_num_str, 9.0, FontFamily::Sans);
                write_pdf_text_cmd(
                    &mut full,
                    "F1",
                    9.0,
                    (page_w - m_right - approx_w).max(m_left),
                    28.0,
                    RgbColor::MUTED_GRAY,
                    &page_num_str,
                );
            }
        }
        final_streams.push(full);
    }

    assemble_pdf_binary_with_images(
        &doc.title,
        &doc.author,
        page_w,
        page_h,
        &final_streams,
        &pdf_images,
        opts.compress_streams,
        opts.pdfa_conformance,
    )
}

#[derive(Clone)]
struct WordToken {
    text: String,
    style: SpanStyle,
    width_pt: f32,
}

fn render_paragraph_pdf(
    p: &Paragraph,
    show_revisions: bool,
    m_left: f32,
    content_w: f32,
    page_h: f32,
    m_top: f32,
    m_bot: f32,
    cursor_y: &mut f32,
    streams: &mut Vec<String>,
) {
    let base_size = p
        .spans
        .first()
        .map(|s| s.style.font_size_pt)
        .unwrap_or_else(|| p.style.default_font_size());
    let space_before = match p.style {
        ParagraphStyle::Title => 6.0,
        ParagraphStyle::Heading1 => 12.0,
        ParagraphStyle::Heading2 => 9.0,
        ParagraphStyle::Heading3 => 7.0,
        _ => 2.0,
    };
    *cursor_y -= space_before;

    let indent_pt = (p.indent_level as f32) * 18.0
        + if p.list_kind != ListKind::None || p.style == ParagraphStyle::Quote {
            18.0
        } else {
            0.0
        };
    let avail_w = (content_w - indent_pt).max(80.0);

    // Tokenize into words preserving SpanStyle (and redline markup if enabled)
    let mut tokens: Vec<WordToken> = Vec::new();
    for span in &p.spans {
        let mut span_style = span.style.clone();
        if let Some(rev) = &span.revision {
            match rev.kind {
                RevisionKind::Deletion => {
                    if !show_revisions {
                        continue;
                    }
                    span_style.color = RgbColor::CORAL;
                    span_style.strikethrough = true;
                }
                RevisionKind::Insertion => {
                    if show_revisions {
                        span_style.color = RgbColor::EMERALD;
                        span_style.underline = true;
                    }
                }
            }
        }
        let mut current = String::new();
        for ch in span.text.chars() {
            if ch == '\n' {
                if !current.is_empty() {
                    let w = estimate_text_width(&current, span_style.font_size_pt, span_style.font_family);
                    tokens.push(WordToken {
                        text: std::mem::take(&mut current),
                        style: span_style.clone(),
                        width_pt: w,
                    });
                }
                tokens.push(WordToken {
                    text: "\n".into(),
                    style: span_style.clone(),
                    width_pt: 0.0,
                });
            } else if ch == ' ' {
                current.push(' ');
                let w = estimate_text_width(&current, span_style.font_size_pt, span_style.font_family);
                tokens.push(WordToken {
                    text: std::mem::take(&mut current),
                    style: span_style.clone(),
                    width_pt: w,
                });
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            let w = estimate_text_width(&current, span_style.font_size_pt, span_style.font_family);
            tokens.push(WordToken {
                text: current,
                style: span_style,
                width_pt: w,
            });
        }
    }

    let mut lines: Vec<Vec<WordToken>> = Vec::new();
    let mut cur_line: Vec<WordToken> = Vec::new();
    let mut cur_w = 0.0f32;
    for tok in tokens {
        if tok.text == "\n" {
            lines.push(std::mem::take(&mut cur_line));
            cur_w = 0.0;
            continue;
        }
        if !cur_line.is_empty() && cur_w + tok.width_pt > avail_w {
            lines.push(std::mem::take(&mut cur_line));
            cur_w = 0.0;
        }
        cur_w += tok.width_pt;
        cur_line.push(tok);
    }
    if !cur_line.is_empty() || lines.is_empty() {
        lines.push(cur_line);
    }

    let line_h = (base_size * p.line_spacing).max(base_size + 3.0);

    for (line_idx, line) in lines.iter().enumerate() {
        if *cursor_y - line_h < m_bot {
            streams.push(String::new());
            *cursor_y = page_h - m_top;
        }
        *cursor_y -= base_size;
        let baseline_y = *cursor_y;
        let s = streams.last_mut().unwrap();

        if show_revisions && p.has_revisions() {
            let _ = writeln!(
                s,
                "q 0.88 0.11 0.28 RG 1.8 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                (m_left - 8.0).max(8.0),
                baseline_y - 3.0,
                (m_left - 8.0).max(8.0),
                baseline_y + base_size
            );
        }

        if p.style == ParagraphStyle::Quote {
            let _ = writeln!(
                s,
                "q 0.15 0.39 0.92 RG 2.2 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                m_left + 6.0,
                baseline_y - 3.0,
                m_left + 6.0,
                baseline_y + base_size
            );
        }

        if line_idx == 0 {
            let marker = match p.list_kind {
                ListKind::Bullet => Some("\\225".to_string()), // WinAnsi bullet • (0x95 = octal 225)
                ListKind::Numbered(n) => Some(format!("{n}.")),
                ListKind::Checklist(done) => Some(if done { "[x]" } else { "[ ]" }.to_string()),
                ListKind::None => None,
            };
            if let Some(mk) = marker {
                write_pdf_raw_winansi(
                    s,
                    "F2",
                    base_size,
                    m_left + indent_pt - 14.0,
                    baseline_y,
                    RgbColor::COBALT,
                    &mk,
                );
            }
        }

        let line_w: f32 = line.iter().map(|t| t.width_pt).sum();
        let mut x = match p.alignment {
            Alignment::Left | Alignment::Justify => m_left + indent_pt,
            Alignment::Center => m_left + indent_pt + ((avail_w - line_w) * 0.5).max(0.0),
            Alignment::Right => m_left + indent_pt + (avail_w - line_w).max(0.0),
        };

        // Coalesce adjacent same-style tokens on the line so PDF streams emit clean full-line runs
        let mut coalesced: Vec<WordToken> = Vec::with_capacity(line.len());
        for tok in line {
            if let Some(prev) = coalesced.last_mut() {
                if prev.style == tok.style {
                    prev.text.push_str(&tok.text);
                    prev.width_pt += tok.width_pt;
                    continue;
                }
            }
            coalesced.push(tok.clone());
        }

        for tok in &coalesced {
            let clean_text = tok.text.trim_end_matches(' ');
            if clean_text.is_empty() {
                x += tok.width_pt;
                continue;
            }
            if let Some(hl) = tok.style.highlight {
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} rg {:.2} {:.2} {:.2} {:.2} re f Q",
                    hl.r as f32 / 255.0,
                    hl.g as f32 / 255.0,
                    hl.b as f32 / 255.0,
                    x,
                    baseline_y - 2.5,
                    tok.width_pt,
                    tok.style.font_size_pt + 3.0
                );
            }
            let font_id = select_pdf_font(tok.style.font_family, tok.style.bold, tok.style.italic);
            write_pdf_text_cmd(
                s,
                font_id,
                tok.style.font_size_pt,
                x,
                baseline_y,
                tok.style.color,
                clean_text,
            );
            if tok.style.underline {
                let c = tok.style.color;
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} RG 0.65 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                    x,
                    baseline_y - 1.8,
                    x + tok.width_pt,
                    baseline_y - 1.8
                );
            }
            if tok.style.strikethrough {
                let c = tok.style.color;
                let sy = baseline_y + tok.style.font_size_pt * 0.32;
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} RG 0.65 w {:.2} {:.2} m {:.2} {:.2} l S Q",
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                    x,
                    sy,
                    x + tok.width_pt,
                    sy
                );
            }
            x += tok.width_pt * 1.08;
        }

        *cursor_y -= line_h - base_size;
    }

    *cursor_y -= 5.0;
}

fn render_table_pdf(
    t: &TableBlock,
    m_left: f32,
    content_w: f32,
    page_h: f32,
    m_top: f32,
    m_bot: f32,
    cursor_y: &mut f32,
    streams: &mut Vec<String>,
) {
    let cols = t.cols.max(1);
    let col_w = content_w / (cols as f32);
    let row_h = 24.0f32;
    *cursor_y -= 4.0;

    for row in &t.rows {
        if *cursor_y - row_h < m_bot {
            streams.push(String::new());
            *cursor_y = page_h - m_top;
        }
        let y_top = *cursor_y;
        let y_bot = y_top - row_h;
        let s = streams.last_mut().unwrap();

        for (c_idx, cell) in row.iter().enumerate().take(cols) {
            let x = m_left + (c_idx as f32) * col_w;
            if let Some(bg) = cell.bg_color {
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} rg {:.2} {:.2} {:.2} {:.2} re f Q",
                    bg.r as f32 / 255.0,
                    bg.g as f32 / 255.0,
                    bg.b as f32 / 255.0,
                    x,
                    y_bot,
                    col_w,
                    row_h
                );
            }
            let bc = t.border_color;
            let _ = writeln!(
                s,
                "q {:.3} {:.3} {:.3} RG 0.6 w {:.2} {:.2} {:.2} {:.2} re S Q",
                bc.r as f32 / 255.0,
                bc.g as f32 / 255.0,
                bc.b as f32 / 255.0,
                x,
                y_bot,
                col_w,
                row_h
            );

            let font_id = if cell.bold { "F2" } else { "F1" };
            let max_chars = ((col_w - 12.0) / 5.3).max(4.0) as usize;
            let clipped: String = cell.text.chars().take(max_chars).collect();
            let est_w = (clipped.chars().count() as f32) * 5.2;
            let tx = match cell.alignment {
                doc_model::Alignment::Center => x + ((col_w - est_w) * 0.5).max(6.0),
                doc_model::Alignment::Right => (x + col_w - 6.0 - est_w).max(x + 6.0),
                _ => x + 6.0,
            };
            write_pdf_text_cmd(
                s,
                font_id,
                10.0,
                tx,
                y_bot + 7.5,
                RgbColor::INK_SLATE,
                &clipped,
            );
        }
        *cursor_y = y_bot;
    }

    *cursor_y -= 10.0;
}

pub fn select_pdf_font(family: FontFamily, bold: bool, italic: bool) -> &'static str {
    match (family, bold, italic) {
        (FontFamily::Sans, false, false) => "F1",
        (FontFamily::Sans, true, false) => "F2",
        (FontFamily::Sans, false, true) => "F3",
        (FontFamily::Sans, true, true) => "F4",
        (FontFamily::Serif, false, false) => "F5",
        (FontFamily::Serif, true, false) => "F6",
        (FontFamily::Serif, false, true) => "F7",
        (FontFamily::Serif, true, true) => "F8",
        (FontFamily::Mono, false, false) => "F9",
        (FontFamily::Mono, true, false) => "F10",
        (FontFamily::Mono, false, true) => "F11",
        (FontFamily::Mono, true, true) => "F12",
    }
}

pub fn estimate_text_width(text: &str, size_pt: f32, family: FontFamily) -> f32 {
    if family == FontFamily::Mono {
        return (text.chars().count() as f32) * size_pt * 0.60;
    }
    let mut units = 0.0f32;
    for ch in text.chars() {
        units += match ch {
            ' ' => 0.28,
            'i' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' => 0.27,
            'f' | 'j' | 'r' | 't' | '(' | ')' | '[' | ']' => 0.35,
            'm' | 'w' => 0.82,
            'M' | 'W' => 0.90,
            c if c.is_ascii_uppercase() => 0.67,
            _ => 0.53,
        };
    }
    units * size_pt
}

pub fn write_pdf_text_cmd(
    s: &mut String,
    font_id: &str,
    size_pt: f32,
    x: f32,
    y: f32,
    color: RgbColor,
    text: &str,
) {
    let encoded = escape_pdf_winansi(text);
    write_pdf_raw_winansi(s, font_id, size_pt, x, y, color, &encoded);
}

fn write_pdf_raw_winansi(
    s: &mut String,
    font_id: &str,
    size_pt: f32,
    x: f32,
    y: f32,
    color: RgbColor,
    escaped_winansi: &str,
) {
    let _ = writeln!(
        s,
        "q {:.3} {:.3} {:.3} rg BT /{font_id} {:.2} Tf {:.2} {:.2} Td ({escaped_winansi}) Tj ET Q",
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        size_pt,
        x,
        y
    );
}

pub fn escape_pdf_winansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        let byte: u8 = match ch {
            '(' => {
                out.push_str("\\(");
                continue;
            }
            ')' => {
                out.push_str("\\)");
                continue;
            }
            '\\' => {
                out.push_str("\\\\");
                continue;
            }
            '\n' | '\r' | '\t' => b' ',
            c if (c as u32) >= 32 && (c as u32) <= 126 => c as u8,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '€' => 0x80,
            '…' => 0x85,
            '·' => 0xB7,
            '×' => 0xD7,
            c if (c as u32) >= 0xA0 && (c as u32) <= 0xFF => c as u8,
            _ => b'?',
        };
        if byte.is_ascii_graphic() || byte == b' ' {
            out.push(byte as char);
        } else {
            let _ = write!(out, "\\{:03o}", byte);
        }
    }
    out
}

pub fn assemble_pdf_binary(
    title: &str,
    author: &str,
    page_w: f32,
    page_h: f32,
    page_streams: &[String],
    compress: bool,
) -> Vec<u8> {
    assemble_pdf_binary_ext(title, author, page_w, page_h, page_streams, compress, false)
}

#[derive(Clone)]
pub struct PdfImageXObject {
    pub width: u32,
    pub height: u32,
    pub color_space: &'static str,
    pub filter: &'static str,
    pub data: Vec<u8>,
}

fn parse_jpeg_sof_dimensions(bytes: &[u8]) -> Option<(u32, u32, u8)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut pos = 2usize;
    while pos + 3 < bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let m = bytes[pos];
        pos += 1;
        if m == 0xD9 {
            break;
        }
        if m == 0x00 || m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        if pos + 2 > bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            break;
        }
        if matches!(m, 0xC0 | 0xC1 | 0xC2) && seg_len >= 8 {
            let h = u16::from_be_bytes([bytes[pos + 3], bytes[pos + 4]]) as u32;
            let w = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
            let nf = bytes[pos + 7];
            if w > 0 && h > 0 && matches!(nf, 1 | 3 | 4) {
                return Some((w, h, nf));
            }
        }
        pos += seg_len;
    }
    None
}

fn extract_png_rgb_for_pdf(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.len() < 33 || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return None;
    }
    let mut w = 0usize;
    let mut h = 0usize;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut interlace = 0u8;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut idat = Vec::new();
    let mut cur = 8usize;

    while cur + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[cur], bytes[cur + 1], bytes[cur + 2], bytes[cur + 3]])
            as usize;
        let ctype = &bytes[cur + 4..cur + 8];
        let ds = cur + 8;
        let de = ds.checked_add(len)?;
        if de + 4 > bytes.len() {
            break;
        }
        let chunk = &bytes[ds..de];
        match ctype {
            b"IHDR" if chunk.len() >= 13 => {
                w = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize;
                h = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as usize;
                bit_depth = chunk[8];
                color_type = chunk[9];
                interlace = chunk[12];
            }
            b"PLTE" => {
                for rgb in chunk.chunks_exact(3) {
                    palette.push([rgb[0], rgb[1], rgb[2]]);
                }
            }
            b"IDAT" => idat.extend_from_slice(chunk),
            b"IEND" => break,
            _ => {}
        }
        cur = de + 4;
    }

    if w == 0 || h == 0 || bit_depth != 8 || interlace != 0 || w * h > 16 * 1024 * 1024 {
        return None;
    }
    let bpp = match color_type {
        0 | 3 => 1usize,
        4 => 2usize,
        2 => 3usize,
        6 => 4usize,
        _ => return None,
    };
    let stride = w * bpp;
    let raw = zlib_decompress(&idat).ok()?;
    if raw.len() < h * (1 + stride) {
        return None;
    }

    let mut prev = vec![0u8; stride];
    let mut row = vec![0u8; stride];
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let off = y * (1 + stride);
        let ftype = raw[off];
        let src = &raw[off + 1..off + 1 + stride];
        for x in 0..stride {
            let a = if x >= bpp { row[x - bpp] } else { 0 };
            let b = prev[x];
            let c = if x >= bpp { prev[x - bpp] } else { 0 };
            let pred = match ftype {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) >> 1) as u8,
                4 => {
                    let p = a as i32 + b as i32 - c as i32;
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
                _ => return None,
            };
            row[x] = src[x].wrapping_add(pred);
        }
        for px in 0..w {
            match color_type {
                0 => {
                    let g = row[px];
                    rgb.extend_from_slice(&[g, g, g]);
                }
                2 => {
                    let i = px * 3;
                    rgb.extend_from_slice(&row[i..i + 3]);
                }
                3 => {
                    let c = palette.get(row[px] as usize).copied().unwrap_or([255, 255, 255]);
                    rgb.extend_from_slice(&c);
                }
                4 => {
                    let g = row[px * 2] as u16;
                    let a = row[px * 2 + 1] as u16;
                    let v = ((g * a + 255 * (255 - a)) / 255) as u8;
                    rgb.extend_from_slice(&[v, v, v]);
                }
                6 => {
                    let i = px * 4;
                    let a = row[i + 3] as u16;
                    let r = ((row[i] as u16 * a + 255 * (255 - a)) / 255) as u8;
                    let g = ((row[i + 1] as u16 * a + 255 * (255 - a)) / 255) as u8;
                    let b = ((row[i + 2] as u16 * a + 255 * (255 - a)) / 255) as u8;
                    rgb.extend_from_slice(&[r, g, b]);
                }
                _ => {}
            }
        }
        prev.copy_from_slice(&row);
    }
    Some((w as u32, h as u32, rgb))
}

fn prepare_pdf_image_xobject(img: &ImageBlock) -> Option<PdfImageXObject> {
    // 1. Native JPEG `/DCTDecode` passthrough (lossless byte embedding, ultra-compact PDF!)
    if let Some((w, h, nf)) = parse_jpeg_sof_dimensions(&img.png_bytes) {
        let cs = match nf {
            1 => "/DeviceGray",
            4 => "/DeviceCMYK",
            _ => "/DeviceRGB",
        };
        return Some(PdfImageXObject {
            width: w,
            height: h,
            color_space: cs,
            filter: "/DCTDecode",
            data: img.png_bytes.clone(),
        });
    }

    // 2. Decoded `rgba_preview` -> composited 24-bit RGB over paper white -> `/FlateDecode`
    if let Some((w, h, rgba)) = &img.rgba_preview {
        let px_count = (*w as usize).saturating_mul(*h as usize);
        if *w > 0 && *h > 0 && rgba.len() >= px_count * 4 {
            let mut rgb = Vec::with_capacity(px_count * 3);
            for px in rgba[..px_count * 4].chunks_exact(4) {
                let a = px[3] as u16;
                rgb.push(((px[0] as u16 * a + 255 * (255 - a)) / 255) as u8);
                rgb.push(((px[1] as u16 * a + 255 * (255 - a)) / 255) as u8);
                rgb.push(((px[2] as u16 * a + 255 * (255 - a)) / 255) as u8);
            }
            return Some(PdfImageXObject {
                width: *w,
                height: *h,
                color_space: "/DeviceRGB",
                filter: "/FlateDecode",
                data: zlib_compress(&rgb),
            });
        }
    }

    // 3. Headless CLI PNG fallback -> unfilter & compress RGB via `/FlateDecode`
    if let Some((w, h, rgb)) = extract_png_rgb_for_pdf(&img.png_bytes) {
        return Some(PdfImageXObject {
            width: w,
            height: h,
            color_space: "/DeviceRGB",
            filter: "/FlateDecode",
            data: zlib_compress(&rgb),
        });
    }

    None
}

pub fn assemble_pdf_binary_ext(
    title: &str,
    author: &str,
    page_w: f32,
    page_h: f32,
    page_streams: &[String],
    compress: bool,
    pdfa_conformance: bool,
) -> Vec<u8> {
    assemble_pdf_binary_with_images(
        title,
        author,
        page_w,
        page_h,
        page_streams,
        &[],
        compress,
        pdfa_conformance,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn assemble_pdf_binary_with_images(
    title: &str,
    author: &str,
    page_w: f32,
    page_h: f32,
    page_streams: &[String],
    images: &[PdfImageXObject],
    compress: bool,
    pdfa_conformance: bool,
) -> Vec<u8> {
    const FONT_NAMES: [(&str, &str); 12] = [
        ("F1", "Helvetica"),
        ("F2", "Helvetica-Bold"),
        ("F3", "Helvetica-Oblique"),
        ("F4", "Helvetica-BoldOblique"),
        ("F5", "Times-Roman"),
        ("F6", "Times-Bold"),
        ("F7", "Times-Italic"),
        ("F8", "Times-BoldItalic"),
        ("F9", "Courier"),
        ("F10", "Courier-Bold"),
        ("F11", "Courier-Oblique"),
        ("F12", "Courier-BoldOblique"),
    ];

    // Object numbering:
    // 1: Catalog
    // 2: Pages
    // 3: Info
    // 4..=15: 12 standard Type1 Font objects
    // 16: Shared Resources dictionary (/Font + optional /XObject)
    // 17 .. 17 + page_count*2: Pairs of (Page Obj, Content Stream Obj)
    // 17 + page_count*2 .. + images.len(): Image XObject streams
    // 17 + page_count*2 + images.len(): Optional PDF/A-2b XMP Metadata
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let page_count = page_streams.len().max(1);
    let mut kids_refs = String::new();
    for i in 0..page_count {
        let page_obj_id = 17 + i * 2;
        let _ = write!(kids_refs, "{page_obj_id} 0 R ");
    }

    let img_base_obj_id = 17 + page_count * 2;
    let xmp_obj_id = img_base_obj_id + images.len();
    if pdfa_conformance {
        objects.push(
            format!("<< /Type /Catalog /Pages 2 0 R /Metadata {xmp_obj_id} 0 R >>").into_bytes(),
        );
    } else {
        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    }
    objects.push(
        format!("<< /Type /Pages /Kids [ {kids_refs}] /Count {page_count} >>").into_bytes(),
    );
    objects.push(
        format!(
            "<< /Title ({}) /Author ({}) /Creator (RustPad Native Desktop Studio) /Producer (RustPad PDF 1.7 Engine) >>",
            escape_pdf_winansi(title),
            escape_pdf_winansi(author)
        )
        .into_bytes(),
    );

    let mut font_map_entries = String::new();
    for (idx, (alias, base_font)) in FONT_NAMES.iter().enumerate() {
        let obj_id = 4 + idx;
        objects.push(
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{base_font} /Encoding /WinAnsiEncoding >>"
            )
            .into_bytes(),
        );
        let _ = write!(font_map_entries, "/{alias} {obj_id} 0 R ");
    }

    if images.is_empty() {
        objects.push(format!("<< /Font << {font_map_entries}>> >>").into_bytes()); // Obj 16
    } else {
        let mut xobj_entries = String::new();
        for k in 0..images.len() {
            let xid = img_base_obj_id + k;
            let _ = write!(xobj_entries, "/Im{} {xid} 0 R ", k + 1);
        }
        objects.push(
            format!("<< /Font << {font_map_entries}>> /XObject << {xobj_entries}>> >>")
                .into_bytes(),
        );
    }

    for (i, stream_str) in page_streams.iter().enumerate() {
        let content_obj_id = 17 + i * 2 + 1;
        let page_dict = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.2} {page_h:.2}] /Resources 16 0 R /Contents {content_obj_id} 0 R >>"
        );
        objects.push(page_dict.into_bytes());

        let raw_bytes = stream_str.as_bytes();
        let (filter_header, payload) = if compress {
            let z = zlib_compress(raw_bytes);
            (" /Filter /FlateDecode", z)
        } else {
            ("", raw_bytes.to_vec())
        };
        let mut stream_obj =
            format!("<< /Length {}{filter_header} >>\nstream\n", payload.len()).into_bytes();
        stream_obj.extend_from_slice(&payload);
        stream_obj.extend_from_slice(b"\nendstream");
        objects.push(stream_obj);
    }

    for img in images {
        let mut img_obj = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {} /BitsPerComponent 8 /Filter {} /Length {} >>\nstream\n",
            img.width,
            img.height,
            img.color_space,
            img.filter,
            img.data.len()
        )
        .into_bytes();
        img_obj.extend_from_slice(&img.data);
        img_obj.extend_from_slice(b"\nendstream");
        objects.push(img_obj);
    }

    if pdfa_conformance {
        let xmp = build_pdfa_xmp_packet(title, author);
        let mut meta_obj = format!(
            "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n",
            xmp.len()
        )
        .into_bytes();
        meta_obj.extend_from_slice(xmp.as_bytes());
        meta_obj.extend_from_slice(b"\nendstream");
        objects.push(meta_obj);
    }

    let mut out: Vec<u8> = Vec::with_capacity(4096);
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::with_capacity(objects.len() + 1);
    offsets.push(0);
    for (idx, obj_body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let obj_id = idx + 1;
        out.extend_from_slice(format!("{obj_id} 0 obj\n").as_bytes());
        out.extend_from_slice(obj_body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = out.len();
    let total_objs = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {total_objs}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for &off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {total_objs} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

pub fn build_pdfa_xmp_packet(title: &str, author: &str) -> String {
    let clean_title = title.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let clean_author = author.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/"
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <pdfaid:part>2</pdfaid:part>
      <pdfaid:conformance>B</pdfaid:conformance>
      <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{clean_title}</rdf:li></rdf:Alt></dc:title>
      <dc:creator><rdf:Seq><rdf:li>{clean_author}</rdf:li></rdf:Seq></dc:creator>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_model::TemplateKind;

    #[test]
    fn test_export_welcome_to_valid_pdf() {
        let doc = TemplateKind::WelcomeGuide.build();
        let pdf = export_document_to_pdf_bytes(&doc, &PdfCreateOptions::default());
        assert!(pdf.starts_with(b"%PDF-1.7"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.len() > 600);
    }

    #[test]
    fn test_export_document_embeds_real_pdf_image_xobjects() {
        let mut doc = Document::blank();
        let mut img = ImageBlock::from_decoded(
            "Diagram",
            2,
            2,
            vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255],
            vec![137, 80, 78, 71, 13, 10, 26, 10],
            400.0,
        );
        img.caption = "Figure 1: System Topology".into();
        doc.blocks.push(Block::Image(img));

        let opts = PdfCreateOptions {
            compress_streams: false,
            ..PdfCreateOptions::default()
        };
        let pdf = export_document_to_pdf_bytes(&doc, &opts);
        let pdf_ascii = String::from_utf8_lossy(&pdf);
        assert!(pdf_ascii.contains("/Subtype /Image"));
        assert!(pdf_ascii.contains("/Im1 Do"));
        assert!(pdf_ascii.contains("Figure 1: System Topology"));
    }
}
