//! ECMA-376 OpenXML (`.docx`) Serializer with Track Changes (`<w:ins>`/`<w:del>`),
//! Threaded Comments (`word/comments.xml`), Settings, and Lossless Archive Passthrough.

use crate::xml_lite::escape_xml;
use crate::DocxError;
use doc_model::{
    Alignment, Block, DocComment, Document, ListKind, Paragraph, ParagraphStyle, RevisionKind,
    RgbColor, SharedBytes, TableBlock,
};
use rustpad_zip::ZipWriter;
use std::path::Path;

pub fn save_docx_file(path: impl AsRef<Path>, doc: &Document) -> Result<(), DocxError> {
    let bytes = save_docx_bytes(doc);
    std::fs::write(path, bytes).map_err(|e| DocxError(e.to_string()))
}

pub fn save_docx_bytes(doc: &Document) -> Vec<u8> {
    let mut zw = ZipWriter::new();

    // Collect embedded images with true media extensions (`png`, `jpeg`, `bmp`, `gif`)
    let mut images: Vec<(String, SharedBytes)> = Vec::new();
    for block in &doc.blocks {
        if let Block::Image(img) = block {
            if !img.png_bytes.is_empty() {
                let idx = images.len() + 1;
                let ext = img.media_extension();
                images.push((format!("media/image{idx}.{ext}"), img.png_bytes.clone()));
            }
        }
    }

    let has_comments = !doc.comments.is_empty();

    zw.add_file(
        "[Content_Types].xml",
        build_content_types(has_comments).into_bytes(),
    );
    zw.add_file("_rels/.rels", build_root_rels().into_bytes());
    zw.add_file("docProps/core.xml", build_core_props(doc).into_bytes());
    zw.add_file(
        "word/_rels/document.xml.rels",
        build_doc_rels(&images, has_comments).into_bytes(),
    );
    zw.add_file("word/styles.xml", build_styles_xml().into_bytes());
    zw.add_file("word/numbering.xml", build_numbering_xml().into_bytes());
    zw.add_file(
        "word/settings.xml",
        build_settings_xml(doc.track_changes).into_bytes(),
    );
    zw.add_file(
        "word/header1.xml",
        build_header_xml(&doc.page_setup.header_text).into_bytes(),
    );
    zw.add_file(
        "word/footer1.xml",
        build_footer_xml(&doc.page_setup.footer_text, doc.page_setup.show_page_numbers)
            .into_bytes(),
    );
    if has_comments {
        zw.add_file(
            "word/comments.xml",
            build_comments_xml(&doc.comments).into_bytes(),
        );
    }
    zw.add_file("word/document.xml", build_document_xml(doc).into_bytes());

    for (rel_path, data) in &images {
        zw.add_file(format!("word/{rel_path}"), data.to_vec());
    }

    // Re-emit unmodeled enterprise passthrough parts (`customXml/*`, `word/theme/*`, etc.)
    for (part_name, data) in &doc.passthrough_parts {
        let collides_img = images
            .iter()
            .any(|(rel, _)| format!("word/{rel}") == *part_name);
        if !collides_img {
            zw.add_file(part_name.clone(), data.clone());
        }
    }

    zw.finish()
}

fn build_content_types(has_comments: bool) -> String {
    let comments_override = if has_comments {
        "\n  <Override PartName=\"/word/comments.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml\"/>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="jpeg" ContentType="image/jpeg"/>
  <Default Extension="jpg" ContentType="image/jpeg"/>
  <Default Extension="bmp" ContentType="image/bmp"/>
  <Default Extension="gif" ContentType="image/gif"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>{comments_override}
</Types>"#
    )
}

fn build_root_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#
        .to_string()
}

fn build_core_props(doc: &Document) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
  xmlns:dc="http://purl.org/dc/elements/1.1/"
  xmlns:dcterms="http://purl.org/dc/terms/">
  <dc:title>{}</dc:title>
  <dc:creator>{}</dc:creator>
  <cp:lastModifiedBy>{}</cp:lastModifiedBy>
  <cp:category>{}</cp:category>
</cp:coreProperties>"#,
        escape_xml(&doc.title),
        escape_xml(&doc.author),
        escape_xml(&doc.author),
        escape_xml(doc.classification.slug()),
    )
}

fn build_settings_xml(track_changes: bool) -> String {
    let track_node = if track_changes {
        "  <w:trackRevisions/>\n"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
{track_node}</w:settings>"#
    )
}

