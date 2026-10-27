//! RustPad v2.0 Enterprise Power Tools Suite:
//! - Bidirectional `Word (.docx) ↔ PDF (.pdf)` and `Markdown (.md) ↔ Word (.docx)` converters
//! - Enterprise MDM `policy.json` DLP Governance, SHA-256 Hash-Chained Audit Trail & Crash WAL
//! - Native WinHTTP Schannel / Local TCP Pluggable AI Agent Bridge (`Ollama`, `nour-code`, Cloud)
//! - Offline Spellchecker, OCR Text Extractor, Semantic Accessibility Tree & CycloneDX 1.5 SBOM

pub mod ai_agent;
pub mod converter;
pub mod enterprise;
pub mod ocr;
pub mod snapshots;
pub mod spellcheck;

pub use ai_agent::{
    build_prompts, discover_local_ollama_models, parse_ai_output_to_blocks, run_ai_task,
    run_ai_task_with_policy, sanitize_model_output, AiAgentConfig, AiProviderPreset, AiResponse,
    AiTask,
};
pub use converter::{
    convert_file, markdown_to_word, pdf_to_word, word_to_markdown, word_to_pdf_document,
};
pub use enterprise::{
    build_document_a11y_tree, generate_cyclonedx_sbom_json, AccessibilityNode, AdvisoryFileLock,
    AuditEvent, AuditTrail, EnterprisePolicy, WalJournal, GENESIS_AUDIT_HASH,
};
pub use ocr::extract_text_ocr_or_pdf;
pub use snapshots::{SnapshotEntry, SnapshotVault};
pub use spellcheck::SpellChecker;
