//! RustPad v3.0 Enterprise Power Tools Suite:
//! - Bidirectional `Word (.docx) ↔ PDF (.pdf) ↔ HTML5 (.html) ↔ Markdown (.md)` converters
//! - Pillar D Security Engine: Permanent `.docx` & PII redaction, Metadata Cleaner, Hidden Stripper & `0600` Atomic I/O
//! - Pillar E/F Productivity Engine: Multi-Surface Search, Duplicate Paragraph Detector, Broken Link Auditor & Batch Processor
//! - Enterprise MDM `policy.json` DLP Governance, SHA-256 Hash-Chained Audit Trail & Crash WAL
//! - Native WinHTTP Schannel / Local TCP Pluggable AI Agent Bridge (`Ollama`, `nour-code`, Cloud)
//! - Offline Spellchecker, Pure-Rust Bitmap + Tesseract OCR, Semantic Accessibility Tree & CycloneDX 1.5 SBOM

pub mod ai_agent;
pub mod converter;
pub mod enterprise;
pub mod ocr;
pub mod productivity;
pub mod security;
pub mod snapshots;
pub mod spellcheck;

pub use ai_agent::{
    build_prompts, discover_local_ollama_models, parse_ai_output_to_blocks, run_ai_task,
    run_ai_task_with_policy, sanitize_model_output, AiAgentConfig, AiProviderPreset, AiResponse,
    AiTask,
};
pub use converter::{
    convert_file, markdown_to_word, pdf_to_word, word_to_html, word_to_markdown,
    word_to_pdf_document,
};
pub use enterprise::{
    build_document_a11y_tree, generate_cyclonedx_sbom_json, AccessibilityNode, AdvisoryFileLock,
    AuditEvent, AuditTrail, EnterprisePolicy, WalJournal, GENESIS_AUDIT_HASH,
};
pub use ocr::{extract_text_ocr_or_pdf, pure_rust_bitmap_ocr_bytes};
pub use productivity::{
    detect_broken_links, detect_duplicate_paragraphs, run_batch_docx_job, search_document,
    BatchJobSummary, BatchOperation, BrokenLinkFinding, DuplicateParagraphGroup, SearchHit,
    SearchSurface,
};
pub use security::{
    clean_document_metadata, clean_docx_file_metadata, redact_docx_document, redact_docx_file,
    strip_hidden_content, write_bytes_secure_atomic, MetadataCleanOptions, MetadataCleanReport,
    RedactionOptions, RedactionReport,
};
pub use snapshots::{SnapshotEntry, SnapshotVault};
pub use spellcheck::SpellChecker;

#[cfg(test)]
mod tests {
    use super::*;
    use doc_model::{Block, Document, Paragraph, ParagraphStyle, SpanStyle, TextSpan};

    #[test]
    fn test_redaction_metadata_clean_html_export_and_quality_audits() {
        let mut doc = Document::blank();
        doc.title = "Confidential Merger Spec".into();
        doc.author = "Alice Reviewer".into();
        doc.metadata.company = "Acme Holdings".into();
        doc.page_setup.header_text = "Contact: alice@acme.example".into();
        doc.blocks = vec![
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "Overview Specification",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "This paragraph is intentionally duplicated verbatim across two sections for testing.",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "This paragraph is intentionally duplicated verbatim across two sections for testing.",
            )),
        ];
        // Add a broken internal link + a hidden span + an API key
        let mut p = Paragraph::new(ParagraphStyle::Normal, "");
        p.spans = vec![
            TextSpan::styled(
                "Broken Jump",
                SpanStyle {
                    hyperlink: Some("#missing-section".into()),
                    underline: true,
                    ..SpanStyle::default()
                },
            ),
            TextSpan::plain(" token sk-live1234567890abcdefXYZ "),
            TextSpan::styled(
                "HIDDEN_WATERMARK",
                SpanStyle {
                    hidden: true,
                    ..SpanStyle::default()
                },
            ),
        ];
        doc.blocks.push(Block::Paragraph(p));

        let dups = detect_duplicate_paragraphs(&doc, 24);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].block_indices, vec![1, 2]);

        let broken = detect_broken_links(&doc, None);
        assert_eq!(broken.len(), 1);

        let hits = search_document(&doc, "alice@acme", false);
        assert_eq!(hits.len(), 1);

        let html_dark = word_to_html(&doc, true);
        assert!(html_dark.contains("data-theme=\"dark\""));
        assert!(!html_dark.contains("HIDDEN_WATERMARK"));

        let redact_rep = redact_docx_document(
            &mut doc,
            &RedactionOptions {
                terms: vec!["duplicated".into()],
                redact_emails: true,
                redact_api_secrets: true,
                redact_phone_and_id_numbers: true,
                purge_track_changes_history: true,
                strip_hidden_runs: true,
            },
        );
        assert!(redact_rep.total_hits() >= 4);
        assert!(!doc.page_setup.header_text.contains("alice@acme.example"));

        let clean_rep = clean_document_metadata(&mut doc, &MetadataCleanOptions::strict_privacy());
        assert!(clean_rep.cleared_metadata_fields >= 2);
        assert!(doc.author.is_empty());
        assert!(doc.metadata.company.is_empty());
    }
}