fn build_comments_xml(comments: &[DocComment]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
"#,
    );
    for c in comments {
        let done_val = if c.resolved { "1" } else { "0" };
        let date_attr = if c.date_iso.is_empty() {
            "2026-10-27T00:00:00Z"
        } else {
            c.date_iso.as_str()
        };
        xml.push_str(&format!(
            r#"  <w:comment w:id="{}" w:author="{}" w:date="{}" w:done="{done_val}"><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:comment>"#,
            c.id,
            escape_xml(&c.author),
            escape_xml(date_attr),
            escape_xml(&c.text)
        ));
        xml.push('\n');
    }
    xml.push_str("</w:comments>");
    xml
}

fn build_doc_rels(images: &[(String, Vec<u8>)], has_comments: bool) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rIdNumbering" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>
  <Relationship Id="rIdSettings" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rIdHeader1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
  <Relationship Id="rIdFooter1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
"#,
    );
    if has_comments {
        xml.push_str(
            r#"  <Relationship Id="rIdComments" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>"#,
        );
        xml.push('\n');
    }
    for (idx, (target, _)) in images.iter().enumerate() {
        let id = idx + 1;
        xml.push_str(&format!(
            r#"  <Relationship Id="rIdImg{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{target}"/>"#
        ));
        xml.push('\n');
    }
    xml.push_str("</Relationships>");
    xml
}

fn build_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/><w:sz w:val="23"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Title">
    <w:name w:val="Title"/>
    <w:rPr><w:b/><w:sz w:val="52"/><w:color w:val="0F172A"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Subtitle">
    <w:name w:val="Subtitle"/>
    <w:rPr><w:sz w:val="30"/><w:color w:val="64748B"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:rPr><w:b/><w:sz w:val="40"/><w:color w:val="0F172A"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:rPr><w:b/><w:sz w:val="32"/><w:color w:val="0F172A"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:rPr><w:b/><w:sz w:val="27"/><w:color w:val="0F172A"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Quote">
    <w:name w:val="Quote"/>
    <w:rPr><w:i/><w:sz w:val="23"/><w:color w:val="64748B"/></w:rPr>
  </w:style>
</w:styles>"#
        .to_string()
}

fn build_numbering_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="•"/>
      <w:lvlJc w:val="left"/>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="2">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
      <w:lvlJc w:val="left"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="1"/></w:num>
  <w:num w:numId="2"><w:abstractNumId w:val="2"/></w:num>
</w:numbering>"#
        .to_string()
}

fn build_header_xml(header_text: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p>
    <w:r><w:rPr><w:color w:val="64748B"/><w:sz w:val="18"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>
  </w:p>
</w:hdr>"#,
        escape_xml(header_text)
    )
}

fn build_footer_xml(footer_text: &str, show_page_numbers: bool) -> String {
    let mut page_run = String::new();
    if show_page_numbers {
        let prefix = if footer_text.is_empty() {
            "Page "
        } else {
            "   ·   Page "
        };
        page_run = format!(
            r#"<w:r><w:rPr><w:color w:val="64748B"/><w:sz w:val="18"/></w:rPr><w:t xml:space="preserve">{prefix}</w:t></w:r><w:fldSimple w:instr="PAGE"/>"#
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p>
    <w:r><w:rPr><w:color w:val="64748B"/><w:sz w:val="18"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>
    {page_run}
  </w:p>
</w:ftr>"#,
        escape_xml(footer_text)
    )
}

fn style_id(s: ParagraphStyle) -> &'static str {
    match s {
        ParagraphStyle::Normal => "Normal",
        ParagraphStyle::Title => "Title",
        ParagraphStyle::Subtitle => "Subtitle",
        ParagraphStyle::Heading1 => "Heading1",
        ParagraphStyle::Heading2 => "Heading2",
        ParagraphStyle::Heading3 => "Heading3",
        ParagraphStyle::Quote => "Quote",
    }
}

fn jc_val(a: Alignment) -> &'static str {
    match a {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "both",
    }
}

