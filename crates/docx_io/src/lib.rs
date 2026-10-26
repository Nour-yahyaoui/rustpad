//! Pure-Rust ECMA-376 Microsoft Word (`.docx`) OpenXML Reader & Writer.

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
    use doc_model::{Block, TemplateKind};

    #[test]
    fn test_docx_welcome_roundtrip() {
        let original = TemplateKind::WelcomeGuide.build();
        let bytes = save_docx_bytes(&original);
        assert!(bytes.len() > 800);

        let parsed = load_docx_bytes(&bytes).expect("parse generated docx");
        assert_eq!(parsed.title, "Welcome to RustPad");
        assert_eq!(parsed.author, "Nour Yahyaoui");
        assert_eq!(
            parsed.page_setup.header_text,
            "RustPad — Pure-Rust Word & PDF Desktop Studio"
        );
        let has_table = parsed.blocks.iter().any(|b| matches!(b, Block::Table(_)));
        assert!(has_table, "expected round-tripped table block");
        let plain = parsed.to_plain_text();
        assert!(plain.contains("RustPad Document & PDF Studio"));
        assert!(plain.contains("Paper & Daylight"));
        assert!(plain.contains("Word Studio (.docx)"));
    }
}
