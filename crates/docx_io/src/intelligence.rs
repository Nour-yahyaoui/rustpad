//! RustPad v3.0.0 OOXML Intelligence, Validation, Corruption Repair, Formatting Consistency,
//! and Hidden Content / Suspicious Metadata Forensic Engine.

use crate::reader::load_docx_bytes;
use crate::writer::save_docx_bytes;
use crate::xml_lite::{parse_xml, XmlChild, XmlNode};
use crate::DocxError;
use doc_model::{Block, Document, Paragraph, ParagraphStyle};
use rustpad_zip::{salvage_corrupt_zip_entries, LazyZipArchive};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

impl IssueSeverity {
    pub fn badge(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    pub severity: IssueSeverity,
    pub code: &'static str,
    pub part: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct DocxValidationReport {
    pub is_valid: bool,
    pub archive_entries: usize,
    pub findings: Vec<ValidationFinding>,
}

impl DocxValidationReport {
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == IssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == IssueSeverity::Warning)
            .count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DocxRepairReport {
    pub salvaged_entries: usize,
    pub repaired_xml_parts: usize,
    pub recovered_paragraphs: usize,
    pub recovered_images: usize,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyFinding {
    pub block_index: usize,
    pub category: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    pub severity: IssueSeverity,
    pub category: &'static str,
    pub location: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityInspectionReport {
    pub hidden_runs_count: usize,
    pub camouflaged_runs_count: usize,
    pub micro_font_runs_count: usize,
    pub external_links_count: usize,
    pub has_vba_macros: bool,
    pub has_embedded_ole: bool,
    pub has_external_template_injection: bool,
    pub tracked_revisions_count: usize,
    pub comments_count: usize,
    pub findings: Vec<SecurityFinding>,
}

/// Performs deep structural, PKZIP/CRC-32, OPC package, XML well-formedness, and relationship
/// integrity validation on `.docx` bytes.
pub fn validate_docx_file(path: impl AsRef<Path>) -> Result<DocxValidationReport, DocxError> {
    let bytes = std::fs::read(path).map_err(|e| DocxError(e.to_string()))?;
    Ok(validate_docx_bytes(&bytes))
}

pub fn validate_docx_bytes(bytes: &[u8]) -> DocxValidationReport {
    let mut findings = Vec::new();
    let archive = match LazyZipArchive::from_bytes(bytes) {
        Ok(a) => a,
        Err(e) => {
            findings.push(ValidationFinding {
                severity: IssueSeverity::Error,
                code: "ZIP_CORRUPT",
                part: "<archive>".into(),
                message: format!("PKZIP container failed integrity check: {}", e.0),
            });
            return DocxValidationReport {
                is_valid: false,
                archive_entries: 0,
                findings,
            };
        }
    };

    let names: BTreeSet<String> = archive.file_names().iter().cloned().collect();
    let archive_entries = names.len();

    // 1. Verify CRC-32 & DEFLATE stream health across all entries
    for crc_err in archive.verify_all_entries() {
        findings.push(ValidationFinding {
            severity: IssueSeverity::Error,
            code: "ZIP_STREAM_CRC",
            part: "<stream>".into(),
            message: crc_err,
        });
    }

    // 2. Verify required ECMA-376 / OPC parts
    for required in ["[Content_Types].xml", "_rels/.rels", "word/document.xml"] {
        if !names.contains(required) {
            findings.push(ValidationFinding {
                severity: IssueSeverity::Error,
                code: "OPC_MISSING_PART",
                part: required.into(),
                message: format!("Missing mandatory OpenXML package part '{required}'"),
            });
        }
    }

    // 3. Parse & validate all XML parts + check relationship targets
    let mut rel_targets: BTreeMap<String, (String, bool)> = BTreeMap::new();
    if let Ok(Some(rels_xml)) = archive.get_str("word/_rels/document.xml.rels") {
        match parse_xml(&rels_xml) {
            Some(rels_root) => {
                for rel in rels_root.children_named("Relationship") {
                    if let (Some(id), Some(target)) = (rel.attr("Id"), rel.attr("Target")) {
                        let is_ext = rel
                            .attr("TargetMode")
                            .is_some_and(|m| m.eq_ignore_ascii_case("External"));
                        if rel_targets
                            .insert(id.to_string(), (target.to_string(), is_ext))
                            .is_some()
                        {
                            findings.push(ValidationFinding {
                                severity: IssueSeverity::Warning,
                                code: "REL_DUPLICATE_ID",
                                part: "word/_rels/document.xml.rels".into(),
                                message: format!("Duplicate Relationship Id='{id}'"),
                            });
                        }
                        if !is_ext {
                            let resolved = normalize_word_target(target);
                            if !resolved.is_empty() && !names.contains(&resolved) {
                                findings.push(ValidationFinding {
                                    severity: IssueSeverity::Error,
                                    code: "REL_DANGLING_TARGET",
                                    part: "word/_rels/document.xml.rels".into(),
                                    message: format!(
                                        "Relationship '{id}' references missing archive part '{resolved}'"
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            None => findings.push(ValidationFinding {
                severity: IssueSeverity::Error,
                code: "XML_MALFORMED",
                part: "word/_rels/document.xml.rels".into(),
                message: "Malformed XML in word/_rels/document.xml.rels".into(),
            }),
        }
    }

    for part_name in archive.file_names() {
        if part_name.ends_with(".xml") || part_name.ends_with(".rels") {
            if let Ok(Some(xml_str)) = archive.get_str(part_name) {
                if let Some(root) = parse_xml(&xml_str) {
                    if part_name == "word/document.xml" {
                        validate_document_xml_semantics(&root, &rel_targets, &mut findings);
                    }
                } else {
                    findings.push(ValidationFinding {
                        severity: IssueSeverity::Error,
                        code: "XML_MALFORMED",
                        part: part_name.clone(),
                        message: format!("Malformed or over-nested XML in '{part_name}'"),
                    });
                }
            }
        }
    }

    let is_valid = findings
        .iter()
        .all(|f| f.severity != IssueSeverity::Error);
    DocxValidationReport {
        is_valid,
        archive_entries,
        findings,
    }
}

fn normalize_word_target(raw: &str) -> String {
    let cleaned = raw.trim().replace('\\', "/");
    if cleaned.starts_with("http://")
        || cleaned.starts_with("https://")
        || cleaned.starts_with("mailto:")
        || cleaned.starts_with('#')
    {
        return String::new();
    }
    let mut stack: Vec<&str> = if cleaned.starts_with('/') || cleaned.starts_with("word/") {
        Vec::new()
    } else {
        vec!["word"]
    };
    for seg in cleaned.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            other => stack.push(other),
        }
    }
    stack.join("/")
}

fn validate_document_xml_semantics(
    root: &XmlNode,
    rels: &BTreeMap<String, (String, bool)>,
    findings: &mut Vec<ValidationFinding>,
) {
    if root.child("body").is_none() {
        findings.push(ValidationFinding {
            severity: IssueSeverity::Error,
            code: "DOC_MISSING_BODY",
            part: "word/document.xml".into(),
            message: "Missing <w:body> inside word/document.xml".into(),
        });
        return;
    }

    let mut open_comments: BTreeSet<String> = BTreeSet::new();
    let mut closed_comments: BTreeSet<String> = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node.local_name() {
            "blip" => {
                if let Some(rid) = node.attr("embed") {
                    if !rels.contains_key(rid) {
                        findings.push(ValidationFinding {
                            severity: IssueSeverity::Warning,
                            code: "DRAWING_UNRESOLVED_RID",
                            part: "word/document.xml".into(),
                            message: format!("Image <a:blip> references unknown r:embed='{rid}'"),
                        });
                    }
                }
            }
            "hyperlink" => {
                if let Some(rid) = node.attr("id") {
                    if !rels.contains_key(rid) {
                        findings.push(ValidationFinding {
                            severity: IssueSeverity::Warning,
                            code: "HYPERLINK_UNRESOLVED_RID",
                            part: "word/document.xml".into(),
                            message: format!("<w:hyperlink> references unknown r:id='{rid}'"),
                        });
                    }
                }
            }
            "commentRangeStart" => {
                if let Some(id) = node.attr("id") {
                    open_comments.insert(id.to_string());
                }
            }
            "commentRangeEnd" => {
                if let Some(id) = node.attr("id") {
                    closed_comments.insert(id.to_string());
                }
            }
            _ => {}
        }
        for c in &node.children {
            if let XmlChild::Element(el) = c {
                stack.push(el);
            }
        }
    }

    for id in open_comments.difference(&closed_comments) {
        findings.push(ValidationFinding {
            severity: IssueSeverity::Warning,
            code: "COMMENT_UNCLOSED_RANGE",
            part: "word/document.xml".into(),
            message: format!("commentRangeStart id='{id}' has no matching commentRangeEnd"),
        });
    }
}

/// Repairs damaged or truncated `.docx` files by salvaging local PKZIP entries, healing truncated
/// XML tag stacks, reconstructing missing OPC scaffolding, and re-serializing a clean `.docx`.
pub fn repair_docx_file(
    in_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<DocxRepairReport, DocxError> {
    let raw = std::fs::read(in_path).map_err(|e| DocxError(e.to_string()))?;
    let (repaired_bytes, report) = repair_docx_bytes(&raw)?;
    std::fs::write(out_path, repaired_bytes).map_err(|e| DocxError(e.to_string()))?;
    Ok(report)
}

pub fn repair_docx_bytes(bytes: &[u8]) -> Result<(Vec<u8>, DocxRepairReport), DocxError> {
    let mut report = DocxRepairReport::default();

    // Fast path if standard loader still succeeds after normal recovery
    if let Ok(doc) = load_docx_bytes(bytes) {
        report.recovered_paragraphs = doc
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::Paragraph(_)))
            .count();
        report.recovered_images = doc
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::Image(_)))
            .count();
        report
            .actions
            .push("Rebuilt clean OPC package & normalized Central Directory".into());
        return Ok((save_docx_bytes(&doc), report));
    }

    // Deep salvage path: walk Local File Headers (`PK\x03\x04`) even if EOCD/Central Directory is destroyed
    let salvaged = salvage_corrupt_zip_entries(bytes);
    if salvaged.is_empty() {
        return Err(DocxError(
            "Unable to salvage any PKZIP local entries from corrupted file".into(),
        ));
    }
    report.salvaged_entries = salvaged.len();
    report.actions.push(format!(
        "Salvaged {} raw entry stream(s) from Local File Headers",
        salvaged.len()
    ));

    let mut zw = rustpad_zip::ZipWriter::new();
    let mut has_doc_xml = false;
    for (name, data) in &salvaged {
        if name == "word/document.xml" {
            has_doc_xml = true;
            let raw_xml = String::from_utf8_lossy(data).into_owned();
            let healed = if parse_xml(&raw_xml).is_some() {
                raw_xml
            } else {
                report.repaired_xml_parts += 1;
                report
                    .actions
                    .push("Healed truncated/unclosed XML tags in word/document.xml".into());
                heal_truncated_word_xml(&raw_xml)
            };
            zw.add_file(name.clone(), healed.into_bytes());
        } else if name.ends_with(".xml") || name.ends_with(".rels") {
            let s = String::from_utf8_lossy(data);
            if parse_xml(&s).is_some() {
                zw.add_file(name.clone(), data.clone());
            } else {
                report.repaired_xml_parts += 1;
                report
                    .actions
                    .push(format!("Skipped irreparably truncated auxiliary XML '{name}'"));
            }
        } else {
            zw.add_stored(name.clone(), data.clone());
        }
    }

    if !has_doc_xml {
        return Err(DocxError(
            "Corrupted archive did not contain a recoverable word/document.xml stream".into(),
        ));
    }

    let rebuilt_zip = zw.finish();
    let doc = load_docx_bytes(&rebuilt_zip)?;
    report.recovered_paragraphs = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Paragraph(_)))
        .count();
    report.recovered_images = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Image(_)))
        .count();
    report
        .actions
        .push("Synthesized clean ECMA-376 package with verified CRC-32 & EOCD".into());
    Ok((save_docx_bytes(&doc), report))
}

fn heal_truncated_word_xml(raw: &str) -> String {
    // Trim any incomplete trailing `<...` tag fragment, then close open element tags LIFO
    let clean_prefix = match (raw.rfind('<'), raw.rfind('>')) {
        (Some(lt), Some(gt)) if lt > gt => &raw[..lt],
        (Some(lt), None) => &raw[..lt],
        _ => raw,
    };

    let mut open_stack: Vec<String> = Vec::new();
    let mut pos = 0usize;
    while let Some(rel_lt) = clean_prefix[pos..].find('<') {
        let lt = pos + rel_lt;
        let rest = &clean_prefix[lt..];
        let Some(gt_rel) = rest.find('>') else {
            break;
        };
        let inner = rest[1..gt_rel].trim();
        pos = lt + gt_rel + 1;
        if inner.is_empty()
            || inner.starts_with('?')
            || inner.starts_with('!')
            || inner.ends_with('/')
        {
            continue;
        }
        if let Some(close_raw) = inner.strip_prefix('/') {
            let cname = close_raw
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if let Some(idx) = open_stack.iter().rposition(|t| t == cname) {
                open_stack.truncate(idx);
            }
        } else {
            let oname = inner
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .trim();
            if !oname.is_empty() {
                open_stack.push(oname.to_string());
            }
        }
    }

    let mut out = String::with_capacity(clean_prefix.len() + open_stack.len() * 16 + 64);
    out.push_str(clean_prefix);
    while let Some(tag) = open_stack.pop() {
        out.push_str("</");
        out.push_str(&tag);
        out.push('>');
    }
    if !out.contains("body>") {
        out = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:body></w:document>"#,
            crate::xml_lite::escape_xml(clean_prefix)
        );
    }
    out
}

/// Inspects a `Document` for typography & formatting consistency problems:
/// skipped heading levels, manual "fake headings", font-family sprawl, erratic body font sizes,
/// and double/trailing whitespace.
pub fn inspect_formatting_consistency(doc: &Document) -> Vec<ConsistencyFinding> {
    let mut findings = Vec::new();
    let mut prev_heading_lvl: u8 = 0;
    let mut body_fonts = BTreeMap::new();
    let mut body_sizes = BTreeMap::new();

    for (idx, block) in doc.blocks.iter().enumerate() {
        let Block::Paragraph(p) = block else {
            continue;
        };
        let txt = p.accepted_plain_text();
        let trimmed = txt.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 1. Heading hierarchy skips (e.g. Heading 1 -> Heading 3 without Heading 2)
        if let Some(lvl) = p.style.heading_level() {
            if lvl > 1 && prev_heading_lvl > 0 && lvl > prev_heading_lvl + 1 {
                findings.push(ConsistencyFinding {
                    block_index: idx,
                    category: "HeadingHierarchy",
                    message: format!(
                        "Skipped heading level (jumped from H{} directly to H{} at '{}')",
                        prev_heading_lvl,
                        lvl,
                        truncate_preview(trimmed, 42)
                    ),
                });
            }
            if lvl >= 1 {
                prev_heading_lvl = lvl;
            }
        } else if p.style == ParagraphStyle::Normal {
            // 2. Detect "Fake Headings": short all-bold Normal paragraph instead of semantic Heading
            let all_bold = !p.spans.is_empty() && p.spans.iter().all(|s| s.style.bold);
            if all_bold
                && trimmed.chars().count() <= 68
                && !trimmed.ends_with('.')
                && p.list_kind == doc_model::ListKind::None
            {
                findings.push(ConsistencyFinding {
                    block_index: idx,
                    category: "FakeHeading",
                    message: format!(
                        "All-bold Normal paragraph '{}' looks like a manual heading (use Heading 1–3 for Outline/TOC)",
                        truncate_preview(trimmed, 42)
                    ),
                });
            }

            // Track body font family & size distribution
            for sp in &p.spans {
                if !sp.text.trim().is_empty() {
                    *body_fonts.entry(sp.style.font_family.label()).or_insert(0usize) += 1;
                    let sz_key = (sp.style.font_size_pt * 2.0).round() as u32;
                    *body_sizes.entry(sz_key).or_insert(0usize) += 1;
                }
            }
        }

        // 3. Double spaces inside prose
        if txt.contains("  ") && p.spans.iter().all(|s| s.style.font_family != doc_model::FontFamily::Mono) {
            findings.push(ConsistencyFinding {
                block_index: idx,
                category: "Whitespace",
                message: format!(
                    "Repeated consecutive spaces in paragraph #{}",
                    idx + 1
                ),
            });
        }
    }

    if body_fonts.len() >= 3 {
        findings.push(ConsistencyFinding {
            block_index: 0,
            category: "FontConsistency",
            message: format!(
                "Body paragraphs mix {} different font families ({})",
                body_fonts.len(),
                body_fonts
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if body_sizes.len() >= 4 {
        findings.push(ConsistencyFinding {
            block_index: 0,
            category: "SizeConsistency",
            message: format!(
                "Body paragraphs use {} different ad-hoc font sizes; standardize on Normal style",
                body_sizes.len()
            ),
        });
    }

    findings
}

/// Inspects a `Document` (and optional raw `.docx` archive bytes) for hidden `<w:vanish/>` text,
/// camouflaged white-on-white / microscopic text, author/company metadata residue, VBA macros,
/// embedded OLE objects, and external template-injection relationships.
pub fn inspect_security_and_hidden(
    doc: &Document,
    raw_docx: Option<&[u8]>,
) -> SecurityInspectionReport {
    let mut rep = SecurityInspectionReport::default();
    let (ins, del) = doc.revision_counts();
    rep.tracked_revisions_count = ins + del;
    rep.comments_count = doc.comments.len();

    if rep.tracked_revisions_count > 0 {
        rep.findings.push(SecurityFinding {
            severity: IssueSeverity::Warning,
            category: "RevisionHistory",
            location: "word/document.xml".into(),
            detail: format!(
                "{} unaccepted Track Changes ({} insertions, {} deletions — deleted text remains readable inside XML!)",
                rep.tracked_revisions_count, ins, del
            ),
        });
    }
    if rep.comments_count > 0 {
        rep.findings.push(SecurityFinding {
            severity: IssueSeverity::Info,
            category: "Comments",
            location: "word/comments.xml".into(),
            detail: format!(
                "{} review comment(s) with author attribution present",
                rep.comments_count
            ),
        });
    }

    if !doc.author.trim().is_empty() && doc.author != "Anonymous" {
        rep.findings.push(SecurityFinding {
            severity: IssueSeverity::Info,
            category: "MetadataIdentity",
            location: "docProps/core.xml".into(),
            detail: format!(
                "Creator metadata present: author='{}' lastModifiedBy='{}' company='{}'",
                doc.author, doc.metadata.last_modified_by, doc.metadata.company
            ),
        });
    }

    // Inspect spans for hidden (`w:vanish`), micro-font (`<= 2.5 pt`), camouflaged ink, and external links
    for (idx, block) in doc.blocks.iter().enumerate() {
        if let Block::Paragraph(p) = block {
            check_paragraph_hidden_and_links(idx, p, &mut rep);
        }
    }

    // Check preserved relationships for external template injection (`attachedTemplate` CVE vector!)
    for rel in &doc.extra_relationships {
        let is_ext = rel
            .target_mode
            .as_deref()
            .is_some_and(|m| m.eq_ignore_ascii_case("External"));
        if is_ext && rel.rel_type.contains("attachedTemplate") {
            rep.has_external_template_injection = true;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Error,
                category: "RemoteTemplateInjection",
                location: "word/_rels/document.xml.rels".into(),
                detail: format!(
                    "Suspicious external template relationship detected: Target='{}'",
                    rel.target
                ),
            });
        } else if is_ext {
            rep.external_links_count += 1;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Warning,
                category: "ExternalRelationship",
                location: format!("Relationship {}", rel.id),
                detail: format!("External relationship ({}) -> {}", rel.rel_type, rel.target),
            });
        }
    }

    // Check passthrough parts & optional raw archive for VBA macros / OLE / customXml
    let mut all_part_names: BTreeSet<String> = doc
        .passthrough_parts
        .keys()
        .chain(doc.passthrough_shared.keys())
        .cloned()
        .collect();
    if let Some(raw) = raw_docx {
        if let Ok(arch) = LazyZipArchive::from_bytes(raw) {
            for n in arch.file_names() {
                all_part_names.insert(n.clone());
            }
            if let Ok(Some(settings_rels)) = arch.get_str("word/_rels/settings.xml.rels") {
                if settings_rels.contains("attachedTemplate") && settings_rels.contains("External") {
                    rep.has_external_template_injection = true;
                    rep.findings.push(SecurityFinding {
                        severity: IssueSeverity::Error,
                        category: "RemoteTemplateInjection",
                        location: "word/_rels/settings.xml.rels".into(),
                        detail: "External attachedTemplate beacon detected in settings.xml.rels"
                            .into(),
                    });
                }
            }
        }
    }

    for part in &all_part_names {
        let lower = part.to_ascii_lowercase();
        if lower.ends_with("vbaproject.bin") || lower.contains("vba") {
            rep.has_vba_macros = true;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Error,
                category: "MacroPayload",
                location: part.clone(),
                detail: "Embedded VBA macro binary (vbaProject.bin) detected inside archive".into(),
            });
        } else if lower.starts_with("word/embeddings/") || lower.ends_with(".bin") {
            rep.has_embedded_ole = true;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Warning,
                category: "EmbeddedOLE",
                location: part.clone(),
                detail: format!("Embedded binary/OLE object '{}' preserved in package", part),
            });
        } else if lower.starts_with("customxml/") {
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Info,
                category: "CustomXml",
                location: part.clone(),
                detail: format!("Enterprise customXml payload '{}'", part),
            });
        }
    }

    rep
}

