//! ECMA-376 OpenXML (`.docx`) Reader, Track Changes (`<w:ins>`/`<w:del>`), Threaded Comments
//! (`word/comments.xml`), Security Classification, and Lossless Archive Passthrough Reconstructor.

use crate::xml_lite::{parse_xml, XmlChild, XmlNode};
use crate::DocxError;
use doc_model::{
    Alignment, Block, DocComment, Document, FontFamily, ImageBlock, ListKind, PageSetup, PaperSize,
    Paragraph, ParagraphStyle, RevisionKind, RevisionMeta, RgbColor, SecurityClassification,
    SpanStyle, TableBlock, TableCell, TextSpan,
};
use rustpad_zip::ZipArchive;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn load_docx_file(path: impl AsRef<Path>) -> Result<Document, DocxError> {
    let bytes = std::fs::read(&path).map_err(|e| DocxError(e.to_string()))?;
    let mut doc = load_docx_bytes(&bytes)?;
    if doc.title == "Untitled Document" {
        if let Some(stem) = path.as_ref().file_stem().and_then(|s| s.to_str()) {
            doc.title = stem.to_string();
        }
    }
    Ok(doc)
}

pub fn load_docx_bytes(bytes: &[u8]) -> Result<Document, DocxError> {
    let archive = ZipArchive::from_bytes(bytes).map_err(|e| DocxError(e.0))?;
    let doc_xml = archive
        .get_str("word/document.xml")
        .ok_or_else(|| DocxError("Missing word/document.xml inside .docx archive".into()))?;

    let root = parse_xml(&doc_xml).ok_or_else(|| DocxError("Malformed word/document.xml".into()))?;
    let body = root
        .child("body")
        .ok_or_else(|| DocxError("Missing <w:body> in word/document.xml".into()))?;

    // Relationships map (rId -> Target)
    let mut rels = BTreeMap::new();
    if let Some(rels_xml) = archive.get_str("word/_rels/document.xml.rels") {
        if let Some(rels_root) = parse_xml(&rels_xml) {
            for rel in rels_root.children_named("Relationship") {
                if let (Some(id), Some(target)) = (rel.attr("Id"), rel.attr("Target")) {
                    rels.insert(id.to_string(), target.to_string());
                }
            }
        }
    }

    let mut title = "Untitled Document".to_string();
    let mut author = "Nour Yahyaoui".to_string();
    let mut classification = SecurityClassification::Unclassified;
    if let Some(core_xml) = archive.get_str("docProps/core.xml") {
        if let Some(core_root) = parse_xml(&core_xml) {
            if let Some(t) = core_root.child("title") {
                let txt = t.text_content().trim().to_string();
                if !txt.is_empty() {
                    title = txt;
                }
            }
            if let Some(c) = core_root.child("creator") {
                let txt = c.text_content().trim().to_string();
                if !txt.is_empty() {
                    author = txt;
                }
            }
            if let Some(cat) = core_root.child("category") {
                classification = SecurityClassification::from_slug(&cat.text_content());
            }
        }
    }

    let mut track_changes = false;
    if let Some(settings_xml) = archive.get_str("word/settings.xml") {
        if settings_xml.contains("trackRevisions") {
            track_changes = true;
        }
    }

    let mut page_setup = PageSetup::default();
    if let Some(hdr_xml) = archive.get_str("word/header1.xml") {
        if let Some(hdr_root) = parse_xml(&hdr_xml) {
            page_setup.header_text = hdr_root.text_content().trim().to_string();
        }
    }
    if let Some(ftr_xml) = archive.get_str("word/footer1.xml") {
        if let Some(ftr_root) = parse_xml(&ftr_xml) {
            let raw = ftr_root.text_content();
            let cleaned = raw
                .split("·   Page")
                .next()
                .unwrap_or(&raw)
                .trim_end_matches("Page")
                .trim()
                .to_string();
            page_setup.footer_text = cleaned;
        }
    }

    let mut blocks = Vec::new();
    let mut num_counter = 0u32;
    let mut consumed_media_paths = BTreeSet::new();
    // Map comment_id -> block_index from `<w:commentRangeStart w:id="..."/>` or `<w:commentReference w:id="..."/>`
    let mut comment_anchors: BTreeMap<u32, usize> = BTreeMap::new();

    for child in &body.children {
        let XmlChild::Element(el) = child else {
            continue;
        };
        match el.local_name() {
            "p" => {
                parse_paragraph_or_special(
                    el,
                    &archive,
                    &rels,
                    &mut num_counter,
                    &mut blocks,
                    &mut consumed_media_paths,
                    &mut comment_anchors,
                );
            }
            "tbl" => {
                if let Some(tbl) = parse_table(el) {
                    blocks.push(Block::Table(tbl));
                }
            }
            "sectPr" => {
                parse_sect_pr(el, &mut page_setup);
            }
            _ => {}
        }
    }

    // Parse `word/comments.xml` if present
    let mut comments = Vec::new();
    if let Some(comments_xml) = archive.get_str("word/comments.xml") {
        if let Some(c_root) = parse_xml(&comments_xml) {
            for c_el in c_root.children_named("comment") {
                let id = c_el
                    .attr("id")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or((comments.len() + 1) as u32);
                let c_author = c_el.attr("author").unwrap_or("Reviewer").to_string();
                let date_iso = c_el.attr("date").unwrap_or("").to_string();
                let resolved = matches!(c_el.attr("done"), Some("1") | Some("true"));
                let text = c_el.text_content().trim().to_string();
                let block_index = comment_anchors.get(&id).copied().unwrap_or(0);
                if !text.is_empty() {
                    comments.push(DocComment {
                        id,
                        block_index: block_index.min(blocks.len().saturating_sub(1)),
                        author: c_author,
                        date_iso,
                        text,
                        resolved,
                    });
                }
            }
        }
    }

    // Preserve unmodeled enterprise archive entries (`customXml/*`, `word/theme/*`, `docProps/app.xml`, etc.)
    let managed_parts: [&str; 11] = [
        "[Content_Types].xml",
        "_rels/.rels",
        "docProps/core.xml",
        "word/_rels/document.xml.rels",
        "word/styles.xml",
        "word/numbering.xml",
        "word/settings.xml",
        "word/header1.xml",
        "word/footer1.xml",
        "word/document.xml",
        "word/comments.xml",
    ];
    let mut passthrough_parts = BTreeMap::new();
    for (entry_name, raw_bytes) in archive.entries() {
        if managed_parts.contains(&entry_name.as_str()) {
            continue;
        }
        if consumed_media_paths.contains(entry_name) {
            continue;
        }
        passthrough_parts.insert(entry_name.clone(), raw_bytes.clone());
    }

    // Enable track_changes automatically if loaded spans carry revisions
    if !track_changes {
        track_changes = blocks.iter().any(|b| match b {
            Block::Paragraph(p) => p.has_revisions(),
            _ => false,
        });
    }

    let mut doc = Document {
        title,
        author,
        classification,
        track_changes,
        page_setup,
        blocks,
        comments,
        passthrough_parts,
    };
    doc.ensure_non_empty();
    Ok(doc)
}

