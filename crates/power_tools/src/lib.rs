//! RustPad Power Tools Suite:
//! - Bidirectional `Word (.docx) ↔ PDF (.pdf)` and `Markdown (.md) ↔ Word (.docx)` converters
//! - Offline Spellchecker with edit-distance suggestions
//! - Offline OCR & Scan Text Extractor
//! - Local Time-Machine Version Snapshots & Recent Files store

pub mod converter;
pub mod ocr;
pub mod snapshots;
pub mod spellcheck;

pub use converter::{
    convert_file, markdown_to_word, pdf_to_word, word_to_markdown, word_to_pdf_document,
};
pub use ocr::extract_text_ocr_or_pdf;
pub use snapshots::{SnapshotEntry, SnapshotVault};
pub use spellcheck::SpellChecker;
