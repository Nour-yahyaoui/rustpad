//! Pure-Rust ISO 32000 Vector PDF 1.7 Document Generator.
//! Converts `doc_model::Document` (and custom PDF Studio pages) into crisp, multi-page vector PDFs.

use doc_model::{
    Alignment, Block, Document, FontFamily, ListKind, Paragraph, ParagraphStyle, RgbColor,
    SpanStyle, TableBlock,
};
use rustpad_zip::zlib_compress;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PdfCreateOptions {
    pub compress_streams: bool,
    pub watermark_override: Option<String>,
}

impl Default for PdfCreateOptions {
    fn default() -> Self {
        Self {
            compress_streams: true,
            watermark_override: None,
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
                let box_h = img.height_pt.clamp(40.0, 260.0);
                let box_w = img.width_pt.clamp(60.0, content_w);
                if cursor_y - box_h - 14.0 < m_bot {
                    start_new_page(&mut page_streams, &mut cursor_y);
                }
                let s = page_streams.last_mut().unwrap();
                let y_bot = cursor_y - box_h;
                let _ = writeln!(
                    s,
                    "q 0.95 0.96 0.98 rg 0.82 0.86 0.91 RG 0.75 w {:.2} {:.2} {:.2} {:.2} re B Q",
                    m_left, y_bot, box_w, box_h
                );
                write_pdf_text_cmd(
                    s,
                    "F1",
                    10.0,
                    m_left + 12.0,
                    y_bot + box_h * 0.5,
                    RgbColor::MUTED_GRAY,
                    &format!("[Image: {}]", img.name),
                );
                cursor_y = y_bot - 12.0;
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

    assemble_pdf_binary(
        &doc.title,
        &doc.author,
        page_w,
        page_h,
        &final_streams,
        opts.compress_streams,
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

    // Tokenize into words preserving SpanStyle
    let mut tokens: Vec<WordToken> = Vec::new();
    for span in &p.spans {
        let mut current = String::new();
        for ch in span.text.chars() {
            if ch == '\n' {
                if !current.is_empty() {
                    let w = estimate_text_width(&current, span.style.font_size_pt, span.style.font_family);
                    tokens.push(WordToken {
                        text: std::mem::take(&mut current),
                        style: span.style.clone(),
                        width_pt: w,
                    });
                }
                tokens.push(WordToken {
                    text: "\n".into(),
                    style: span.style.clone(),
                    width_pt: 0.0,
                });
            } else if ch == ' ' {
                current.push(' ');
                let w = estimate_text_width(&current, span.style.font_size_pt, span.style.font_family);
                tokens.push(WordToken {
                    text: std::mem::take(&mut current),
                    style: span.style.clone(),
                    width_pt: w,
                });
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            let w = estimate_text_width(&current, span.style.font_size_pt, span.style.font_family);
            tokens.push(WordToken {
                text: current,
                style: span.style.clone(),
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
                ListKind::Bullet => Some("\x95".to_string()), // WinAnsi bullet •
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

        for tok in line {
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
                &tok.text,
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
            x += tok.width_pt;
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
            write_pdf_text_cmd(
                s,
                font_id,
                10.0,
                x + 6.0,
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
    // 16: Shared Resources dictionary
    // 17..: Pairs of (Page Obj, Content Stream Obj)
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let page_count = page_streams.len().max(1);
    let mut kids_refs = String::new();
    for i in 0..page_count {
        let page_obj_id = 17 + i * 2;
        let _ = write!(kids_refs, "{page_obj_id} 0 R ");
    }

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
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

    objects.push(format!("<< /Font << {font_map_entries}>> >>").into_bytes()); // Obj 16

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
}
