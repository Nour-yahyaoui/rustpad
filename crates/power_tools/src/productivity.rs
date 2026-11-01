//! Pillars E & F — Productivity Diagnostics, Multi-Surface Search, Quality Audit & Batch Engine.

use crate::converter::{word_to_html, word_to_markdown};
use crate::security::{
    clean_docx_file_metadata, redact_docx_file, write_bytes_secure_atomic, MetadataCleanOptions,
    RedactionOptions,
};
use doc_model::{Block, Document};
use docx_io::{inspect_security_and_hidden, load_docx_bytes, validate_docx_bytes};
use pdf_create::{export_document_to_pdf_bytes, PdfCreateOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchSurface {
    Paragraph,
    TableCell,
    HeaderFooter,
    Comment,
    Footnote,
    Endnote,
}

impl SearchSurface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Paragraph => "Body Paragraph",
            Self::TableCell => "Table Cell",
            Self::HeaderFooter => "Header / Footer",
            Self::Comment => "Comment",
            Self::Footnote => "Footnote",
            Self::Endnote => "Endnote",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub surface: SearchSurface,
    pub block_index: usize,
    pub byte_offset: usize,
    pub location_label: String,
    pub snippet: String,
}

/// Fast multi-surface full-document search across body blocks, tables, headers/footers,
/// comments, footnotes, and endnotes.
pub fn search_document(doc: &Document, query: &str, case_sensitive: bool) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    if query.trim().is_empty() {
        return hits;
    }
    let needle = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    for (b_idx, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Paragraph(p) => {
                let text = p.plain_text();
                collect_text_hits(
                    &text,
                    &needle,
                    case_sensitive,
                    SearchSurface::Paragraph,
                    b_idx,
                    &format!("Block #{} ({})", b_idx + 1, p.style.label()),
                    &mut hits,
                );
            }
            Block::Table(t) => {
                for (r_i, row) in t.rows.iter().enumerate() {
                    for (c_i, cell) in row.iter().enumerate() {
                        collect_text_hits(
                            &cell.text,
                            &needle,
                            case_sensitive,
                            SearchSurface::TableCell,
                            b_idx,
                            &format!("Table #{} R{}C{}", b_idx + 1, r_i + 1, c_i + 1),
                            &mut hits,
                        );
                    }
                }
            }
            Block::Image(img) => {
                collect_text_hits(
                    &img.caption,
                    &needle,
                    case_sensitive,
                    SearchSurface::Paragraph,
                    b_idx,
                    &format!("Figure #{} Caption", b_idx + 1),
                    &mut hits,
                );
            }
            Block::HorizontalRule | Block::PageBreak | Block::SectionBreak(_) => {}
        }
    }

    for (label, text) in [
        ("Running Header", doc.page_setup.header_text.as_str()),
        ("Running Footer", doc.page_setup.footer_text.as_str()),
        ("First-Page Header", doc.page_setup.first_page_header.as_str()),
        ("First-Page Footer", doc.page_setup.first_page_footer.as_str()),
    ] {
        collect_text_hits(
            text,
            &needle,
            case_sensitive,
            SearchSurface::HeaderFooter,
            0,
            label,
            &mut hits,
        );
    }

    for c in &doc.comments {
        collect_text_hits(
            &c.text,
            &needle,
            case_sensitive,
            SearchSurface::Comment,
            c.block_index,
            &format!("Comment #{} ({})", c.id, c.author),
            &mut hits,
        );
    }
    for n in &doc.footnotes {
        collect_text_hits(
            &n.text,
            &needle,
            case_sensitive,
            SearchSurface::Footnote,
            0,
            &format!("Footnote [{}]", n.id),
            &mut hits,
        );
    }
    for n in &doc.endnotes {
        collect_text_hits(
            &n.text,
            &needle,
            case_sensitive,
            SearchSurface::Endnote,
            0,
            &format!("Endnote [e{}]", n.id),
            &mut hits,
        );
    }

    hits
}

fn collect_text_hits(
    text: &str,
    needle: &str,
    case_sensitive: bool,
    surface: SearchSurface,
    block_index: usize,
    location_label: &str,
    out: &mut Vec<SearchHit>,
) {
    if text.is_empty() {
        return;
    }
    let hay = if case_sensitive {
        text.to_string()
    } else {
        text.to_lowercase()
    };
    let mut start = 0usize;
    while let Some(rel) = hay[start..].find(needle) {
        let pos = start + rel;
        let snippet = make_snippet(text, pos, needle.len());
        out.push(SearchHit {
            surface,
            block_index,
            byte_offset: pos,
            location_label: location_label.to_string(),
            snippet,
        });
        start = pos + needle.len().max(1);
        if start >= hay.len() {
            break;
        }
    }
}

