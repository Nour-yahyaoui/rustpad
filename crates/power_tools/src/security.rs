//! Pillar D — Security, Forensic Redaction, Metadata Sanitization & Atomic I/O (`v3.0.0`).
//!
//! 100% air-gapped, pure-Rust document sanitization engine:
//! - Permanent `.docx` text & PII redaction across body, tables, headers/footers, comments,
//!   footnotes, endnotes, hyperlinks, and Track Changes history (preventing un-redaction).
//! - Deep metadata & hidden-layer cleaner (`core.xml`, `app.xml`, `custom.xml`, `customXml/*`,
//!   external `attachedTemplate` remote links, `<w:vanish/>` hidden text, micro-text).
//! - Secure atomic file writer (`0600` owner-only temp file + `fsync` + atomic rename + zeroize on error).

use doc_model::{Block, DocMetadata, Document, RevisionKind, RgbColor};
use docx_io::{load_docx_file, save_docx_bytes};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedactionOptions {
    pub terms: Vec<String>,
    pub redact_emails: bool,
    pub redact_api_secrets: bool,
    pub redact_phone_and_id_numbers: bool,
    pub purge_track_changes_history: bool,
    pub strip_hidden_runs: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedactionReport {
    pub body_replacements: usize,
    pub table_replacements: usize,
    pub header_footer_replacements: usize,
    pub comment_and_note_replacements: usize,
    pub purged_deleted_revisions: usize,
    pub stripped_hidden_runs: usize,
}

impl RedactionReport {
    pub fn total_hits(&self) -> usize {
        self.body_replacements
            + self.table_replacements
            + self.header_footer_replacements
            + self.comment_and_note_replacements
            + self.purged_deleted_revisions
            + self.stripped_hidden_runs
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetadataCleanOptions {
    pub strip_comments: bool,
    pub accept_and_purge_revisions: bool,
    pub strip_hidden_text: bool,
    pub strip_embedded_macros_and_ole: bool,
    pub strip_external_template_links: bool,
}

impl MetadataCleanOptions {
    pub fn strict_privacy() -> Self {
        Self {
            strip_comments: true,
            accept_and_purge_revisions: true,
            strip_hidden_text: true,
            strip_embedded_macros_and_ole: true,
            strip_external_template_links: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetadataCleanReport {
    pub cleared_metadata_fields: usize,
    pub removed_comments: usize,
    pub purged_revisions: usize,
    pub removed_hidden_runs: usize,
    pub removed_passthrough_parts: usize,
    pub removed_external_rels: usize,
}

/// Permanently redacts target strings and optional PII patterns across every surface of `doc`.
/// Redacted tokens are replaced by fixed-width block glyphs (`████████`) so original character
/// lengths and substrings cannot be recovered from saved XML streams.
pub fn redact_docx_document(doc: &mut Document, opts: &RedactionOptions) -> RedactionReport {
    let mut rep = RedactionReport::default();
    let active_terms: Vec<String> = opts
        .terms
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    if opts.strip_hidden_runs {
        rep.stripped_hidden_runs += strip_hidden_content(doc);
    }

    if opts.purge_track_changes_history {
        for block in &mut doc.blocks {
            if let Block::Paragraph(p) = block {
                let before = p.spans.len();
                p.spans
                    .retain(|s| !matches!(&s.revision, Some(r) if r.kind == RevisionKind::Deletion));
                rep.purged_deleted_revisions += before.saturating_sub(p.spans.len());
                for sp in &mut p.spans {
                    sp.revision = None;
                }
            }
        }
        doc.track_changes = false;
    }

    for block in &mut doc.blocks {
        match block {
            Block::Paragraph(p) => {
                for sp in &mut p.spans {
                    let (new_txt, hits) = scrub_text(&sp.text, &active_terms, opts);
                    if hits > 0 {
                        sp.text = new_txt;
                        sp.style.highlight = Some(RgbColor::INK_SLATE);
                        sp.style.hyperlink = None;
                        rep.body_replacements += hits;
                    } else if let Some(url) = &sp.style.hyperlink {
                        let (_, url_hits) = scrub_text(url, &active_terms, opts);
                        if url_hits > 0 {
                            sp.style.hyperlink = None;
                            rep.body_replacements += url_hits;
                        }
                    }
                }
            }
            Block::Table(t) => {
                for row in &mut t.rows {
                    for cell in row {
                        let (new_txt, hits) = scrub_text(&cell.text, &active_terms, opts);
                        if hits > 0 {
                            cell.text = new_txt;
                            rep.table_replacements += hits;
                        }
                    }
                }
            }
            Block::Image(img) => {
                let (new_cap, hits) = scrub_text(&img.caption, &active_terms, opts);
                if hits > 0 {
                    img.caption = new_cap;
                    rep.body_replacements += hits;
                }
            }
            Block::HorizontalRule | Block::PageBreak | Block::SectionBreak(_) => {}
        }
    }

    for field in [
        &mut doc.page_setup.header_text,
        &mut doc.page_setup.footer_text,
        &mut doc.page_setup.first_page_header,
        &mut doc.page_setup.first_page_footer,
    ] {
        let (new_txt, hits) = scrub_text(field, &active_terms, opts);
        if hits > 0 {
            *field = new_txt;
            rep.header_footer_replacements += hits;
        }
    }

    for c in &mut doc.comments {
        let (new_txt, hits) = scrub_text(&c.text, &active_terms, opts);
        if hits > 0 {
            c.text = new_txt;
            rep.comment_and_note_replacements += hits;
        }
    }
    for n in &mut doc.footnotes {
        let (new_txt, hits) = scrub_text(&n.text, &active_terms, opts);
        if hits > 0 {
            n.text = new_txt;
            rep.comment_and_note_replacements += hits;
        }
    }
    for n in &mut doc.endnotes {
        let (new_txt, hits) = scrub_text(&n.text, &active_terms, opts);
        if hits > 0 {
            n.text = new_txt;
            rep.comment_and_note_replacements += hits;
        }
    }

    rep
}

/// Redacts a `.docx` file on disk using atomic secure write semantics.
pub fn redact_docx_file(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    opts: &RedactionOptions,
) -> Result<RedactionReport, String> {
    let mut doc = load_docx_file(input).map_err(|e| e.0)?;
    let report = redact_docx_document(&mut doc, opts);
    let bytes = save_docx_bytes(&doc);
    write_bytes_secure_atomic(output, &bytes)?;
    Ok(report)
}

/// Strips `<w:vanish/>` hidden runs, sub-2pt micro-text runs, and white-on-white invisible runs.
pub fn strip_hidden_content(doc: &mut Document) -> usize {
    let mut removed = 0usize;
    for block in &mut doc.blocks {
        if let Block::Paragraph(p) = block {
            let before = p.spans.len();
            p.spans.retain(|sp| {
                let is_white_stealth = sp.style.color.r >= 250
                    && sp.style.color.g >= 250
                    && sp.style.color.b >= 250
                    && sp.style.highlight.is_none()
                    && !sp.text.trim().is_empty();
                let is_micro = sp.style.font_size_pt < 2.0 && !sp.text.trim().is_empty();
                !(sp.style.hidden || is_white_stealth || is_micro)
            });
            removed += before.saturating_sub(p.spans.len());
            p.normalize_spans();
        }
    }
    removed
}

/// Scrubs identifying metadata, revision authors, custom XML stores, and optional comments/macros.
pub fn clean_document_metadata(
    doc: &mut Document,
    opts: &MetadataCleanOptions,
) -> MetadataCleanReport {
    let mut rep = MetadataCleanReport::default();

    if !doc.author.is_empty() && doc.author != "Redacted" {
        doc.author.clear();
        rep.cleared_metadata_fields += 1;
    }
    let meta = &mut doc.metadata;
    for s in [
        &mut meta.subject,
        &mut meta.keywords,
        &mut meta.description,
        &mut meta.last_modified_by,
        &mut meta.company,
        &mut meta.created_iso,
        &mut meta.modified_iso,
    ] {
        if !s.is_empty() {
            s.clear();
            rep.cleared_metadata_fields += 1;
        }
    }
    if meta.revision_number > 0 {
        meta.revision_number = 0;
        rep.cleared_metadata_fields += 1;
    }
    rep.cleared_metadata_fields += meta.custom_properties.len();
    *meta = DocMetadata::default();

    // Remove `docProps/app.xml`, `docProps/custom.xml`, and `customXml/*` passthrough parts
    let strip_macros = opts.strip_embedded_macros_and_ole;
    let should_drop_part = |name: &str| -> bool {
        let lower = name.to_ascii_lowercase();
        lower == "docprops/app.xml"
            || lower == "docprops/custom.xml"
            || lower.starts_with("customxml/")
            || (strip_macros
                && (lower.ends_with("vbaproject.bin")
                    || lower.starts_with("word/embeddings/")
                    || lower.contains("activex")))
    };

    let drop_keys_vec: Vec<String> = doc
        .passthrough_parts
        .keys()
        .filter(|k| should_drop_part(k))
        .cloned()
        .collect();
    for k in drop_keys_vec {
        doc.passthrough_parts.remove(&k);
        rep.removed_passthrough_parts += 1;
    }
    let drop_keys_shared: Vec<String> = doc
        .passthrough_shared
        .keys()
        .filter(|k| should_drop_part(k))
        .cloned()
        .collect();
    for k in drop_keys_shared {
        doc.passthrough_shared.remove(&k);
        rep.removed_passthrough_parts += 1;
    }

    if strip_macros {
        doc.extra_relationships.retain(|r| {
            let is_ext = r
                .target_mode
                .as_deref()
                .is_some_and(|m| m.eq_ignore_ascii_case("external"));
            if is_ext {
                return true;
            }
            let cleaned = r.target.trim().trim_start_matches('/').replace('\\', "/");
            let full = if cleaned.starts_with("word/")
                || cleaned.starts_with("customXml/")
                || cleaned.starts_with("docProps/")
            {
                cleaned
            } else {
                format!("word/{cleaned}")
            };
            !should_drop_part(&full)
        });
    }

    if opts.strip_external_template_links {
        let before = doc.extra_relationships.len();
        doc.extra_relationships.retain(|r| {
            let is_ext = r
                .target_mode
                .as_deref()
                .is_some_and(|m| m.eq_ignore_ascii_case("external"));
            !is_ext
        });
        rep.removed_external_rels = before.saturating_sub(doc.extra_relationships.len());
    }

    if opts.strip_comments {
        rep.removed_comments = doc.comments.len();
        doc.comments.clear();
    } else {
        for c in &mut doc.comments {
            c.author = "Reviewer".into();
            c.date_iso.clear();
        }
    }

    for n in &mut doc.footnotes {
        n.author.clear();
    }
    for n in &mut doc.endnotes {
        n.author.clear();
    }

    if opts.accept_and_purge_revisions {
        for block in &mut doc.blocks {
            if let Block::Paragraph(p) = block {
                let before = p.spans.len();
                p.spans
                    .retain(|s| !matches!(&s.revision, Some(r) if r.kind == RevisionKind::Deletion));
                rep.purged_revisions += before.saturating_sub(p.spans.len());
                for sp in &mut p.spans {
                    if sp.revision.take().is_some() {
                        rep.purged_revisions += 1;
                    }
                }
                p.normalize_spans();
            }
        }
        doc.track_changes = false;
    }

    if opts.strip_hidden_text {
        rep.removed_hidden_runs = strip_hidden_content(doc);
    }

    rep
}

pub fn clean_docx_file_metadata(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    opts: &MetadataCleanOptions,
) -> Result<MetadataCleanReport, String> {
    let mut doc = load_docx_file(input).map_err(|e| e.0)?;
    let rep = clean_document_metadata(&mut doc, opts);
    let bytes = save_docx_bytes(&doc);
    write_bytes_secure_atomic(output, &bytes)?;
    Ok(rep)
}

/// Writes `bytes` to `dest` via a restricted (`0600` on Unix) temporary sibling file, `fsync`s,
/// and atomically renames into place. Overwrites temporary bytes with zeros if an I/O error occurs.
pub fn write_bytes_secure_atomic(dest: impl AsRef<Path>, bytes: &[u8]) -> Result<(), String> {
    let dest = dest.as_ref();
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let tmp_path = build_secure_temp_sibling(dest);
    let mut open_opts = OpenOptions::new();
    open_opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_opts.mode(0o600);
    }

    let mut file = open_opts
        .open(&tmp_path)
        .map_err(|e| format!("Failed to create secure temp file '{}': {e}", tmp_path.display()))?;

    if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = wipe_and_remove(&tmp_path, bytes.len().min(65_536));
        return Err(format!("Failed writing atomic file '{}': {e}", dest.display()));
    }
    drop(file);

    if let Err(e) = fs::rename(&tmp_path, dest) {
        let _ = wipe_and_remove(&tmp_path, bytes.len().min(65_536));
        return Err(format!(
            "Atomic rename '{}' -> '{}' failed: {e}",
            tmp_path.display(),
            dest.display()
        ));
    }
    Ok(())
}

fn wipe_and_remove(path: &Path, zero_len: usize) -> std::io::Result<()> {
    if let Ok(mut f) = OpenOptions::new().write(true).open(path) {
        let zeros = vec![0u8; zero_len.max(512)];
        let _ = f.write_all(&zeros);
        let _ = f.sync_all();
    }
    fs::remove_file(path)
}

fn build_secure_temp_sibling(dest: &Path) -> PathBuf {
    static TEMP_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let pid = std::process::id();
    let seq = TEMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stem = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let tmp_name = format!(".{stem}.rustpad-tmp-{pid}-{nanos:x}-{seq:x}");
    match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(tmp_name),
        _ => PathBuf::from(tmp_name),
    }
}

fn scrub_text(input: &str, terms: &[String], opts: &RedactionOptions) -> (String, usize) {
    if input.is_empty() {
        return (String::new(), 0);
    }
    let mut cur = input.to_string();
    let mut total_hits = 0usize;

    for term in terms {
        let (replaced, hits) = replace_case_insensitive(&cur, term, "████████");
        if hits > 0 {
            cur = replaced;
            total_hits += hits;
        }
    }

    if opts.redact_emails || opts.redact_api_secrets || opts.redact_phone_and_id_numbers {
        let mut rebuilt = String::with_capacity(cur.len());
        for token in cur.split_inclusive(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '<' | '>' | '"' | '\'')) {
            let trimmed_end = token.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '.' | '(' | ')' | '<' | '>' | '"' | '\''));
            let suffix = &token[trimmed_end.len()..];
            let core = trimmed_end;
            let should_mask = (opts.redact_emails && looks_like_email(core))
                || (opts.redact_api_secrets && looks_like_secret(core))
                || (opts.redact_phone_and_id_numbers && looks_like_id_number(core));
            if should_mask {
                rebuilt.push_str("████████");
                rebuilt.push_str(suffix);
                total_hits += 1;
            } else {
                rebuilt.push_str(token);
            }
        }
        cur = rebuilt;
    }

    (cur, total_hits)
}

