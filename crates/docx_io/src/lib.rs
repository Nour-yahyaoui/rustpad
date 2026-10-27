//! Pure-Rust ECMA-376 Microsoft Word (`.docx`) OpenXML Reader & Writer
//! supporting Track Changes (`<w:ins>`/`<w:del>`), Threaded Comments (`word/comments.xml`),
//! Security Classifications, and Lossless Enterprise ZIP Passthrough.

pub mod reader;
pub mod writer;
pub mod xml_lite;

pub use reader::{load_docx_bytes, load_docx_file};
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
}
