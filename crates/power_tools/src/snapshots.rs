//! Local Time-Machine Version Snapshots Vault (`SnapshotVault`).
//! Stores compressed `.docx` snapshots locally for 1-click rewind & crash recovery.

use doc_model::Document;
use docx_io::{load_docx_bytes, save_docx_bytes};
use rustpad_zip::{zlib_compress, zlib_decompress};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    pub id: u64,
    pub timestamp_secs: u64,
    pub label: String,
    pub word_count: usize,
    pub compressed_bytes: Vec<u8>,
}

impl SnapshotEntry {
    pub fn human_age(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(self.timestamp_secs);
        let diff = now.saturating_sub(self.timestamp_secs);
        if diff < 10 {
            "Just now".into()
        } else if diff < 60 {
            format!("{diff}s ago")
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else {
            format!("{}h ago", diff / 3600)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotVault {
    entries: Vec<SnapshotEntry>,
    next_id: u64,
    max_entries: usize,
    disk_dir: Option<PathBuf>,
}

impl Default for SnapshotVault {
    fn default() -> Self {
        Self::new_in_memory(30)
    }
}

impl SnapshotVault {
    pub fn new_in_memory(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            max_entries: max_entries.max(5),
            disk_dir: None,
        }
    }

    pub fn with_disk_dir(dir: impl AsRef<Path>, max_entries: usize) -> Self {
        let path = dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&path);
        Self {
            entries: Vec::new(),
            next_id: 1,
            max_entries: max_entries.max(5),
            disk_dir: Some(path),
        }
    }

    pub fn capture(&mut self, doc: &Document, label: impl Into<String>) -> u64 {
        let docx_bytes = save_docx_bytes(doc);
        let compressed = zlib_compress(&docx_bytes);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let id = self.next_id;
        self.next_id += 1;

        if let Some(dir) = &self.disk_dir {
            let file_path = dir.join(format!("snapshot_{id:05}.rpsnap"));
            let _ = std::fs::write(file_path, &compressed);
        }

        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(SnapshotEntry {
            id,
            timestamp_secs: ts,
            label: label.into(),
            word_count: doc.stats().words,
            compressed_bytes: compressed,
        });
        id
    }

    pub fn list(&self) -> &[SnapshotEntry] {
        &self.entries
    }

    pub fn restore(&self, id: u64) -> Result<Document, String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| format!("Snapshot #{id} not found"))?;
        let docx_bytes = zlib_decompress(&entry.compressed_bytes).map_err(|e| e.0)?;
        load_docx_bytes(&docx_bytes).map_err(|e| e.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{markdown_to_word, pdf_to_word, word_to_markdown, word_to_pdf_document, SpellChecker};
    use doc_model::TemplateKind;

    #[test]
    fn test_pdf_word_markdown_conversions_and_snapshots() {
        let orig = TemplateKind::AcademicReport.build();
        let mut vault = SnapshotVault::default();
        let snap1 = vault.capture(&orig, "Initial Report");

        let pdf_doc = word_to_pdf_document(&orig).expect("word -> pdf doc");
        let back_word = pdf_to_word(&pdf_doc);
        assert!(back_word
            .to_plain_text()
            .contains("High-Speed Native Document Processing"));

        let md = word_to_markdown(&orig);
        assert!(md.contains("# High-Speed Native Document Processing"));
        let from_md = markdown_to_word(&md, "From MD");
        assert!(from_md.stats().tables >= 1);

        let restored = vault.restore(snap1).expect("restore snapshot");
        assert_eq!(restored.title, orig.title);

        let checker = SpellChecker::new();
        assert!(checker.is_valid("RustPad"));
        assert!(checker.is_valid("Nour"));
        assert!(!checker.is_valid("documnt"));
        assert!(checker
            .suggestions("documnt", 5)
            .contains(&"document".to_string()));
    }
}