fn parse_sect_pr(sect: &XmlNode, setup: &mut PageSetup) {
    if let Some(pg_sz) = sect.child("pgSz") {
        if let (Some(w_s), Some(h_s)) = (pg_sz.attr("w"), pg_sz.attr("h")) {
            if let (Ok(w), Ok(h)) = (w_s.parse::<f32>(), h_s.parse::<f32>()) {
                let w_pt = w / 20.0;
                let h_pt = h / 20.0;
                setup.landscape = w_pt > h_pt;
                let long_edge = w_pt.max(h_pt);
                setup.paper_size = if long_edge > 920.0 {
                    PaperSize::Legal
                } else if (long_edge - 792.0).abs() < 20.0 {
                    PaperSize::Letter
                } else {
                    PaperSize::A4
                };
            }
        }
    }
    if let Some(mar) = sect.child("pgMar") {
        if let Some(v) = mar.attr("top").and_then(|s| s.parse::<f32>().ok()) {
            setup.margin_top_pt = v / 20.0;
        }
        if let Some(v) = mar.attr("bottom").and_then(|s| s.parse::<f32>().ok()) {
            setup.margin_bottom_pt = v / 20.0;
        }
        if let Some(v) = mar.attr("left").and_then(|s| s.parse::<f32>().ok()) {
            setup.margin_left_pt = v / 20.0;
        }
        if let Some(v) = mar.attr("right").and_then(|s| s.parse::<f32>().ok()) {
            setup.margin_right_pt = v / 20.0;
        }
    }
}

