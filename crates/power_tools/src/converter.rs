//! Bidirectional Smart Document Converter (`DOCX ↔ PDF`, `Markdown ↔ DOCX`, `MD → PDF`).

use doc_model::{
    Block, Document, FontFamily, ListKind, Paragraph, ParagraphStyle, RgbColor, SpanStyle,
    TableBlock, TableCell, TextSpan,
};
use docx_io::{load_docx_file, save_docx_file};
use pdf_create::{export_document_to_pdf_bytes, export_document_to_pdf_file, PdfCreateOptions};
use pdf_engine::{load_pdf_bytes, load_pdf_file, PdfDocument};
use std::path::Path;

pub fn word_to_pdf_document(doc: &Document) -> Result<PdfDocument, String> {
    let bytes = export_document_to_pdf_bytes(doc, &PdfCreateOptions::default());
    load_pdf_bytes(&bytes)
}

pub fn pdf_to_word(pdf: &PdfDocument) -> Document {
    let mut blocks: Vec<Block> = Vec::new();

    for (p_idx, page) in pdf.pages.iter().enumerate() {
        if p_idx > 0 {
            blocks.push(Block::PageBreak);
        }

        // Group items on the same baseline (within 4.5 pt) in top-to-bottom order
        let mut sorted = page.text_items.clone();
        sorted.sort_by(|a, b| {
            let dy = (b.y - a.y).abs();
            if dy > 4.5 {
                b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        let mut lines: Vec<Vec<pdf_engine::PdfTextItem>> = Vec::new();
        for item in sorted {
            // Skip running footer page counters
            if item.y < 44.0 && item.font_size <= 9.5 && item.text.starts_with("Page ") {
                continue;
            }
            if let Some(last_line) = lines.last_mut() {
                if (last_line[0].y - item.y).abs() <= 4.5 {
                    last_line.push(item);
                    continue;
                }
            }
            lines.push(vec![item]);
        }

        for line in lines {
            let max_size = line
                .iter()
                .map(|i| i.font_size)
                .fold(11.5f32, f32::max);
            let any_bold = line.iter().any(|i| i.bold);

            let style = if max_size >= 23.0 {
                ParagraphStyle::Title
            } else if max_size >= 18.0 {
                ParagraphStyle::Heading1
            } else if max_size >= 15.0 && any_bold {
                ParagraphStyle::Heading2
            } else if max_size >= 13.0 && any_bold {
                ParagraphStyle::Heading3
            } else {
                ParagraphStyle::Normal
            };

            let mut spans: Vec<TextSpan> = Vec::new();
            let mut list_kind = ListKind::None;

            for (idx, item) in line.into_iter().enumerate() {
                let mut txt = item.text;
                if idx == 0 {
                    let trimmed = txt.trim();
                    if trimmed == "•" {
                        list_kind = ListKind::Bullet;
                        continue;
                    } else if trimmed == "[x]" {
                        list_kind = ListKind::Checklist(true);
                        continue;
                    } else if trimmed == "[ ]" {
                        list_kind = ListKind::Checklist(false);
                        continue;
                    } else if let Some(num_s) = trimmed.strip_suffix('.') {
                        if let Ok(n) = num_s.parse::<u32>() {
                            list_kind = ListKind::Numbered(n);
                            continue;
                        }
                    }
                }
                if !spans.is_empty()
                    && !spans.last().unwrap().text.ends_with(' ')
                    && !txt.starts_with(' ')
                {
                    txt.insert(0, ' ');
                }
                spans.push(TextSpan {
                    text: txt,
                    style: SpanStyle {
                        font_family: item.font_family,
                        font_size_pt: item.font_size,
                        bold: item.bold,
                        italic: item.italic,
                        color: item.color,
                        ..SpanStyle::default()
                    },
                    revision: None,
                });
            }

            if !spans.is_empty() {
                let mut para = Paragraph {
                    style,
                    alignment: doc_model::Alignment::Left,
                    list_kind,
                    line_spacing: 1.35,
                    indent_level: 0,
                    spans,
                };
                para.normalize_spans();
                if let Some(Block::Paragraph(prev)) = blocks.last_mut() {
                    if prev.style == para.style
                        && prev.style != ParagraphStyle::Normal
                        && para.list_kind == ListKind::None
                    {
                        if let Some(last_span) = prev.spans.last_mut() {
                            if !last_span.text.ends_with(' ') {
                                last_span.text.push(' ');
                            }
                        }
                        for mut sp in para.spans {
                            if prev
                                .spans
                                .last()
                                .map(|ls| ls.text.ends_with(' '))
                                .unwrap_or(false)
                            {
                                sp.text = sp.text.trim_start().to_string();
                            }
                            prev.spans.push(sp);
                        }
                        prev.normalize_spans();
                        continue;
                    }
                }
                blocks.push(Block::Paragraph(para));
            }
        }
    }

    let mut doc = Document {
        title: pdf.title.clone(),
        author: pdf.author.clone(),
        blocks,
        ..Document::blank()
    };
    doc.ensure_non_empty();
    doc
}

pub fn markdown_to_word(md: &str, title: impl Into<String>) -> Document {
    let mut blocks = Vec::new();
    let mut table_rows: Vec<Vec<TableCell>> = Vec::new();

    let flush_table = |rows: &mut Vec<Vec<TableCell>>, blocks: &mut Vec<Block>| {
        if !rows.is_empty() {
            let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
            let mut normalized = Vec::new();
            for (r_idx, mut row) in rows.drain(..).enumerate() {
                while row.len() < cols {
                    row.push(TableCell::plain(""));
                }
                if r_idx == 0 {
                    for c in &mut row {
                        c.bold = true;
                        c.bg_color = Some(RgbColor::TABLE_HEADER_BG);
                    }
                }
                normalized.push(row);
            }
            blocks.push(Block::Table(TableBlock {
                cols,
                rows: normalized,
                border_color: RgbColor::TABLE_BORDER,
            }));
        }
    };

    for raw_line in md.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let cells: Vec<&str> = inner.split('|').map(|s| s.trim()).collect();
            let is_separator = cells
                .iter()
                .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '));
            if !is_separator {
                table_rows.push(cells.into_iter().map(TableCell::plain).collect());
            }
            continue;
        } else {
            flush_table(&mut table_rows, &mut blocks);
        }

        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            blocks.push(Block::HorizontalRule);
            continue;
        }
        if let Some(after_bang) = trimmed.strip_prefix("![") {
            if let Some((alt_cap, rest_paren)) = after_bang.split_once("](") {
                if let Some(img_path_str) = rest_paren.strip_suffix(')') {
                    let p_ref = std::path::Path::new(img_path_str.trim());
                    if let Ok(raw_img) = std::fs::read(p_ref) {
                        let name = p_ref
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Markdown Figure");
                        let mut img = doc_model::ImageBlock::new(name, 360.0, 220.0, raw_img);
                        img.caption = alt_cap.trim().to_string();
                        blocks.push(Block::Image(img));
                        continue;
                    }
                }
            }
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Title,
                h1.trim(),
            )));
        } else if let Some(h2) = trimmed.strip_prefix("## ") {
            blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                h2.trim(),
            )));
        } else if let Some(h3) = trimmed.strip_prefix("### ") {
            blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                h3.trim(),
            )));
        } else if let Some(h4) = trimmed.strip_prefix("#### ") {
            blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading3,
                h4.trim(),
            )));
        } else if let Some(q) = trimmed.strip_prefix("> ") {
            blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Quote,
                q.trim(),
            )));
        } else if let Some(done_item) = trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("- [X] ")) {
            let mut p = parse_inline_md(done_item);
            p.list_kind = ListKind::Checklist(true);
            blocks.push(Block::Paragraph(p));
        } else if let Some(todo_item) = trimmed.strip_prefix("- [ ] ") {
            let mut p = parse_inline_md(todo_item);
            p.list_kind = ListKind::Checklist(false);
            blocks.push(Block::Paragraph(p));
        } else if let Some(b) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let mut p = parse_inline_md(b);
            p.list_kind = ListKind::Bullet;
            blocks.push(Block::Paragraph(p));
        } else {
            blocks.push(Block::Paragraph(parse_inline_md(trimmed)));
        }
    }
    flush_table(&mut table_rows, &mut blocks);

    let mut doc = Document {
        title: title.into(),
        author: "Nour Yahyaoui".into(),
        blocks,
        ..Document::blank()
    };
    doc.ensure_non_empty();
    doc
}