fn looks_like_email(s: &str) -> bool {
    if let Some((local, domain)) = s.split_once('@') {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
    } else {
        false
    }
}

fn looks_like_secret(s: &str) -> bool {
    (s.starts_with("sk-") && s.len() >= 18)
        || (s.starts_with("ghp_") && s.len() >= 20)
        || (s.starts_with("AKIA") && s.len() >= 16)
}

fn looks_like_id_number(s: &str) -> bool {
    // Matches `XXX-XX-XXXX` SSN-style or 13..19 digit card/ID sequences
    let bytes = s.as_bytes();
    if bytes.len() == 11
        && bytes[3] == b'-'
        && bytes[6] == b'-'
        && bytes.iter().enumerate().all(|(i, b)| i == 3 || i == 6 || b.is_ascii_digit())
    {
        return true;
    }
    let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
    let allowed = s
        .chars()
        .all(|c| c.is_ascii_digit() || c == '-' || c == '+');
    allowed && (12..=19).contains(&digits)
}

fn replace_case_insensitive(haystack: &str, needle: &str, mask: &str) -> (String, usize) {
    if needle.is_empty() {
        return (haystack.to_string(), 0);
    }
    let lower_h = haystack.to_lowercase();
    let lower_n = needle.to_lowercase();
    if lower_h.len() != haystack.len() || lower_n.len() != needle.len() {
        // Unicode length shift fallback: exact case-sensitive match
        let hits = haystack.matches(needle).count();
        if hits == 0 {
            return (haystack.to_string(), 0);
        }
        return (haystack.replace(needle, mask), hits);
    }
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0usize;
    let mut hits = 0usize;
    while let Some(rel) = lower_h[cursor..].find(&lower_n) {
        let abs = cursor + rel;
        out.push_str(&haystack[cursor..abs]);
        out.push_str(mask);
        cursor = abs + lower_n.len();
        hits += 1;
    }
    out.push_str(&haystack[cursor..]);
    (out, hits)
}
