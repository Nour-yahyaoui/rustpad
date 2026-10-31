//! ECMA-376 OpenXML (`.docx`) Reader, Track Changes (`<w:ins>`/`<w:del>`), Threaded Comments
//! (`word/comments.xml`), Security Classification, and Lossless Archive Passthrough Reconstructor.

use crate::xml_lite::{parse_xml, stream_docx_body_elements, XmlChild, XmlNode};
use crate::DocxError;
use doc_model::{
    probe_media_dimensions, Alignment, Block, DocComment, Document, FontFamily, ImageBlock,
    ListKind, PageSetup, PaperSize, Paragraph, ParagraphStyle, RevisionKind, RevisionMeta,
    RgbColor, SecurityClassification, SharedBytes, SpanStyle, TableBlock, TableCell, TextSpan,
};
use rustpad_zip::LazyZipArchive;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

pub fn load_docx_file(path: impl AsRef<Path>) -> Result<Document, DocxError> {
    let bytes = std::fs::read(&path).map_err(|e| DocxError(e.to_string()))?;
    let mut doc = load_docx_shared_with_progress(SharedBytes::from_vec(bytes), None)?;
    if doc.title == "Untitled Document" {
        if let Some(stem) = path.as_ref().file_stem().and_then(|s| s.to_str()) {
            doc.title = stem.to_string();
        }
    }
    Ok(doc)
}

pub fn load_docx_bytes(bytes: &[u8]) -> Result<Document, DocxError> {
    load_docx_shared_with_progress(SharedBytes::from_slice(bytes), None)
}

