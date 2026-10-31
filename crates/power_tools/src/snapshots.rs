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
    pub approx_bytes: usize,
    in_memory_doc: Option<Document>,
}

impl SnapshotEntry {
    pub fn approx_size_bytes(&self) -> usize {
        if !self.compressed_bytes.is_empty() {
            self.compressed_bytes.len()
        } else {
            self.approx_bytes
        }
    }

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
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let id = self.next_id;
        self.next_id += 1;

        let media_bytes = doc.estimated_media_bytes();
        let passthrough_bytes: usize = doc.passthrough_parts.values().map(|v| v.len()).sum();
        // Fast path for in-memory desktop vaults on large/media/passthrough documents:
        // store a zero-copy `SharedBytes`-backed `Document` snapshot in < 0.05 ms instead of
        // running double LZ77 compression over 30–100+ MB of images or prose on the UI thread!
        let (compressed, approx_bytes, in_memory_doc) = if self.disk_dir.is_none()
            && (media_bytes > 128 * 1024
                || passthrough_bytes > 128 * 1024
                || doc.blocks.len() > 256)
        {
            let slim = doc.clone_without_passthrough();
            let est = media_bytes
                .saturating_add(passthrough_bytes)
                .saturating_add(doc.blocks.len() * 128);
            (Vec::new(), est, Some(slim))
        } else {
                let docx_bytes = save_docx_bytes(doc);
                let comp = zlib_compress(&docx_bytes);
                if let Some(dir) = &self.disk_dir {
                    let file_path = dir.join(format!("snapshot_{id:05}.rpsnap"));
                    let _ = std::fs::write(file_path, &comp);
                }
                let len = comp.len();
                (comp, len, None)
            };

        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(SnapshotEntry {
            id,
            timestamp_secs: ts,
            label: label.into(),
            word_count: doc.stats().words,
            compressed_bytes: compressed,
            approx_bytes,
            in_memory_doc,
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
        if let Some(mem_doc) = &entry.in_memory_doc {
            return Ok(mem_doc.clone());
        }
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

    #[test]
    fn test_32mb_multi_page_100_screenshots_docx_roundtrip_and_zero_copy_cow() {
        use doc_model::{Block, Document, ImageBlock, Paragraph, ParagraphStyle, UndoHistory};

        let mut doc = Document::blank();
        doc.title = "Enterprise 32MB 100-Screenshot Stress Audit".into();
        doc.blocks.clear();

        // Build 100 distinct ~340 KiB synthetic PNG-header screenshot payloads (~34 MB total)
        // across 100 multi-page sections to reproduce the exact 30 MB+ `.docx` crash scenario.
        for i in 0..100u32 {
            doc.blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                format!("Section {}: Production Telemetry & Architecture Capture", i + 1),
            )));
            doc.blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "Detailed diagnostic commentary accompanying high-resolution Retina screenshot capture.",
            )));

            // Valid 24-byte PNG header + IHDR (3840 x 2160 4K dimensions) + 340 KiB payload
            let mut fake_png = vec![0u8; 340 * 1024];
            fake_png[..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
            fake_png[8..12].copy_from_slice(&13u32.to_be_bytes());
            fake_png[12..16].copy_from_slice(b"IHDR");
            fake_png[16..20].copy_from_slice(&3840u32.to_be_bytes());
            fake_png[20..24].copy_from_slice(&2160u32.to_be_bytes());
            // Unique big-endian tail marker so each screenshot hashes to its own O(1) bucket
            let tag = i.to_be_bytes();
            let n = fake_png.len();
            fake_png[n - 4..].copy_from_slice(&tag);

            let mut img = ImageBlock::new(format!("Screenshot-{:03}.png", i + 1), 440.0, 247.5, fake_png);
            img.caption = format!("Figure {}: 4K cluster dashboard capture", i + 1);
            assert_eq!(img.intrinsic_px, Some((3840, 2160)));
            doc.blocks.push(Block::Image(img));

            if i + 1 < 100 {
                doc.blocks.push(Block::PageBreak);
            }
        }

        // 1. Serialize >33 MB `.docx` via `STORED` media fast-path (no redundant LZ77 on PNGs)
        let docx_bytes = save_docx_bytes(&doc);
        assert!(
            docx_bytes.len() > 32 * 1024 * 1024,
            "expected >32 MiB .docx archive, got {} B",
            docx_bytes.len()
        );

        // 2. Reload >33 MB `.docx` with zero pixel decoding (`rgba_preview` stays None, `intrinsic_px` probed)
        let mut loaded = load_docx_bytes(&docx_bytes).expect("load >32MB 100-screenshot .docx");
        let img_blocks: Vec<&ImageBlock> = loaded
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(img_blocks.len(), 100);
        assert!(img_blocks.iter().all(|i| i.rgba_preview.is_none()));
        assert!(img_blocks.iter().all(|i| i.intrinsic_px == Some((3840, 2160))));

        // 3. `SnapshotVault::capture` uses zero-copy CoW (`SharedBytes`) instead of double LZ77!
        let mut vault = SnapshotVault::default();
        let snap_id = vault.capture(&loaded, "Opened / Initial state");
        let entry = &vault.list()[0];
        assert!(entry.compressed_bytes.is_empty());
        assert!(entry.approx_size_bytes() > 32 * 1024 * 1024);

        // 4. `UndoHistory` records 10 edits in O(1) media memory via `SharedBytes`
        let mut history = UndoHistory::default();
        for step in 0..10 {
            history.record(&loaded);
            loaded.blocks.push(Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                format!("Live edit #{step}"),
            )));
        }
        assert!(history.undo(&mut loaded));

        let restored = vault.restore(snap_id).expect("restore 32MB CoW snapshot");
        assert_eq!(restored.blocks.len(), doc.blocks.len());
    }
}