fn parse_paragraph_or_special(
    p_node: &XmlNode,
    archive: &ZipArchive,
    rels: &BTreeMap<String, String>,
    num_counter: &mut u32,
    blocks: &mut Vec<Block>,
    consumed_media_paths: &mut BTreeSet<String>,
    comment_anchors: &mut BTreeMap<u32, usize>,
) {
    if let Some(p_pr) = p_node.child("pPr") {
        if p_pr.child("pBdr").is_some() && p_node.text_content().trim().is_empty() {
            blocks.push(Block::HorizontalRule);
            return;
        }
    }

    for r in p_node.children_named("r") {
        for br in r.children_named("br") {
            if br.attr("type") == Some("page") {
                blocks.push(Block::PageBreak);
                return;
            }
        }
        if let Some(drawing) = r.child("drawing") {
            if let Some((img, media_path)) = extract_drawing_image(drawing, archive, rels) {
                consumed_media_paths.insert(media_path);
                blocks.push(Block::Image(img));
                return;
            }
        }
    }

    let mut style = ParagraphStyle::Normal;
    let mut alignment = Alignment::Left;
    let mut list_kind = ListKind::None;
    let mut line_spacing = 1.35f32;
    let mut indent_level = 0u8;

    if let Some(p_pr) = p_node.child("pPr") {
        if let Some(p_style) = p_pr.child("pStyle").and_then(|n| n.attr("val")) {
            let lower = p_style.to_ascii_lowercase();
            style = if lower == "title" {
                ParagraphStyle::Title
            } else if lower == "subtitle" {
                ParagraphStyle::Subtitle
            } else if lower.contains("heading1") || lower == "heading 1" || lower == "1" {
                ParagraphStyle::Heading1
            } else if lower.contains("heading2") || lower == "heading 2" || lower == "2" {
                ParagraphStyle::Heading2
            } else if lower.contains("heading3") || lower == "heading 3" || lower == "3" {
                ParagraphStyle::Heading3
            } else if lower.contains("quote") {
                ParagraphStyle::Quote
            } else {
                ParagraphStyle::Normal
            };
        }
        if let Some(jc) = p_pr.child("jc").and_then(|n| n.attr("val")) {
            alignment = match jc {
                "center" => Alignment::Center,
                "right" | "end" => Alignment::Right,
                "both" | "distribute" => Alignment::Justify,
                _ => Alignment::Left,
            };
        }
        if let Some(num_pr) = p_pr.child("numPr") {
            let num_id = num_pr
                .child("numId")
                .and_then(|n| n.attr("val"))
                .unwrap_or("1");
            if num_id == "2" {
                *num_counter += 1;
                list_kind = ListKind::Numbered(*num_counter);
            } else {
                *num_counter = 0;
                list_kind = ListKind::Bullet;
            }
        } else {
            *num_counter = 0;
        }
        if let Some(ind) = p_pr
            .child("ind")
            .and_then(|n| n.attr("left"))
            .and_then(|v| v.parse::<u32>().ok())
        {
            indent_level = (ind / 360).min(6) as u8;
        }
        if let Some(line) = p_pr
            .child("spacing")
            .and_then(|n| n.attr("line"))
            .and_then(|v| v.parse::<f32>().ok())
        {
            if line > 100.0 {
                line_spacing = (line / 240.0).clamp(1.0, 3.0);
            }
        }
    } else {
        *num_counter = 0;
    }

    let target_block_idx = blocks.len();
    let mut spans = Vec::new();
    for child in &p_node.children {
        let XmlChild::Element(el) = child else {
            continue;
        };
        match el.local_name() {
            "r" => {
                for cref in el.children_named("commentReference") {
                    if let Some(cid) = cref.attr("id").and_then(|v| v.parse::<u32>().ok()) {
                        comment_anchors.entry(cid).or_insert(target_block_idx);
                    }
                }
                if let Some(span) = parse_run(el, style, None) {
                    spans.push(span);
                }
            }
            "ins" => {
                let rev = RevisionMeta {
                    id: el.attr("id").and_then(|v| v.parse().ok()).unwrap_or(1),
                    kind: RevisionKind::Insertion,
                    author: el.attr("author").unwrap_or("Reviewer").to_string(),
                    date_iso: el.attr("date").unwrap_or("").to_string(),
                };
                for r in el.children_named("r") {
                    if let Some(span) = parse_run(r, style, Some(rev.clone())) {
                        spans.push(span);
                    }
                }
            }
            "del" => {
                let rev = RevisionMeta {
                    id: el.attr("id").and_then(|v| v.parse().ok()).unwrap_or(1),
                    kind: RevisionKind::Deletion,
                    author: el.attr("author").unwrap_or("Reviewer").to_string(),
                    date_iso: el.attr("date").unwrap_or("").to_string(),
                };
                for r in el.children_named("r") {
                    if let Some(span) = parse_run(r, style, Some(rev.clone())) {
                        spans.push(span);
                    }
                }
            }
            "commentRangeStart" => {
                if let Some(cid) = el.attr("id").and_then(|v| v.parse::<u32>().ok()) {
                    comment_anchors.entry(cid).or_insert(target_block_idx);
                }
            }
            "hyperlink" => {
                for r in el.children_named("r") {
                    if let Some(mut span) = parse_run(r, style, None) {
                        span.style.underline = true;
                        span.style.color = RgbColor::COBALT;
                        spans.push(span);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(first) = spans.first_mut() {
        if let Some(rest) = first.text.strip_prefix("☑ ") {
            list_kind = ListKind::Checklist(true);
            first.text = rest.to_string();
        } else if let Some(rest) = first.text.strip_prefix("☐ ") {
            list_kind = ListKind::Checklist(false);
            first.text = rest.to_string();
        }
    }

    let mut para = Paragraph {
        style,
        alignment,
        list_kind,
        line_spacing,
        indent_level,
        spans,
    };
    para.normalize_spans();
    blocks.push(Block::Paragraph(para));
}

fn parse_run(
    r_node: &XmlNode,
    para_style: ParagraphStyle,
    revision: Option<RevisionMeta>,
) -> Option<TextSpan> {
    let mut text = String::new();
    for child in &r_node.children {
        if let XmlChild::Element(el) = child {
            match el.local_name() {
                "t" | "delText" => text.push_str(&el.text_content()),
                "tab" => text.push('\t'),
                "br" if el.attr("type") != Some("page") => text.push('\n'),
                _ => {}
            }
        }
    }
    if text.is_empty() {
        return None;
    }

    let mut style = SpanStyle {
        font_size_pt: para_style.default_font_size(),
        bold: para_style.is_bold_by_default(),
        italic: para_style == ParagraphStyle::Quote,
        color: if matches!(para_style, ParagraphStyle::Subtitle | ParagraphStyle::Quote) {
            RgbColor::MUTED_GRAY
        } else {
            RgbColor::INK_SLATE
        },
        ..SpanStyle::default()
    };

    if let Some(r_pr) = r_node.child("rPr") {
        if let Some(fonts) = r_pr.child("rFonts") {
            if let Some(ascii) = fonts.attr("ascii").or_else(|| fonts.attr("hAnsi")) {
                style.font_family = FontFamily::from_docx_name(ascii);
            }
        }
        if let Some(b) = r_pr.child("b") {
            style.bold = b.attr("val") != Some("0") && b.attr("val") != Some("false");
        }
        if let Some(i) = r_pr.child("i") {
            style.italic = i.attr("val") != Some("0") && i.attr("val") != Some("false");
        }
        if let Some(u) = r_pr.child("u") {
            style.underline = u.attr("val") != Some("none");
        }
        if r_pr.child("strike").is_some() {
            style.strikethrough = true;
        }
        if let Some(sz) = r_pr
            .child("sz")
            .and_then(|n| n.attr("val"))
            .and_then(|v| v.parse::<f32>().ok())
        {
            style.font_size_pt = (sz / 2.0).clamp(6.0, 96.0);
        }
        if let Some(col) = r_pr
            .child("color")
            .and_then(|n| n.attr("val"))
            .and_then(RgbColor::from_hex)
        {
            style.color = col;
        }
        if let Some(fill) = r_pr
            .child("shd")
            .and_then(|n| n.attr("fill"))
            .and_then(RgbColor::from_hex)
        {
            style.highlight = Some(fill);
        } else if let Some(hl_name) = r_pr.child("highlight").and_then(|n| n.attr("val")) {
            style.highlight = match hl_name {
                "yellow" => Some(RgbColor::AMBER_HIGHLIGHT),
                "green" => Some(RgbColor::MINT_HIGHLIGHT),
                "cyan" | "blue" => Some(RgbColor::SKY_HIGHLIGHT),
                "magenta" | "red" => Some(RgbColor::ROSE_HIGHLIGHT),
                _ => None,
            };
        }
    }

    Some(TextSpan {
        text,
        style,
        revision,
    })
}

fn parse_table(tbl_node: &XmlNode) -> Option<TableBlock> {
    let mut rows = Vec::new();
    let mut max_cols = 0usize;

    for tr in tbl_node.children_named("tr") {
        let mut row = Vec::new();
        for tc in tr.children_named("tc") {
            let mut bg_color = None;
            if let Some(tc_pr) = tc.child("tcPr") {
                if let Some(fill) = tc_pr
                    .child("shd")
                    .and_then(|n| n.attr("fill"))
                    .and_then(RgbColor::from_hex)
                {
                    bg_color = Some(fill);
                }
            }
            let mut cell_text = String::new();
            let mut bold = false;
            let mut alignment = Alignment::Left;
            for (p_idx, p) in tc.children_named("p").enumerate() {
                if p_idx > 0 {
                    cell_text.push('\n');
                }
                if let Some(jc) = p
                    .child("pPr")
                    .and_then(|pr| pr.child("jc"))
                    .and_then(|n| n.attr("val"))
                {
                    alignment = match jc {
                        "center" => Alignment::Center,
                        "right" => Alignment::Right,
                        _ => Alignment::Left,
                    };
                }
                for r in p.children_named("r") {
                    if r.child("rPr").and_then(|pr| pr.child("b")).is_some() {
                        bold = true;
                    }
                    cell_text.push_str(&r.text_content());
                }
            }
            row.push(TableCell {
                text: cell_text,
                bold,
                bg_color,
                alignment,
            });
        }
        if !row.is_empty() {
            max_cols = max_cols.max(row.len());
            rows.push(row);
        }
    }

    if rows.is_empty() || max_cols == 0 {
        return None;
    }
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(TableCell::plain(""));
        }
    }

    Some(TableBlock {
        cols: max_cols,
        rows,
        border_color: RgbColor::TABLE_BORDER,
    })
}

fn extract_drawing_image(
    drawing: &XmlNode,
    archive: &ZipArchive,
    rels: &BTreeMap<String, String>,
) -> Option<(ImageBlock, String)> {
    let inline = drawing.child("inline").or_else(|| drawing.child("anchor"))?;
    let mut width_pt = 240.0f32;
    let mut height_pt = 160.0f32;
    if let Some(ext) = inline.child("extent") {
        if let Some(cx) = ext.attr("cx").and_then(|v| v.parse::<f64>().ok()) {
            width_pt = (cx / 12700.0) as f32;
        }
        if let Some(cy) = ext.attr("cy").and_then(|v| v.parse::<f64>().ok()) {
            height_pt = (cy / 12700.0) as f32;
        }
    }
    let name = inline
        .child("docPr")
        .and_then(|n| n.attr("name"))
        .unwrap_or("Embedded Image")
        .to_string();

    let blip_id = find_attr_recursive(inline, "blip", "embed")?;
    let target = rels.get(&blip_id)?;
    let full_path = if target.starts_with("word/") {
        target.clone()
    } else {
        format!("word/{}", target.trim_start_matches('/'))
    };
    let png_bytes = archive.get(&full_path)?.to_vec();

    Some((
        ImageBlock {
            name,
            width_pt,
            height_pt,
            png_bytes,
            rgba_preview: None,
            caption: String::new(),
        },
        full_path,
    ))
}

fn find_attr_recursive(node: &XmlNode, local_tag: &str, attr_key: &str) -> Option<String> {
    if node.local_name() == local_tag {
        if let Some(v) = node.attr(attr_key) {
            return Some(v.to_string());
        }
    }
    for c in &node.children {
        if let XmlChild::Element(child) = c {
            if let Some(found) = find_attr_recursive(child, local_tag, attr_key) {
                return Some(found);
            }
        }
    }
    None
}