fn make_snippet(text: &str, byte_pos: usize, match_len: usize) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let center_idx = chars
        .iter()
        .position(|(b, _)| *b >= byte_pos)
        .unwrap_or(0);
    let from = center_idx.saturating_sub(28);
    let to = (center_idx + match_len.max(8) + 28).min(chars.len());
    let mut s = String::new();
    if from > 0 {
        s.push('…');
    }
    for (_, ch) in &chars[from..to] {
        s.push(if *ch == '\n' { ' ' } else { *ch });
    }
    if to < chars.len() {
        s.push('…');
    }
    s
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateParagraphGroup {
    pub normalized_fingerprint: String,
    pub sample_text: String,
    pub block_indices: Vec<usize>,
}

/// Detects repeated substantive paragraphs (`>= min_chars`, default `24`) normalized across
/// case and whitespace runs.
pub fn detect_duplicate_paragraphs(
    doc: &Document,
    min_chars: usize,
) -> Vec<DuplicateParagraphGroup> {
    let threshold = min_chars.max(12);
    let mut buckets: BTreeMap<String, (String, Vec<usize>)> = BTreeMap::new();

    for (idx, block) in doc.blocks.iter().enumerate() {
        if let Block::Paragraph(p) = block {
            let raw = p.accepted_plain_text();
            let trimmed = raw.trim();
            if trimmed.chars().count() < threshold {
                continue;
            }
            let norm = normalize_for_dedup(trimmed);
            if norm.len() < threshold {
                continue;
            }
            let entry = buckets
                .entry(norm)
                .or_insert_with(|| (trimmed.chars().take(120).collect(), Vec::new()));
            entry.1.push(idx);
        }
    }

    let mut groups: Vec<DuplicateParagraphGroup> = buckets
        .into_iter()
        .filter(|(_, (_, indices))| indices.len() >= 2)
        .map(|(fp, (sample, indices))| DuplicateParagraphGroup {
            normalized_fingerprint: fp,
            sample_text: sample,
            block_indices: indices,
        })
        .collect();
    groups.sort_by(|a, b| b.block_indices.len().cmp(&a.block_indices.len()));
    groups
}

fn normalize_for_dedup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
            prev_space = false;
        }
    }
    out.trim().to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokenLinkFinding {
    pub block_index: usize,
    pub link_target: String,
    pub anchor_text: String,
    pub reason: String,
}

/// Audits hyperlinks and internal `#anchor` targets for broken anchors, unsafe schemes,
/// malformed hosts, whitespace corruption, or missing relative files.
pub fn detect_broken_links(doc: &Document, base_dir: Option<&Path>) -> Vec<BrokenLinkFinding> {
    let mut findings = Vec::new();
    let mut valid_anchors = BTreeSet::new();

    for block in &doc.blocks {
        if let Block::Paragraph(p) = block {
            if p.style.heading_level().is_some() {
                let slug = heading_anchor_slug(&p.accepted_plain_text());
                if !slug.is_empty() {
                    valid_anchors.insert(slug);
                }
            }
        }
    }

    for (b_idx, block) in doc.blocks.iter().enumerate() {
        if let Block::Paragraph(p) = block {
            for sp in &p.spans {
                if let Some(url) = &sp.style.hyperlink {
                    if let Some(reason) = validate_link_target(url, &valid_anchors, base_dir) {
                        findings.push(BrokenLinkFinding {
                            block_index: b_idx,
                            link_target: url.clone(),
                            anchor_text: sp.text.trim().to_string(),
                            reason,
                        });
                    }
                }
            }
        }
    }

    findings
}