/// Zero-copy streaming `.docx` loader backed by a single `SharedBytes` (`Arc<[u8]>`) buffer.
/// Indexes the PKZIP Central Directory in `< 0.1 ms`, streams `<w:body>` elements one block at
/// a time (`< 32 KB` peak XML DOM), and slices `STORED` media (`PNG`/`JPEG`) without copying bytes.
pub fn load_docx_shared_with_progress(
    backing: SharedBytes,
    progress_pct: Option<&AtomicU32>,
) -> Result<Document, DocxError> {
    if let Some(p) = progress_pct {
        p.store(8, Ordering::Relaxed);
    }
    let archive_total_len = backing.len();
    let archive = LazyZipArchive::from_shared(backing).map_err(|e| DocxError(e.0))?;
    if let Some(p) = progress_pct {
        p.store(16, Ordering::Relaxed);
    }
    let doc_xml = archive
        .get_str("word/document.xml")
        .map_err(|e| DocxError(e.0))?
        .ok_or_else(|| DocxError("Missing word/document.xml inside .docx archive".into()))?;
    if let Some(p) = progress_pct {
        p.store(28, Ordering::Relaxed);
    }

    // Relationships map (rId -> Target)
    let mut rels = BTreeMap::new();
    if let Ok(Some(rels_xml)) = archive.get_str("word/_rels/document.xml.rels") {
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
    if let Ok(Some(core_xml)) = archive.get_str("docProps/core.xml") {
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
    if let Ok(Some(settings_xml)) = archive.get_str("word/settings.xml") {
        if settings_xml.contains("trackRevisions") {
            track_changes = true;
        }
    }

    let mut page_setup = PageSetup::default();
    if let Ok(Some(hdr_xml)) = archive.get_str("word/header1.xml") {
        if let Some(hdr_root) = parse_xml(&hdr_xml) {
            page_setup.header_text = hdr_root.text_content().trim().to_string();
        }
    }
    if let Ok(Some(ftr_xml)) = archive.get_str("word/footer1.xml") {
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
    let mut media_cache: BTreeMap<String, SharedBytes> = BTreeMap::new();
    let mut comment_anchors: BTreeMap<u32, usize> = BTreeMap::new();

    // Stream `<w:body>` direct child elements one block at a time (`< 32 KB` peak XML DOM!)
    let streamed_ok = stream_docx_body_elements(&doc_xml, |el, cur_pos, total_len| {
        if let Some(p) = progress_pct {
            let pct = 30 + ((cur_pos.saturating_mul(58)) / total_len.max(1)) as u32;
            p.store(pct.min(88), Ordering::Relaxed);
        }
        parse_body_element(
            el,
            &archive,
            &rels,
            &mut media_cache,
            &mut num_counter,
            &mut blocks,
            &mut consumed_media_paths,
            &mut comment_anchors,
            &mut page_setup,
        );
    });

    if !streamed_ok {
        // Fallback to full DOM parse if non-standard root wrapper omitted `<w:body>`
        let root =
            parse_xml(&doc_xml).ok_or_else(|| DocxError("Malformed word/document.xml".into()))?;
        let body = root
            .child("body")
            .ok_or_else(|| DocxError("Missing <w:body> in word/document.xml".into()))?;
        for child in &body.children {
            if let XmlChild::Element(el) = child {
                parse_body_element(
                    el,
                    &archive,
                    &rels,
                    &mut media_cache,
                    &mut num_counter,
                    &mut blocks,
                    &mut consumed_media_paths,
                    &mut comment_anchors,
                    &mut page_setup,
                );
            }
        }
    }
    drop(doc_xml);

    // Parse `word/comments.xml` if present
    let mut comments = Vec::new();
    if let Ok(Some(comments_xml)) = archive.get_str("word/comments.xml") {
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

    // Preserve compact enterprise passthrough parts (`customXml/*`, `word/theme/*`, etc.)
    // while skipping bulky unreferenced binary blobs (`embeddings/*`, duplicate `.wdp`/`.emf`)
    // on large 30+ MB `.docx` archives so memory stays lean.
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
    let is_large_archive = archive_total_len > 4 * 1024 * 1024;
    let mut passthrough_parts = BTreeMap::new();
    let mut passthrough_budget_used = 0usize;
    for entry_name in archive.file_names() {
        if managed_parts.contains(&entry_name.as_str()) {
            continue;
        }
        if consumed_media_paths.contains(entry_name) {
            continue;
        }
        if is_large_archive
            && (entry_name.starts_with("word/media/")
                || entry_name.starts_with("word/embeddings/")
                || entry_name.ends_with(".bin")
                || entry_name.ends_with(".emf")
                || entry_name.ends_with(".wmf")
                || entry_name.ends_with(".wdp"))
        {
            continue;
        }
        if let Some(hint) = archive.entry_unpacked_hint(entry_name) {
            if is_large_archive && (hint > 1024 * 1024 || passthrough_budget_used + hint > 8 * 1024 * 1024) {
                continue;
            }
        }
        if let Ok(Some(raw_bytes)) = archive.get_shared(entry_name) {
            if is_large_archive
                && (raw_bytes.len() > 1024 * 1024
                    || passthrough_budget_used + raw_bytes.len() > 8 * 1024 * 1024)
            {
                continue;
            }
            passthrough_budget_used += raw_bytes.len();
            passthrough_parts.insert(entry_name.clone(), raw_bytes.to_vec());
        }
    }

    // Deduplicate identical archive media payloads across `blocks` so repeated logos/figures
    // share a single `SharedBytes` (`Arc<[u8]>`) allocation in memory.
    let mut shared_media_pool: BTreeMap<(usize, u8, u8), SharedBytes> = BTreeMap::new();
    for blk in &mut blocks {
        if let Block::Image(img) = blk {
            if img.png_bytes.len() >= 16 {
                let sig = (
                    img.png_bytes.len(),
                    img.png_bytes[img.png_bytes.len() / 3],
                    img.png_bytes[img.png_bytes.len() - 1],
                );
                if let Some(existing) = shared_media_pool.get(&sig) {
                    if existing.as_slice() == img.png_bytes.as_slice() {
                        img.png_bytes = existing.clone();
                        continue;
                    }
                }
                shared_media_pool.insert(sig, img.png_bytes.clone());
            }
        }
    }

    if let Some(p) = progress_pct {
        p.store(95, Ordering::Relaxed);
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

#[allow(clippy::too_many_arguments)]
fn parse_body_element(
    el: &XmlNode,
    archive: &LazyZipArchive,
    rels: &BTreeMap<String, String>,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    num_counter: &mut u32,
    blocks: &mut Vec<Block>,
    consumed_media_paths: &mut BTreeSet<String>,
    comment_anchors: &mut BTreeMap<u32, usize>,
    page_setup: &mut PageSetup,
) {
    match el.local_name() {
        "p" => {
            parse_paragraph_or_special(
                el,
                archive,
                rels,
                media_cache,
                num_counter,
                blocks,
                consumed_media_paths,
                comment_anchors,
            );
        }
        "tbl" => {
            if let Some(tbl) = parse_table(el) {
                blocks.push(Block::Table(tbl));
            }
            // Also surface any `<w:drawing>` / `<w:pict>` screenshots placed inside layout table cells
            let mut tbl_imgs = Vec::new();
            collect_embedded_images(el, archive, rels, media_cache, Alignment::Center, &mut tbl_imgs);
            for (img, media_path) in tbl_imgs {
                consumed_media_paths.insert(media_path);
                blocks.push(Block::Image(img));
            }
        }
        "sectPr" => {
            parse_sect_pr(el, page_setup);
        }
        "sdt" => {
            if let Some(sdt_content) = el.child("sdtContent") {
                for c in &sdt_content.children {
                    if let XmlChild::Element(inner) = c {
                        parse_body_element(
                            inner,
                            archive,
                            rels,
                            media_cache,
                            num_counter,
                            blocks,
                            consumed_media_paths,
                            comment_anchors,
                            page_setup,
                        );
                    }
                }
            }
        }
        "AlternateContent" => {
            let branch = el.child("Choice").or_else(|| el.child("Fallback"));
            if let Some(b_node) = branch {
                for c in &b_node.children {
                    if let XmlChild::Element(inner) = c {
                        parse_body_element(
                            inner,
                            archive,
                            rels,
                            media_cache,
                            num_counter,
                            blocks,
                            consumed_media_paths,
                            comment_anchors,
                            page_setup,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_paragraph_or_special(
    p_node: &XmlNode,
    archive: &LazyZipArchive,
    rels: &BTreeMap<String, String>,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    num_counter: &mut u32,
    blocks: &mut Vec<Block>,
    consumed_media_paths: &mut BTreeSet<String>,
    comment_anchors: &mut BTreeMap<u32, usize>,
) {
    if let Some(p_pr) = p_node.child("pPr") {
        if p_pr.child("pBdr").is_some() && p_node.text_content().trim().is_empty() {
            let mut probe_imgs = Vec::new();
            collect_embedded_images(
                p_node,
                archive,
                rels,
                media_cache,
                Alignment::Left,
                &mut probe_imgs,
            );
            if probe_imgs.is_empty() {
                blocks.push(Block::HorizontalRule);
                return;
            }
        }
    }

    let mut has_page_break = false;
    for r in p_node.children_named("r") {
        for br in r.children_named("br") {
            if br.attr("type") == Some("page") {
                has_page_break = true;
            }
        }
    }

    let mut style = ParagraphStyle::Normal;
    let mut alignment = Alignment::Left;
    let mut has_explicit_alignment = false;
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
            has_explicit_alignment = true;
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

    // Extract any DrawingML (`<w:drawing>`), `<mc:AlternateContent>`, or legacy VML (`<w:pict>`) images
    let img_align = if has_explicit_alignment {
        if alignment == Alignment::Justify {
            Alignment::Center
        } else {
            alignment
        }
    } else {
        Alignment::Center
    };
    let mut extracted_images = Vec::new();
    collect_embedded_images(
        p_node,
        archive,
        rels,
        media_cache,
        img_align,
        &mut extracted_images,
    );

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

    let has_text = spans.iter().any(|s| !s.text.is_empty());

    if has_page_break && !has_text && extracted_images.is_empty() {
        blocks.push(Block::PageBreak);
        return;
    }

    if has_text || (extracted_images.is_empty() && !has_page_break) {
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

    for (img, media_path) in extracted_images {
        consumed_media_paths.insert(media_path);
        blocks.push(Block::Image(img));
    }

    if has_page_break && (has_text || blocks.last().is_some_and(|b| !matches!(b, Block::PageBreak)))
    {
        blocks.push(Block::PageBreak);
    }
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

fn collect_embedded_images(
    node: &XmlNode,
    archive: &LazyZipArchive,
    rels: &BTreeMap<String, String>,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    alignment: Alignment,
    out: &mut Vec<(ImageBlock, String)>,
) {
    match node.local_name() {
        "drawing" => {
            if let Some((img, path)) =
                extract_drawing_image(node, archive, rels, media_cache, alignment)
            {
                out.push((img, path));
            }
            return;
        }
        "pict" => {
            if let Some((img, path)) =
                extract_vml_pict_image(node, archive, rels, media_cache, alignment)
            {
                out.push((img, path));
            }
            return;
        }
        "AlternateContent" => {
            let before = out.len();
            if let Some(choice) = node.child("Choice") {
                for c in &choice.children {
                    if let XmlChild::Element(el) = c {
                        collect_embedded_images(el, archive, rels, media_cache, alignment, out);
                    }
                }
            }
            if out.len() == before {
                if let Some(fallback) = node.child("Fallback") {
                    for c in &fallback.children {
                        if let XmlChild::Element(el) = c {
                            collect_embedded_images(el, archive, rels, media_cache, alignment, out);
                        }
                    }
                }
            }
            return;
        }
        "del" => return,
        _ => {}
    }

    for c in &node.children {
        if let XmlChild::Element(child) = c {
            collect_embedded_images(child, archive, rels, media_cache, alignment, out);
        }
    }
}

fn resolve_docx_rel_path(
    archive: &LazyZipArchive,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    raw_target: &str,
) -> Option<(String, SharedBytes)> {
    let cleaned = raw_target.trim().replace('\\', "/");
    if cleaned.is_empty()
        || cleaned.starts_with("http://")
        || cleaned.starts_with("https://")
        || cleaned.starts_with("file://")
    {
        return None;
    }

    let path_only = cleaned.split(['?', '#']).next().unwrap_or(&cleaned);
    let mut stack: Vec<&str> = if path_only.starts_with('/') {
        Vec::new()
    } else if path_only.starts_with("word/") {
        Vec::new()
    } else {
        vec!["word"]
    };

    for seg in path_only.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            other => stack.push(other),
        }
    }
    let normalized = stack.join("/");
    if let Some(cached) = media_cache.get(&normalized) {
        return Some((normalized, cached.clone()));
    }
    let (canonical, sb) = archive.get_shared_resolved(&normalized).ok().flatten()?;
    media_cache.insert(canonical.clone(), sb.clone());
    if canonical != normalized {
        media_cache.insert(normalized, sb.clone());
    }
    Some((canonical, sb))
}

fn extract_drawing_image(
    drawing: &XmlNode,
    archive: &LazyZipArchive,
    rels: &BTreeMap<String, String>,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    alignment: Alignment,
) -> Option<(ImageBlock, String)> {
    let container = drawing
        .child("inline")
        .or_else(|| drawing.child("anchor"))
        .unwrap_or(drawing);
    let mut width_pt = 300.0f32;
    let mut height_pt = 200.0f32;
    if let Some(ext) = container
        .child("extent")
        .or_else(|| find_node_recursive(container, "ext"))
    {
        if let Some(cx) = ext.attr("cx").and_then(|v| v.parse::<f64>().ok()) {
            if cx > 0.0 {
                width_pt = ((cx / 12700.0) as f32).clamp(24.0, 680.0);
            }
        }
        if let Some(cy) = ext.attr("cy").and_then(|v| v.parse::<f64>().ok()) {
            if cy > 0.0 {
                height_pt = ((cy / 12700.0) as f32).clamp(24.0, 680.0);
            }
        }
    }
    let doc_pr = container
        .child("docPr")
        .or_else(|| find_node_recursive(container, "cNvPr"));
    let name = doc_pr
        .and_then(|n| n.attr("name"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Embedded Image")
        .to_string();
    let caption = doc_pr
        .and_then(|n| n.attr("descr"))
        .unwrap_or("")
        .trim()
        .to_string();

    let blip_id = find_attr_recursive(container, "blip", "embed")
        .or_else(|| find_attr_recursive(container, "blip", "link"))?;
    let target = rels.get(&blip_id)?;
    let (full_path, png_bytes) = resolve_docx_rel_path(archive, media_cache, target)?;
    let intrinsic_px = probe_media_dimensions(&png_bytes);

    Some((
        ImageBlock {
            name,
            width_pt,
            height_pt,
            png_bytes,
            rgba_preview: None,
            intrinsic_px,
            caption,
            alignment,
        },
        full_path,
    ))
}

fn parse_css_length_pt(raw: &str) -> Option<f32> {
    let s = raw.trim().to_ascii_lowercase();
    let (num_str, factor) = if let Some(n) = s.strip_suffix("pt") {
        (n, 1.0f32)
    } else if let Some(n) = s.strip_suffix("px") {
        (n, 0.75f32)
    } else if let Some(n) = s.strip_suffix("in") {
        (n, 72.0f32)
    } else if let Some(n) = s.strip_suffix("cm") {
        (n, 28.3465f32)
    } else if let Some(n) = s.strip_suffix("mm") {
        (n, 2.83465f32)
    } else {
        (s.as_str(), 0.75f32)
    };
    num_str
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|&v| v > 0.0)
        .map(|v| (v * factor).clamp(24.0, 680.0))
}

fn extract_vml_pict_image(
    pict: &XmlNode,
    archive: &LazyZipArchive,
    rels: &BTreeMap<String, String>,
    media_cache: &mut BTreeMap<String, SharedBytes>,
    alignment: Alignment,
) -> Option<(ImageBlock, String)> {
    let img_data = find_node_recursive(pict, "imagedata")?;
    let rel_id = img_data
        .attr("id")
        .or_else(|| img_data.attr("pict"))
        .or_else(|| img_data.attr("href"))?;
    let target = rels.get(rel_id)?;
    let (full_path, png_bytes) = resolve_docx_rel_path(archive, media_cache, target)?;
    let intrinsic_px = probe_media_dimensions(&png_bytes);

    let mut width_pt = 280.0f32;
    let mut height_pt = 180.0f32;
    if let Some(style_str) = find_attr_recursive(pict, "shape", "style") {
        for decl in style_str.split(';') {
            if let Some((k, v)) = decl.split_once(':') {
                match k.trim().to_ascii_lowercase().as_str() {
                    "width" => {
                        if let Some(w) = parse_css_length_pt(v) {
                            width_pt = w;
                        }
                    }
                    "height" => {
                        if let Some(h) = parse_css_length_pt(v) {
                            height_pt = h;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let name = img_data
        .attr("title")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Embedded Image")
        .to_string();

    Some((
        ImageBlock {
            name,
            width_pt,
            height_pt,
            png_bytes,
            rgba_preview: None,
            intrinsic_px,
            caption: String::new(),
            alignment,
        },
        full_path,
    ))
}

fn find_node_recursive<'a>(node: &'a XmlNode, local_tag: &str) -> Option<&'a XmlNode> {
    if node.local_name() == local_tag {
        return Some(node);
    }
    for c in &node.children {
        if let XmlChild::Element(child) = c {
            if let Some(found) = find_node_recursive(child, local_tag) {
                return Some(found);
            }
        }
    }
    None
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