fn write_paragraph_xml(out: &mut String, p: &Paragraph, anchored_comments: &[&DocComment]) {
    out.push_str("<w:p><w:pPr>");
    out.push_str(&format!(r#"<w:pStyle w:val="{}"/>"#, style_id(p.style)));
    if p.alignment != Alignment::Left {
        out.push_str(&format!(r#"<w:jc w:val="{}"/>"#, jc_val(p.alignment)));
    }
    match p.list_kind {
        ListKind::Bullet => {
            out.push_str(
                r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>"#,
            );
        }
        ListKind::Numbered(_) => {
            out.push_str(
                r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr>"#,
            );
        }
        ListKind::Checklist(_) | ListKind::None => {}
    }
    if p.indent_level > 0 {
        let twips = (p.indent_level as u32) * 360;
        out.push_str(&format!(r#"<w:ind w:left="{twips}"/>"#));
    }
    let line_twips = (p.line_spacing * 240.0).round() as u32;
    out.push_str(&format!(
        r#"<w:spacing w:after="120" w:line="{line_twips}" w:lineRule="auto"/>"#
    ));
    out.push_str("</w:pPr>");

    for c in anchored_comments {
        out.push_str(&format!(r#"<w:commentRangeStart w:id="{}"/>"#, c.id));
    }

    if let ListKind::Checklist(done) = p.list_kind {
        let mark = if done { "☑ " } else { "☐ " };
        out.push_str(&format!(
            r#"<w:r><w:t xml:space="preserve">{mark}</w:t></w:r>"#
        ));
    }

    for span in &p.spans {
        let mut r_xml = String::from("<w:r><w:rPr>");
        let font = span.style.font_family.docx_name();
        r_xml.push_str(&format!(
            r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}"/>"#
        ));
        if span.style.bold {
            r_xml.push_str("<w:b/>");
        }
        if span.style.italic {
            r_xml.push_str("<w:i/>");
        }
        if span.style.underline {
            r_xml.push_str(r#"<w:u w:val="single"/>"#);
        }
        if span.style.strikethrough {
            r_xml.push_str("<w:strike/>");
        }
        let half_pts = (span.style.font_size_pt * 2.0).round() as u32;
        r_xml.push_str(&format!(r#"<w:sz w:val="{half_pts}"/>"#));
        r_xml.push_str(&format!(
            r#"<w:color w:val="{}"/>"#,
            span.style.color.to_hex()
        ));
        if let Some(hl) = span.style.highlight {
            r_xml.push_str(&format!(
                r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
                hl.to_hex()
            ));
        }
        r_xml.push_str("</w:rPr>");

        match &span.revision {
            Some(rev) if rev.kind == RevisionKind::Deletion => {
                r_xml.push_str(&format!(
                    r#"<w:delText xml:space="preserve">{}</w:delText></w:r>"#,
                    escape_xml(&span.text)
                ));
                let date_attr = if rev.date_iso.is_empty() {
                    "2026-10-27T00:00:00Z"
                } else {
                    rev.date_iso.as_str()
                };
                out.push_str(&format!(
                    r#"<w:del w:id="{}" w:author="{}" w:date="{}">{r_xml}</w:del>"#,
                    rev.id,
                    escape_xml(&rev.author),
                    escape_xml(date_attr)
                ));
            }
            Some(rev) => {
                r_xml.push_str(&format!(
                    r#"<w:t xml:space="preserve">{}</w:t></w:r>"#,
                    escape_xml(&span.text)
                ));
                let date_attr = if rev.date_iso.is_empty() {
                    "2026-10-27T00:00:00Z"
                } else {
                    rev.date_iso.as_str()
                };
                out.push_str(&format!(
                    r#"<w:ins w:id="{}" w:author="{}" w:date="{}">{r_xml}</w:ins>"#,
                    rev.id,
                    escape_xml(&rev.author),
                    escape_xml(date_attr)
                ));
            }
            None => {
                r_xml.push_str(&format!(
                    r#"<w:t xml:space="preserve">{}</w:t></w:r>"#,
                    escape_xml(&span.text)
                ));
                out.push_str(&r_xml);
            }
        }
    }

    for c in anchored_comments {
        out.push_str(&format!(
            r#"<w:commentRangeEnd w:id="{id}"/><w:r><w:commentReference w:id="{id}"/></w:r>"#,
            id = c.id
        ));
    }

    out.push_str("</w:p>\n");
}

fn write_table_xml(out: &mut String, t: &TableBlock, content_width_twips: u32) {
    let border_hex = t.border_color.to_hex();
    out.push_str("<w:tbl><w:tblPr>");
    out.push_str(r#"<w:tblW w:w="5000" w:type="pct"/>"#);
    out.push_str(&format!(
        r#"<w:tblBorders>
      <w:top w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
      <w:left w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
      <w:bottom w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
      <w:right w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
      <w:insideH w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
      <w:insideV w:val="single" w:sz="6" w:space="0" w:color="{border_hex}"/>
    </w:tblBorders></w:tblPr>"#
    ));
    let cell_w = content_width_twips / (t.cols.max(1) as u32);
    for row in &t.rows {
        out.push_str("<w:tr>");
        for cell in row {
            out.push_str("<w:tc><w:tcPr>");
            out.push_str(&format!(r#"<w:tcW w:w="{cell_w}" w:type="dxa"/>"#));
            if let Some(bg) = cell.bg_color {
                out.push_str(&format!(
                    r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
                    bg.to_hex()
                ));
            }
            out.push_str("</w:tcPr>");
            for line in cell.text.split('\n') {
                out.push_str("<w:p><w:pPr>");
                if cell.alignment != Alignment::Left {
                    out.push_str(&format!(r#"<w:jc w:val="{}"/>"#, jc_val(cell.alignment)));
                }
                out.push_str("</w:pPr><w:r><w:rPr>");
                if cell.bold {
                    out.push_str("<w:b/>");
                }
                out.push_str(&format!(
                    r#"<w:color w:val="{}"/>"#,
                    RgbColor::INK_SLATE.to_hex()
                ));
                out.push_str("</w:rPr>");
                out.push_str(&format!(
                    r#"<w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
                    escape_xml(line)
                ));
            }
            out.push_str("</w:tc>");
        }
        out.push_str("</w:tr>\n");
    }
    out.push_str("</w:tbl>\n");
}

fn build_document_xml(doc: &Document) -> String {
    let (w_pt, h_pt) = doc
        .page_setup
        .paper_size
        .dimensions_pt(doc.page_setup.landscape);
    let w_twips = (w_pt * 20.0).round() as u32;
    let h_twips = (h_pt * 20.0).round() as u32;
    let m_top = (doc.page_setup.margin_top_pt * 20.0).round() as u32;
    let m_bot = (doc.page_setup.margin_bottom_pt * 20.0).round() as u32;
    let m_left = (doc.page_setup.margin_left_pt * 20.0).round() as u32;
    let m_right = (doc.page_setup.margin_right_pt * 20.0).round() as u32;
    let content_twips = w_twips.saturating_sub(m_left + m_right).max(2000);

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
  <w:body>
"#,
    );

    let mut img_counter = 0usize;
    for (block_idx, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Paragraph(p) => {
                let anchored: Vec<&DocComment> = doc
                    .comments
                    .iter()
                    .filter(|c| c.block_index == block_idx)
                    .collect();
                write_paragraph_xml(&mut xml, p, &anchored);
            }
            Block::Table(t) => write_table_xml(&mut xml, t, content_twips),
            Block::HorizontalRule => {
                xml.push_str(
                    r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="CBD5E1"/></w:pBdr></w:pPr></w:p>"#,
                );
            }
            Block::PageBreak => {
                xml.push_str(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#);
            }
            Block::Image(img) => {
                if !img.png_bytes.is_empty() {
                    img_counter += 1;
                    let cx = (img.width_pt.max(24.0) * 12700.0).round() as u64;
                    let cy = (img.height_pt.max(24.0) * 12700.0).round() as u64;
                    let jc = jc_val(img.alignment);
                    let esc_name = escape_xml(&img.name);
                    let esc_caption = escape_xml(&img.caption);
                    xml.push_str(&format!(
                        r#"<w:p><w:pPr><w:jc w:val="{jc}"/></w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{img_counter}" name="{esc_name}" descr="{esc_caption}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{img_counter}" name="{esc_name}" descr="{esc_caption}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rIdImg{img_counter}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
"#
                    ));
                }
            }
        }
    }

    let orient_attr = if doc.page_setup.landscape {
        r#" w:orient="landscape""#
    } else {
        ""
    };
    xml.push_str(&format!(
        r#"  <w:sectPr>
      <w:headerReference w:type="default" r:id="rIdHeader1"/>
      <w:footerReference w:type="default" r:id="rIdFooter1"/>
      <w:pgSz w:w="{w_twips}" w:h="{h_twips}"{orient_attr}/>
      <w:pgMar w:top="{m_top}" w:right="{m_right}" w:bottom="{m_bot}" w:left="{m_left}" w:header="720" w:footer="720"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    ));
    xml
}