fn heading_anchor_slug(text: &str) -> String {
    let mut slug = String::new();
    for ch in text.trim().chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                slug.push(lc);
            }
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn validate_link_target(
    url: &str,
    valid_anchors: &BTreeSet<String>,
    base_dir: Option<&Path>,
) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Some("Empty hyperlink target".into());
    }
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Some("Unescaped whitespace inside URL".into());
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("vbscript:") || lower.starts_with("data:") {
        return Some("Blocked unsafe script/data URI scheme".into());
    }
    if let Some(anchor) = trimmed.strip_prefix('#') {
        let norm = heading_anchor_slug(anchor);
        if norm.is_empty() {
            return Some("Empty internal '#anchor' target".into());
        }
        if !valid_anchors.is_empty() && !valid_anchors.contains(&norm) {
            return Some(format!(
                "Internal anchor '#{anchor}' does not match any document heading"
            ));
        }
        return None;
    }
    if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        let host = rest.split('/').next().unwrap_or("");
        if host.is_empty() || (!host.contains('.') && host != "localhost" && !host.starts_with("127.")) {
            return Some(format!("Malformed HTTP(S) host '{host}'"));
        }
        return None;
    }
    if lower.starts_with("mailto:") {
        let addr = &trimmed[7..];
        if !addr.contains('@') || !addr.contains('.') {
            return Some("Malformed mailto: address".into());
        }
        return None;
    }
    if let Some(dir) = base_dir {
        if !lower.contains("://") {
            let candidate = dir.join(trimmed);
            if !candidate.exists() {
                return Some(format!(
                    "Relative file target '{}' does not exist",
                    candidate.display()
                ));
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub enum BatchOperation {
    ValidateAndInspect,
    CleanMetadata(MetadataCleanOptions),
    Redact(RedactionOptions),
    ConvertToPdf,
    ConvertToMarkdown,
    ConvertToHtml { dark_mode: bool },
}

#[derive(Clone, Debug, Default)]
pub struct BatchJobSummary {
    pub scanned_files: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub security_warnings: usize,
    pub item_logs: Vec<String>,
}

/// Offline batch processor for a directory of `.docx` files.
pub fn run_batch_docx_job(
    input_dir: impl AsRef<Path>,
    output_dir: Option<&Path>,
    op: &BatchOperation,
) -> Result<BatchJobSummary, String> {
    let in_dir = input_dir.as_ref();
    if !in_dir.exists() {
        return Err(format!("Batch input directory '{}' does not exist", in_dir.display()));
    }
    if let Some(out) = output_dir {
        fs::create_dir_all(out).map_err(|e| e.to_string())?;
    }

    let mut docx_files: Vec<PathBuf> = Vec::new();
    collect_docx_files(in_dir, &mut docx_files)?;
    docx_files.sort();

    let mut summary = BatchJobSummary {
        scanned_files: docx_files.len(),
        ..BatchJobSummary::default()
    };

    for path in docx_files {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let target_dir = output_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| path.parent().map(PathBuf::from).unwrap_or_default());

        let outcome: Result<String, String> = match op {
            BatchOperation::ValidateAndInspect => match fs::read(&path) {
                Ok(raw) => {
                    let v = validate_docx_bytes(&raw);
                    if !v.is_valid {
                        Err(format!("{}: INVALID ({} errors)", path.display(), v.error_count()))
                    } else if let Ok(doc) = load_docx_bytes(&raw) {
                        let sec = inspect_security_and_hidden(&doc, Some(&raw));
                        summary.security_warnings += sec.findings.len();
                        Ok(format!(
                            "{}: VALID ({} entries, {} security note(s))",
                            path.display(),
                            v.archive_entries,
                            sec.findings.len()
                        ))
                    } else {
                        Err(format!("{}: Failed AST load", path.display()))
                    }
                }
                Err(e) => Err(format!("{}: {e}", path.display())),
            },
            BatchOperation::CleanMetadata(opts) => {
                let out_file = target_dir.join(format!("{stem}.cleaned.docx"));
                clean_docx_file_metadata(&path, &out_file, opts).map(|rep| {
                    format!(
                        "{} -> {} (cleared {} metadata fields, {} hidden runs)",
                        path.display(),
                        out_file.display(),
                        rep.cleared_metadata_fields,
                        rep.removed_hidden_runs
                    )
                })
            }
            BatchOperation::Redact(opts) => {
                let out_file = target_dir.join(format!("{stem}.redacted.docx"));
                redact_docx_file(&path, &out_file, opts).map(|rep| {
                    format!(
                        "{} -> {} ({} redactions)",
                        path.display(),
                        out_file.display(),
                        rep.total_hits()
                    )
                })
            }
            BatchOperation::ConvertToPdf => match fs::read(&path).and_then(|b| {
                load_docx_bytes(&b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.0))
            }) {
                Ok(doc) => {
                    let out_file = target_dir.join(format!("{stem}.pdf"));
                    let bytes = export_document_to_pdf_bytes(&doc, &PdfCreateOptions::default());
                    write_bytes_secure_atomic(&out_file, &bytes)
                        .map(|_| format!("{} -> {}", path.display(), out_file.display()))
                }
                Err(e) => Err(format!("{}: {e}", path.display())),
            },
            BatchOperation::ConvertToMarkdown => match fs::read(&path).and_then(|b| {
                load_docx_bytes(&b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.0))
            }) {
                Ok(doc) => {
                    let out_file = target_dir.join(format!("{stem}.md"));
                    let md = word_to_markdown(&doc);
                    write_bytes_secure_atomic(&out_file, md.as_bytes())
                        .map(|_| format!("{} -> {}", path.display(), out_file.display()))
                }
                Err(e) => Err(format!("{}: {e}", path.display())),
            },
            BatchOperation::ConvertToHtml { dark_mode } => match fs::read(&path).and_then(|b| {
                load_docx_bytes(&b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.0))
            }) {
                Ok(doc) => {
                    let out_file = target_dir.join(format!("{stem}.html"));
                    let html = word_to_html(&doc, *dark_mode);
                    write_bytes_secure_atomic(&out_file, html.as_bytes())
                        .map(|_| format!("{} -> {}", path.display(), out_file.display()))
                }
                Err(e) => Err(format!("{}: {e}", path.display())),
            },
        };

        match outcome {
            Ok(msg) => {
                summary.succeeded += 1;
                summary.item_logs.push(msg);
            }
            Err(err) => {
                summary.failed += 1;
                summary.item_logs.push(err);
            }
        }
    }

    Ok(summary)
}

fn collect_docx_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let rd = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_docx_files(&p, out)?;
        } else if p
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("docx"))
        {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("~$") || name.starts_with('.') {
                    continue;
                }
            }
            out.push(p);
        }
    }
    Ok(())
}
