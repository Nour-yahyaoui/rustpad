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
                        underline: false,
                        strikethrough: false,
                        color: item.color,
                        highlight: None,
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
        classification: doc_model::SecurityClassification::Unclassified,
        track_changes: false,
        page_setup: doc_model::PageSetup::default(),
        blocks,
        comments: Vec::new(),
        passthrough_parts: std::collections::BTreeMap::new(),
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
        classification: doc_model::SecurityClassification::Unclassified,
        track_changes: false,
        page_setup: doc_model::PageSetup::default(),
        blocks,
        comments: Vec::new(),
        passthrough_parts: std::collections::BTreeMap::new(),
    };
    doc.ensure_non_empty();
    doc
}

fn parse_inline_md(text: &str) -> Paragraph {
    let mut spans = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
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
            .find(['*', '`'])
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
                    if span.style.bold && p.style == ParagraphStyle::Normal {
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
            Block::PageBreak => md.push_str("\n---\n\n"),
            Block::Image(img) => md.push_str(&format!("![{}]({})\n\n", img.caption, img.name)),
        }
    }
    md
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
