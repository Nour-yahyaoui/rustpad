//! Pure-Rust PDF 1.7 Serializer & Stream Optimizer for `PdfDocument`.

use crate::{load_pdf_bytes, PdfAnnotation, PdfDocument, PdfPage};
use doc_model::RgbColor;
use pdf_create::{escape_pdf_winansi, select_pdf_font, write_pdf_text_cmd};
use rustpad_zip::zlib_compress;
use std::fmt::Write as _;
use std::path::Path;

pub fn save_pdf_file(
    path: impl AsRef<Path>,
    doc: &PdfDocument,
    compress: bool,
) -> Result<(), String> {
    let bytes = save_pdf_bytes(doc, compress);
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn compress_pdf_bytes(input: &[u8]) -> Result<(Vec<u8>, usize, usize), String> {
    let before = input.len();
    let doc = load_pdf_bytes(input)?;
    let compressed = save_pdf_bytes(&doc, true);
    let after = compressed.len();
    Ok((compressed, before, after))
}

pub fn save_pdf_bytes(doc: &PdfDocument, compress: bool) -> Vec<u8> {
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

    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()); // 1
    objects.push(Vec::new()); // 2: placeholder for /Pages
    objects.push(
        format!(
            "<< /Title ({}) /Author ({}) /Creator (RustPad PDF Studio) /Producer (RustPad PDF 1.7) >>",
            escape_pdf_winansi(&doc.title),
            escape_pdf_winansi(&doc.author)
        )
        .into_bytes(),
    ); // 3

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
    objects.push(format!("<< /Font << {font_map_entries}>> >>").into_bytes()); // 16

    let mut page_obj_ids = Vec::with_capacity(doc.pages.len().max(1));
    for page in &doc.pages {
        let stream_str = render_pdf_page_stream(page);
        let raw_bytes = stream_str.as_bytes();
        let (filter_hdr, payload) = if compress {
            (" /Filter /FlateDecode", zlib_compress(raw_bytes))
        } else {
            ("", raw_bytes.to_vec())
        };
        let mut content_obj =
            format!("<< /Length {}{filter_hdr} >>\nstream\n", payload.len()).into_bytes();
        content_obj.extend_from_slice(&payload);
        content_obj.extend_from_slice(b"\nendstream");
        let content_id = objects.len() + 1;
        objects.push(content_obj);

        // Emit standard PDF /Text annotation objects for StickyNotes
        let mut annot_ids = Vec::new();
        for annot in &page.annotations {
            if let PdfAnnotation::StickyNote { x, y, author, note } = annot {
                let aid = objects.len() + 1;
                objects.push(
                    format!(
                        "<< /Type /Annot /Subtype /Text /Rect [{:.2} {:.2} {:.2} {:.2}] /T ({}) /Contents ({}) /C [0.98 0.80 0.08] >>",
                        x,
                        y,
                        x + 20.0,
                        y + 20.0,
                        escape_pdf_winansi(author),
                        escape_pdf_winansi(note)
                    )
                    .into_bytes(),
                );
                annot_ids.push(aid);
            }
        }

        let annots_entry = if annot_ids.is_empty() {
            String::new()
        } else {
            let mut r = String::from(" /Annots [ ");
            for aid in annot_ids {
                let _ = write!(r, "{aid} 0 R ");
            }
            r.push(']');
            r
        };

        let page_id = objects.len() + 1;
        page_obj_ids.push(page_id);
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources 16 0 R /Contents {content_id} 0 R{annots_entry} >>",
                page.width_pt, page.height_pt
            )
            .into_bytes(),
        );
    }

    let mut kids = String::new();
    for pid in &page_obj_ids {
        let _ = write!(kids, "{pid} 0 R ");
    }
    objects[1] = format!(
        "<< /Type /Pages /Kids [ {kids}] /Count {} >>",
        page_obj_ids.len()
    )
    .into_bytes();

    let mut out = Vec::with_capacity(4096);
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize];
    for (idx, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let id = idx + 1;
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = out.len();
    let total = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
    for &off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {total} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

fn render_pdf_page_stream(page: &PdfPage) -> String {
    let mut s = String::new();

    if let Some(wm) = &page.watermark {
        if !wm.trim().is_empty() {
            let _ = writeln!(
                s,
                "q 0.90 0.92 0.95 rg BT /F2 52 Tf 0.7071 0.7071 -0.7071 0.7071 {:.2} {:.2} Tm ({}) Tj ET Q",
                page.width_pt * 0.22,
                page.height_pt * 0.34,
                escape_pdf_winansi(wm)
            );
        }
    }

    // 1. Highlights first (underneath text)
    for annot in &page.annotations {
        if let PdfAnnotation::Highlight { x, y, w, h, color } = annot {
            let _ = writeln!(
                s,
                "q {:.3} {:.3} {:.3} rg {:.2} {:.2} {:.2} {:.2} re f Q",
                color.r as f32 / 255.0,
                color.g as f32 / 255.0,
                color.b as f32 / 255.0,
                x,
                y,
                w,
                h
            );
        }
    }

    // 2. Background & table rects
    for rect in &page.rects {
        match (rect.fill, rect.stroke) {
            (Some(f), Some((st, lw))) => {
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} rg {:.3} {:.3} {:.3} RG {:.2} w {:.2} {:.2} {:.2} {:.2} re B Q",
                    f.r as f32 / 255.0,
                    f.g as f32 / 255.0,
                    f.b as f32 / 255.0,
                    st.r as f32 / 255.0,
                    st.g as f32 / 255.0,
                    st.b as f32 / 255.0,
                    lw,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h
                );
            }
            (Some(f), None) => {
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} rg {:.2} {:.2} {:.2} {:.2} re f Q",
                    f.r as f32 / 255.0,
                    f.g as f32 / 255.0,
                    f.b as f32 / 255.0,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h
                );
            }
            (None, Some((st, lw))) => {
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} RG {:.2} w {:.2} {:.2} {:.2} {:.2} re S Q",
                    st.r as f32 / 255.0,
                    st.g as f32 / 255.0,
                    st.b as f32 / 255.0,
                    lw,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h
                );
            }
            (None, None) => {}
        }
    }

    // 3. Text runs
    for item in &page.text_items {
        let font_id = select_pdf_font(item.font_family, item.bold, item.italic);
        write_pdf_text_cmd(
            &mut s,
            font_id,
            item.font_size,
            item.x,
            item.y,
            item.color,
            &item.text,
        );
    }

    // 4. Foreground annotations (Stamps, Signatures, Sticky markers, Redactions)
    for annot in &page.annotations {
        match annot {
            PdfAnnotation::Stamp { x, y, label, color } => {
                let w = (label.chars().count() as f32 * 8.5 + 20.0).max(80.0);
                let h = 26.0;
                let _ = writeln!(
                    s,
                    "q {:.3} {:.3} {:.3} RG 1.8 w {:.2} {:.2} {:.2} {:.2} re S Q",
                    color.r as f32 / 255.0,
                    color.g as f32 / 255.0,
                    color.b as f32 / 255.0,
                    x,
                    y,
                    w,
                    h
                );
                write_pdf_text_cmd(&mut s, "F2", 12.0, *x + 9.0, *y + 8.0, *color, label);
            }
            PdfAnnotation::Signature {
                strokes,
                signer,
                color,
            } => {
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                for stroke in strokes {
                    if stroke.len() < 2 {
                        continue;
                    }
                    let _ = write!(
                        s,
                        "q {:.3} {:.3} {:.3} RG 1.9 w ",
                        color.r as f32 / 255.0,
                        color.g as f32 / 255.0,
                        color.b as f32 / 255.0
                    );
                    for (idx, &(px, py)) in stroke.iter().enumerate() {
                        min_x = min_x.min(px);
                        min_y = min_y.min(py);
                        if idx == 0 {
                            let _ = write!(s, "{px:.2} {py:.2} m ");
                        } else {
                            let _ = write!(s, "{px:.2} {py:.2} l ");
                        }
                    }
                    let _ = writeln!(s, "S Q");
                }
                if !signer.trim().is_empty() && min_x < f32::MAX {
                    write_pdf_text_cmd(
                        &mut s,
                        "F3",
                        8.5,
                        min_x,
                        (min_y - 11.0).max(8.0),
                        RgbColor::MUTED_GRAY,
                        &format!("Signed by {signer}"),
                    );
                }
            }
            PdfAnnotation::StickyNote { x, y, note, .. } => {
                let _ = writeln!(
                    s,
                    "q 0.99 0.88 0.28 rg 0.85 0.65 0.05 RG 0.9 w {:.2} {:.2} 18.0 18.0 re B Q",
                    x, y
                );
                let preview: String = note.chars().take(28).collect();
                write_pdf_text_cmd(
                    &mut s,
                    "F1",
                    8.0,
                    *x + 22.0,
                    *y + 5.0,
                    RgbColor::INK_SLATE,
                    &preview,
                );
            }
            PdfAnnotation::Redaction { x, y, w, h } => {
                let _ = writeln!(
                    s,
                    "q 0.06 0.09 0.16 rg {:.2} {:.2} {:.2} {:.2} re f Q",
                    x, y, w, h
                );
            }
            PdfAnnotation::Highlight { .. } => {}
        }
    }

    s
}