fn parse_inline_md(text: &str) -> Paragraph {
    let mut spans = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            if let Some((label, after_bracket)) = after.split_once("](") {
                if let Some(close_paren) = after_bracket.find(')') {
                    let url = after_bracket[..close_paren].trim();
                    if !label.is_empty() && !url.is_empty() {
                        spans.push(TextSpan::styled(
                            label,
                            SpanStyle {
                                underline: true,
                                color: RgbColor::COBALT,
                                hyperlink: Some(url.to_string()),
                                ..SpanStyle::default()
                            },
                        ));
                        rest = &after_bracket[close_paren + 1..];
                        continue;
                    }
                }
            }
        }
        if let Some(after) = rest.strip_prefix("**") {
            if let Some(end) = after.find("**") {
                spans.push(TextSpan::styled(
                    &after[..end],
                    SpanStyle {
                        bold: true,
                        ..SpanStyle::default()
                    },
                ));
                rest = &after[end + 2..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('`') {
            if let Some(end) = after.find('`') {
                spans.push(TextSpan::styled(
                    &after[..end],
                    SpanStyle {
                        font_family: FontFamily::Mono,
                        highlight: Some(RgbColor::TABLE_HEADER_BG),
                        ..SpanStyle::default()
                    },
                ));
                rest = &after[end + 1..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('*') {
            if let Some(end) = after.find('*') {
                spans.push(TextSpan::styled(
                    &after[..end],
                    SpanStyle {
                        italic: true,
                        ..SpanStyle::default()
                    },
                ));
                rest = &after[end + 1..];
                continue;
            }
        }
        let next_special = rest[1..]
            .find(['*', '`', '['])
            .map(|idx| idx + 1)
            .unwrap_or(rest.len());
        spans.push(TextSpan::plain(&rest[..next_special]));
        rest = &rest[next_special..];
    }

    let mut p = Paragraph::new(ParagraphStyle::Normal, "");
    p.spans = spans;
    p.normalize_spans();
    p
}

pub fn word_to_markdown(doc: &Document) -> String {
    let mut md = String::new();
    for block in &doc.blocks {
        match block {
            Block::Paragraph(p) => {
                match p.style {
                    ParagraphStyle::Title => md.push_str("# "),
                    ParagraphStyle::Subtitle => md.push_str("*"),
                    ParagraphStyle::Heading1 => md.push_str("## "),
                    ParagraphStyle::Heading2 => md.push_str("### "),
                    ParagraphStyle::Heading3 => md.push_str("#### "),
                    ParagraphStyle::Quote => md.push_str("> "),
                    ParagraphStyle::Normal => match p.list_kind {
                        ListKind::Bullet => md.push_str("- "),
                        ListKind::Numbered(n) => md.push_str(&format!("{n}. ")),
                        ListKind::Checklist(true) => md.push_str("- [x] "),
                        ListKind::Checklist(false) => md.push_str("- [ ] "),
                        ListKind::None => {}
                    },
                }
                for span in &p.spans {
                    if span.style.hidden {
                        continue;
                    }
                    if let Some(url) = &span.style.hyperlink {
                        md.push_str(&format!("[{}]({})", span.text, url));
                    } else if span.style.bold && p.style == ParagraphStyle::Normal {
                        md.push_str(&format!("**{}**", span.text));
                    } else if span.style.italic
                        && !matches!(p.style, ParagraphStyle::Subtitle | ParagraphStyle::Quote)
                    {
                        md.push_str(&format!("*{}*", span.text));
                    } else {
                        md.push_str(&span.text);
                    }
                }
                if p.style == ParagraphStyle::Subtitle {
                    md.push('*');
                }
                md.push_str("\n\n");
            }
            Block::Table(t) => {
                for (r_idx, row) in t.rows.iter().enumerate() {
                    md.push_str("| ");
                    let cells: Vec<&str> = row.iter().map(|c| c.text.as_str()).collect();
                    md.push_str(&cells.join(" | "));
                    md.push_str(" |\n");
                    if r_idx == 0 {
                        md.push_str("| ");
                        let seps: Vec<&str> = (0..t.cols).map(|_| "---").collect();
                        md.push_str(&seps.join(" | "));
                        md.push_str(" |\n");
                    }
                }
                md.push('\n');
            }
            Block::HorizontalRule => md.push_str("---\n\n"),
            Block::PageBreak | Block::SectionBreak(_) => md.push_str("\n---\n\n"),
            Block::Image(img) => md.push_str(&format!("![{}]({})\n\n", img.caption, img.name)),
        }
    }
    if !doc.footnotes.is_empty() || !doc.endnotes.is_empty() {
        md.push_str("---\n\n");
        for n in &doc.footnotes {
            md.push_str(&format!("[^{}]: {}\n", n.id, n.text));
        }
        for n in &doc.endnotes {
            md.push_str(&format!("[^e{}]: {}\n", n.id, n.text));
        }
    }
    md
}

/// Exports `doc` to a self-contained, responsive HTML5 document with Daylight & Native Dark Mode
/// CSS tokens, semantic headings, tables, Track Changes `<ins>`/`<del>` markup, footnotes,
/// and inline Base64 images.
pub fn word_to_html(doc: &Document, dark_mode_default: bool) -> String {
    let theme_attr = if dark_mode_default { "dark" } else { "light" };
    let mut body = String::with_capacity(8192);

    if doc.classification != doc_model::SecurityClassification::Unclassified {
        body.push_str(&format!(
            "<div class=\"rp-banner\">{}</div>\n",
            escape_html(doc.classification.label())
        ));
    }
    if !doc.page_setup.header_text.trim().is_empty() {
        body.push_str(&format!(
            "<header class=\"rp-running-hdr\">{}</header>\n",
            escape_html(&doc.page_setup.header_text)
        ));
    }

    for block in &doc.blocks {
        match block {
            Block::Paragraph(p) => {
                let tag = match p.style {
                    ParagraphStyle::Title => "h1",
                    ParagraphStyle::Subtitle => "p",
                    ParagraphStyle::Heading1 => "h2",
                    ParagraphStyle::Heading2 => "h3",
                    ParagraphStyle::Heading3 => "h4",
                    ParagraphStyle::Quote => "blockquote",
                    ParagraphStyle::Normal => "p",
                };
                let cls = if p.style == ParagraphStyle::Subtitle {
                    " class=\"rp-subtitle\""
                } else {
                    ""
                };
                body.push_str(&format!("<{tag}{cls}>"));
                match p.list_kind {
                    ListKind::Bullet => body.push_str("• "),
                    ListKind::Numbered(n) => body.push_str(&format!("{n}. ")),
                    ListKind::Checklist(true) => body.push_str("☑ "),
                    ListKind::Checklist(false) => body.push_str("☐ "),
                    ListKind::None => {}
                }
                for sp in &p.spans {
                    if sp.style.hidden {
                        continue;
                    }
                    let mut chunk = escape_html(&sp.text);
                    if sp.style.bold {
                        chunk = format!("<strong>{chunk}</strong>");
                    }
                    if sp.style.italic {
                        chunk = format!("<em>{chunk}</em>");
                    }
                    if sp.style.underline && sp.style.hyperlink.is_none() {
                        chunk = format!("<u>{chunk}</u>");
                    }
                    if sp.style.strikethrough {
                        chunk = format!("<s>{chunk}</s>");
                    }
                    if sp.style.superscript {
                        chunk = format!("<sup>{chunk}</sup>");
                    } else if sp.style.subscript {
                        chunk = format!("<sub>{chunk}</sub>");
                    }
                    if let Some(url) = &sp.style.hyperlink {
                        chunk = format!(
                            "<a href=\"{}\" rel=\"noopener noreferrer\">{}</a>",
                            escape_html(url),
                            chunk
                        );
                    }
                    if let Some(rev) = &sp.revision {
                        chunk = match rev.kind {
                            doc_model::RevisionKind::Insertion => {
                                format!("<ins title=\"{}\">{chunk}</ins>", escape_html(&rev.author))
                            }
                            doc_model::RevisionKind::Deletion => {
                                format!("<del title=\"{}\">{chunk}</del>", escape_html(&rev.author))
                            }
                        };
                    }
                    body.push_str(&chunk);
                }
                body.push_str(&format!("</{tag}>\n"));
            }
            Block::Table(t) => {
                body.push_str("<div class=\"rp-table-wrap\"><table>\n");
                for (r_idx, row) in t.rows.iter().enumerate() {
                    body.push_str("  <tr>");
                    let cell_tag = if r_idx == 0 { "th" } else { "td" };
                    for cell in row {
                        body.push_str(&format!(
                            "<{cell_tag}>{}</{cell_tag}>",
                            escape_html(&cell.text)
                        ));
                    }
                    body.push_str("</tr>\n");
                }
                body.push_str("</table></div>\n");
            }
            Block::Image(img) => {
                body.push_str("<figure class=\"rp-figure\">");
                if !img.png_bytes.is_empty() {
                    let mime = match img.media_extension() {
                        "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "bmp" => "image/bmp",
                        _ => "image/png",
                    };
                    let b64 = encode_base64(img.png_bytes.as_slice());
                    body.push_str(&format!(
                        "<img src=\"data:{mime};base64,{b64}\" alt=\"{}\" style=\"max-width:{}px;\"/>",
                        escape_html(&img.name),
                        (img.width_pt * 1.33).round() as u32
                    ));
                }
                if !img.caption.trim().is_empty() {
                    body.push_str(&format!(
                        "<figcaption>{}</figcaption>",
                        escape_html(&img.caption)
                    ));
                }
                body.push_str("</figure>\n");
            }
            Block::HorizontalRule => body.push_str("<hr/>\n"),
            Block::PageBreak | Block::SectionBreak(_) => {
                body.push_str("<hr class=\"rp-page-break\"/>\n")
            }
        }
    }

    if !doc.footnotes.is_empty() || !doc.endnotes.is_empty() {
        body.push_str("<section class=\"rp-notes\"><hr/><ol>\n");
        for n in &doc.footnotes {
            body.push_str(&format!(
                "<li id=\"fn-{}\">[{}] {}</li>\n",
                n.id,
                n.id,
                escape_html(&n.text)
            ));
        }
        for n in &doc.endnotes {
            body.push_str(&format!(
                "<li id=\"en-{}\">[e{}] {}</li>\n",
                n.id,
                n.id,
                escape_html(&n.text)
            ));
        }
        body.push_str("</ol></section>\n");
    }

    if !doc.page_setup.footer_text.trim().is_empty() {
        body.push_str(&format!(
            "<footer class=\"rp-running-ftr\">{}</footer>\n",
            escape_html(&doc.page_setup.footer_text)
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="{theme_attr}">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
  <meta name="generator" content="RustPad v3.0.0"/>
  <title>{}</title>
  <style>
    :root {{ --desk:#eef2f7; --sheet:#ffffff; --ink:#0f172a; --muted:#64748b; --border:#d0d8e4; --accent:#2563eb; --th:#f1f5f9; }}
    [data-theme="dark"] {{ --desk:#0b0f17; --sheet:#161e2e; --ink:#e2e8f0; --muted:#94a3b8; --border:#273449; --accent:#60a5fa; --th:#1e293b; }}
    @media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --desk:#0b0f17; --sheet:#161e2e; --ink:#e2e8f0; --muted:#94a3b8; --border:#273449; --accent:#60a5fa; --th:#1e293b; }} }}
    body {{ margin:0; padding:32px 16px; background:var(--desk); color:var(--ink); font:16px/1.65 system-ui,-apple-system,Segoe UI,sans-serif; }}
    main {{ max-width:816px; margin:0 auto; background:var(--sheet); border:1px solid var(--border); border-radius:10px; padding:56px 64px; box-shadow:0 8px 30px rgba(0,0,0,.12); }}
    h1,h2,h3,h4 {{ line-height:1.25; margin:1.2em 0 .45em; }}
    .rp-subtitle {{ color:var(--muted); font-style:italic; margin-top:-.4em; }}
    blockquote {{ border-left:3px solid var(--accent); margin:1em 0; padding:.25em 1em; color:var(--muted); }}
    a {{ color:var(--accent); }}
    ins {{ color:#10b981; text-decoration:underline; }}
    del {{ color:#ef4444; text-decoration:line-through; }}
    .rp-table-wrap {{ overflow-x:auto; margin:1.1em 0; }}
    table {{ width:100%; border-collapse:collapse; }}
    th,td {{ border:1px solid var(--border); padding:8px 12px; text-align:left; }}
    th {{ background:var(--th); }}
    .rp-figure {{ text-align:center; margin:1.4em 0; }}
    .rp-figure img {{ height:auto; border-radius:6px; max-width:100%; }}
    .rp-figure figcaption,.rp-running-hdr,.rp-running-ftr,.rp-notes {{ color:var(--muted); font-size:.88rem; }}
    .rp-banner {{ display:inline-block; font-size:.75rem; font-weight:700; padding:3px 10px; border-radius:999px; background:var(--th); margin-bottom:14px; }}
    .rp-page-break {{ border:none; border-top:2px dashed var(--border); margin:2.2em 0; }}
    @media (max-width:640px) {{ main {{ padding:24px 18px; }} }}
  </style>
</head>
<body>
  <main>
{body}  </main>
</body>
</html>
"#,
        escape_html(&doc.title)
    )
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn encode_base64(bytes: &[u8]) -> String {
    const LUT: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(LUT[((n >> 18) & 63) as usize] as char);
        out.push(LUT[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            LUT[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            LUT[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn convert_file(input: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<String, String> {
    let in_p = input.as_ref();
    let out_p = output.as_ref();
    let in_ext = in_p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let out_ext = out_p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let word_doc: Document = match in_ext.as_str() {
        "docx" => load_docx_file(in_p).map_err(|e| e.0)?,
        "pdf" => {
            let pdf = load_pdf_file(in_p)?;
            pdf_to_word(&pdf)
        }
        "md" | "markdown" | "txt" => {
            let raw = std::fs::read_to_string(in_p).map_err(|e| e.to_string())?;
            let title = in_p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Converted Document");
            markdown_to_word(&raw, title)
        }
        other => return Err(format!("Unsupported input extension '.{other}'")),
    };

    match out_ext.as_str() {
        "docx" => {
            save_docx_file(out_p, &word_doc).map_err(|e| e.0)?;
        }
        "pdf" => {
            export_document_to_pdf_file(out_p, &word_doc, &PdfCreateOptions::default())?;
        }
        "md" | "markdown" => {
            let md = word_to_markdown(&word_doc);
            std::fs::write(out_p, md).map_err(|e| e.to_string())?;
        }
        "html" | "htm" => {
            let dark = rustpad_render::load_saved_or_system_theme().is_dark_chrome();
            let html = word_to_html(&word_doc, dark);
            std::fs::write(out_p, html).map_err(|e| e.to_string())?;
        }
        "txt" => {
            std::fs::write(out_p, word_doc.to_plain_text()).map_err(|e| e.to_string())?;
        }
        other => return Err(format!("Unsupported output extension '.{other}'")),
    }

    Ok(format!(
        "Converted '{}' -> '{}'",
        in_p.display(),
        out_p.display()
    ))
}