fn check_paragraph_hidden_and_links(
    idx: usize,
    p: &Paragraph,
    rep: &mut SecurityInspectionReport,
) {
    for sp in &p.spans {
        if sp.text.trim().is_empty() {
            continue;
        }
        if sp.style.hidden {
            rep.hidden_runs_count += 1;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Warning,
                category: "HiddenText",
                location: format!("Block #{}", idx + 1),
                detail: format!(
                    "Hidden <w:vanish/> run: '{}'",
                    truncate_preview(&sp.text, 48)
                ),
            });
        }
        if sp.style.font_size_pt <= 2.5 {
            rep.micro_font_runs_count += 1;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Warning,
                category: "MicroText",
                location: format!("Block #{}", idx + 1),
                detail: format!(
                    "Microscopic {:.1}pt run: '{}'",
                    sp.style.font_size_pt,
                    truncate_preview(&sp.text, 48)
                ),
            });
        }
        let c = sp.style.color;
        if c.r >= 242 && c.g >= 242 && c.b >= 242 && sp.style.highlight.is_none() {
            rep.camouflaged_runs_count += 1;
            rep.findings.push(SecurityFinding {
                severity: IssueSeverity::Warning,
                category: "CamouflagedText",
                location: format!("Block #{}", idx + 1),
                detail: format!(
                    "Near-white camouflaged ink (#{}) with no highlight: '{}'",
                    c.to_hex(),
                    truncate_preview(&sp.text, 48)
                ),
            });
        }
        if let Some(link) = &sp.style.hyperlink {
            if link.starts_with("http://") || link.starts_with("https://") || link.starts_with("file://") {
                rep.external_links_count += 1;
            }
        }
    }
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut out: String = trimmed.chars().take(max_chars).collect();
        out.push('…');
        out
    }
}
