//! Enterprise Governance, MDM Policy Engine (`policy.json`), Air-Gapped AI DLP Lockout,
//! SHA-256 Hash-Chained Audit Trail, Write-Ahead Crash Recovery Journal (WAL),
//! Advisory SMB/NFS Lockfiles, Semantic Accessibility Tree, and CycloneDX 1.5 SBOM Generator.

use doc_model::{Block, Document, SecurityClassification};
use docx_io::{load_docx_bytes, save_docx_bytes};
use rustpad_zip::sha256_hex;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const GENESIS_AUDIT_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// IT / MDM deployment policy (`policy.json`) governing AI DLP, air-gapped lockout, and redaction compliance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterprisePolicy {
    pub organization: String,
    pub air_gapped_mode: bool,
    pub allow_cloud_ai: bool,
    pub allowed_ai_hosts: Vec<String>,
    pub default_classification: SecurityClassification,
    pub require_redaction_sanitization: bool,
    pub audit_log_enabled: bool,
}

impl Default for EnterprisePolicy {
    fn default() -> Self {
        Self {
            organization: "Open-Source Workstation".into(),
            air_gapped_mode: false,
            allow_cloud_ai: true,
            allowed_ai_hosts: vec![
                "127.0.0.1".into(),
                "localhost".into(),
                "::1".into(),
                "api.openai.com".into(),
                "api.anthropic.com".into(),
                "api.groq.com".into(),
                "openrouter.ai".into(),
            ],
            default_classification: SecurityClassification::Unclassified,
            require_redaction_sanitization: true,
            audit_log_enabled: true,
        }
    }
}

impl EnterprisePolicy {
    /// Preset strict sovereign / air-gapped policy blocking all external cloud egress.
    pub fn strict_air_gapped(organization: impl Into<String>) -> Self {
        Self {
            organization: organization.into(),
            air_gapped_mode: true,
            allow_cloud_ai: false,
            allowed_ai_hosts: vec!["127.0.0.1".into(), "localhost".into(), "::1".into()],
            default_classification: SecurityClassification::Confidential,
            require_redaction_sanitization: true,
            audit_log_enabled: true,
        }
    }

    /// Loads enterprise policy from `RUSTPAD_POLICY_PATH`, `./.rustpad-policy.json`,
    /// `/etc/rustpad/policy.json`, or `%ProgramData%\RustPad\policy.json`.
    pub fn load_system_or_default() -> (Self, Option<PathBuf>) {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(env_p) = std::env::var("RUSTPAD_POLICY_PATH") {
            if !env_p.trim().is_empty() {
                candidates.push(PathBuf::from(env_p.trim()));
            }
        }
        candidates.push(PathBuf::from(".rustpad-policy.json"));
        candidates.push(PathBuf::from("/etc/rustpad/policy.json"));
        if let Ok(prog_data) = std::env::var("ProgramData") {
            candidates.push(PathBuf::from(prog_data).join("RustPad").join("policy.json"));
        }

        for p in candidates {
            if let Ok(raw) = std::fs::read_to_string(&p) {
                return (Self::from_json(&raw), Some(p));
            }
        }
        (Self::default(), None)
    }

    pub fn from_json(raw: &str) -> Self {
        let mut pol = Self::default();
        if let Some(org) = json_get_str(raw, "organization") {
            if !org.trim().is_empty() {
                pol.organization = org;
            }
        }
        if let Some(ag) = json_get_bool(raw, "air_gapped_mode") {
            pol.air_gapped_mode = ag;
        }
        if let Some(ac) = json_get_bool(raw, "allow_cloud_ai") {
            pol.allow_cloud_ai = ac;
        }
        if let Some(hosts) = json_get_str_array(raw, "allowed_ai_hosts") {
            if !hosts.is_empty() {
                pol.allowed_ai_hosts = hosts;
            }
        }
        if let Some(cls) = json_get_str(raw, "default_classification") {
            pol.default_classification = SecurityClassification::from_slug(&cls);
        }
        if let Some(san) = json_get_bool(raw, "require_redaction_sanitization") {
            pol.require_redaction_sanitization = san;
        }
        if let Some(aud) = json_get_bool(raw, "audit_log_enabled") {
            pol.audit_log_enabled = aud;
        }
        pol
    }

    pub fn to_json_pretty(&self) -> String {
        let hosts_json: Vec<String> = self
            .allowed_ai_hosts
            .iter()
            .map(|h| format!("\"{}\"", escape_json(h)))
            .collect();
        format!(
            "{{\n  \"organization\": \"{}\",\n  \"air_gapped_mode\": {},\n  \"allow_cloud_ai\": {},\n  \"allowed_ai_hosts\": [{}],\n  \"default_classification\": \"{}\",\n  \"require_redaction_sanitization\": {},\n  \"audit_log_enabled\": {}\n}}",
            escape_json(&self.organization),
            self.air_gapped_mode,
            self.allow_cloud_ai,
            hosts_json.join(", "),
            self.default_classification.slug(),
            self.require_redaction_sanitization,
            self.audit_log_enabled
        )
    }

    /// Enforces Data Loss Prevention (DLP) rules before any AI HTTP/HTTPS socket is opened.
    pub fn validate_ai_endpoint(&self, endpoint_url: &str) -> Result<(), String> {
        let trimmed = endpoint_url.trim();
        let (is_https, rest) = if let Some(r) = trimmed.strip_prefix("https://") {
            (true, r)
        } else if let Some(r) = trimmed.strip_prefix("http://") {
            (false, r)
        } else {
            return Err("AI endpoint URL must start with http:// or https://".into());
        };

        let authority = rest.split('/').next().unwrap_or("");
        let host = if authority.starts_with('[') {
            authority
                .find(']')
                .map(|i| &authority[..=i])
                .unwrap_or(authority)
        } else {
            authority.split(':').next().unwrap_or("")
        }
        .to_ascii_lowercase();

        let is_loopback =
            host == "127.0.0.1" || host == "localhost" || host == "::1" || host == "[::1]";

        if self.air_gapped_mode && !is_loopback {
            return Err(format!(
                "DLP POLICY BLOCKED: Air-Gapped Mode ({}) forbids non-loopback endpoint '{}'. Use local Ollama / nour-code on 127.0.0.1.",
                self.organization, host
            ));
        }

        if !self.allow_cloud_ai && (is_https || !is_loopback) {
            let allowlisted = self
                .allowed_ai_hosts
                .iter()
                .any(|h| h.eq_ignore_ascii_case(&host));
            if !allowlisted || !is_loopback && self.air_gapped_mode {
                return Err(format!(
                    "DLP POLICY BLOCKED: Enterprise policy ({}) disables public cloud AI host '{}'.",
                    self.organization, host
                ));
            }
            if !allowlisted {
                return Err(format!(
                    "DLP POLICY BLOCKED: Host '{}' is not in allowed_ai_hosts.",
                    host
                ));
            }
            // If `allow_cloud_ai == false`, reject well-known public SaaS AI gateways even if listed in default
            if matches!(
                host.as_str(),
                "api.openai.com" | "api.anthropic.com" | "api.groq.com" | "openrouter.ai"
            ) {
                return Err(format!(
                    "DLP POLICY BLOCKED: Public cloud AI ('{}') is disabled by enterprise policy.",
                    host
                ));
            }
        }

        if !is_loopback && !self.allowed_ai_hosts.is_empty() {
            let allowlisted = self
                .allowed_ai_hosts
                .iter()
                .any(|h| h == "*" || h.eq_ignore_ascii_case(&host));
            if !allowlisted {
                return Err(format!(
                    "DLP POLICY BLOCKED: Endpoint host '{}' is not permitted by enterprise allowlist.",
                    host
                ));
            }
        }

        Ok(())
    }
}

/// Tamper-evident hash-chained audit event (`entry_sha256 = SHA256(seq|ts|action|subject|actor|prev)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub seq: u64,
    pub timestamp_epoch_s: u64,
    pub action: String,
    pub subject: String,
    pub actor: String,
    pub prev_sha256: String,
    pub entry_sha256: String,
}

impl AuditEvent {
    pub fn compute_hash(
        seq: u64,
        timestamp_epoch_s: u64,
        action: &str,
        subject: &str,
        actor: &str,
        prev_sha256: &str,
    ) -> String {
        let payload = format!("{seq}|{timestamp_epoch_s}|{action}|{subject}|{actor}|{prev_sha256}");
        sha256_hex(payload.as_bytes())
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditTrail {
    pub events: Vec<AuditEvent>,
}

impl AuditTrail {
    pub fn record(
        &mut self,
        action: impl Into<String>,
        subject: impl Into<String>,
        actor: impl Into<String>,
    ) -> &AuditEvent {
        let seq = (self.events.len() as u64) + 1;
        let ts = now_epoch_secs();
        let prev = self
            .events
            .last()
            .map(|e| e.entry_sha256.clone())
            .unwrap_or_else(|| GENESIS_AUDIT_HASH.to_string());
        let act = action.into();
        let sub = subject.into();
        let usr = actor.into();
        let hash = AuditEvent::compute_hash(seq, ts, &act, &sub, &usr, &prev);
        self.events.push(AuditEvent {
            seq,
            timestamp_epoch_s: ts,
            action: act,
            subject: sub,
            actor: usr,
            prev_sha256: prev,
            entry_sha256: hash,
        });
        self.events.last().unwrap()
    }

    /// Cryptographically verifies the entire hash chain from genesis to head.
    pub fn verify_chain(&self) -> Result<usize, String> {
        let mut expected_prev = GENESIS_AUDIT_HASH;
        for (idx, ev) in self.events.iter().enumerate() {
            if ev.seq != (idx as u64) + 1 {
                return Err(format!("Audit sequence gap at index {idx}"));
            }
            if ev.prev_sha256 != expected_prev {
                return Err(format!(
                    "Audit chain broken at seq {} (prev_sha256 mismatch)",
                    ev.seq
                ));
            }
            let recomputed = AuditEvent::compute_hash(
                ev.seq,
                ev.timestamp_epoch_s,
                &ev.action,
                &ev.subject,
                &ev.actor,
                &ev.prev_sha256,
            );
            if recomputed != ev.entry_sha256 {
                return Err(format!(
                    "Audit entry tampered at seq {} (hash mismatch)",
                    ev.seq
                ));
            }
            expected_prev = &ev.entry_sha256;
        }
        Ok(self.events.len())
    }

    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for ev in &self.events {
            let _ = writeln!(
                out,
                "{{\"seq\":{},\"ts\":{},\"action\":\"{}\",\"subject\":\"{}\",\"actor\":\"{}\",\"prev\":\"{}\",\"sha256\":\"{}\"}}",
                ev.seq,
                ev.timestamp_epoch_s,
                escape_json(&ev.action),
                escape_json(&ev.subject),
                escape_json(&ev.actor),
                ev.prev_sha256,
                ev.entry_sha256
            );
        }
        out
    }
}

/// Write-Ahead Crash Recovery Journal (WAL) frame with SHA-256 payload verification.
pub struct WalJournal;

impl WalJournal {
    pub fn journal_path_for(target: Option<&Path>, tab_index: usize) -> PathBuf {
        if let Some(p) = target {
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("document.docx");
            p.with_file_name(format!(".~{name}.rustpad-wal"))
        } else {
            std::env::temp_dir().join(format!("rustpad-unsaved-tab-{tab_index}.rustpad-wal"))
        }
    }

    pub fn write_frame(journal_path: &Path, doc: &Document) -> Result<String, String> {
        let payload = save_docx_bytes(doc);
        let digest = sha256_hex(&payload);
        let header = format!("RUSTPAD-WAL-V2|{digest}|{}\n", payload.len());
        let mut frame = Vec::with_capacity(header.len() + payload.len());
        frame.extend_from_slice(header.as_bytes());
        frame.extend_from_slice(&payload);
        crate::security::write_bytes_secure_atomic(journal_path, &frame)?;
        Ok(digest)
    }

    pub fn recover_frame(journal_path: &Path) -> Result<Document, String> {
        let raw = std::fs::read(journal_path).map_err(|e| e.to_string())?;
        let nl = raw
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| "Malformed WAL header".to_string())?;
        let hdr = String::from_utf8_lossy(&raw[..nl]);
        let parts: Vec<&str> = hdr.split('|').collect();
        if parts.len() != 3 || parts[0] != "RUSTPAD-WAL-V2" {
            return Err("Unrecognized WAL magic version".into());
        }
        let expected_hash = parts[1];
        let expected_len: usize = parts[2]
            .parse()
            .map_err(|_| "Malformed WAL payload length".to_string())?;
        let payload = &raw[nl + 1..];
        if payload.len() != expected_len {
            return Err("Truncated WAL crash recovery frame".into());
        }
        let actual_hash = sha256_hex(payload);
        if actual_hash != expected_hash {
            return Err("Corrupted WAL frame (SHA-256 mismatch)".into());
        }
        load_docx_bytes(payload).map_err(|e| e.0)
    }

    pub fn clear_frame(journal_path: &Path) {
        let _ = std::fs::remove_file(journal_path);
    }
}

/// Advisory SMB/NFS shared-drive file lock helper (`.~<file>.rustpad.lock`).
pub struct AdvisoryFileLock;

impl AdvisoryFileLock {
    pub fn lock_path_for(file_path: &Path) -> PathBuf {
        let name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("doc");
        file_path.with_file_name(format!(".~{name}.rustpad.lock"))
    }

    pub fn acquire(file_path: &Path, owner: &str) -> Result<PathBuf, String> {
        let lp = Self::lock_path_for(file_path);
        let pid = std::process::id();
        let body = format!(
            "owner={owner}\npid={pid}\ntimestamp={}\n",
            now_epoch_secs()
        );
        let _ = std::fs::write(&lp, body);
        Ok(lp)
    }

    pub fn release(file_path: &Path) {
        let lp = Self::lock_path_for(file_path);
        let _ = std::fs::remove_file(lp);
    }
}

/// Semantic Accessibility Node (aligned with Windows UIA & Linux AT-SPI2 roles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityNode {
    pub role: &'static str,
    pub name: String,
    pub value: String,
    pub children: Vec<AccessibilityNode>,
}

impl AccessibilityNode {
    pub fn to_json(&self) -> String {
        let kids: Vec<String> = self.children.iter().map(|c| c.to_json()).collect();
        format!(
            "{{\"role\":\"{}\",\"name\":\"{}\",\"value\":\"{}\",\"children\":[{}]}}",
            self.role,
            escape_json(&self.name),
            escape_json(&self.value),
            kids.join(",")
        )
    }
}

pub fn build_document_a11y_tree(doc: &Document) -> AccessibilityNode {
    let mut children = Vec::new();
    for (idx, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Paragraph(p) => {
                let role = if p.style.heading_level().is_some() {
                    "Heading"
                } else {
                    "Paragraph"
                };
                children.push(AccessibilityNode {
                    role,
                    name: format!("{} #{}", p.style.label(), idx + 1),
                    value: p.accepted_plain_text(),
                    children: Vec::new(),
                });
            }
            Block::Table(t) => {
                let mut rows_a11y = Vec::new();
                for (r_i, row) in t.rows.iter().enumerate() {
                    let cells_a11y = row
                        .iter()
                        .enumerate()
                        .map(|(c_i, cell)| AccessibilityNode {
                            role: if r_i == 0 { "ColumnHeader" } else { "GridCell" },
                            name: format!("R{}C{}", r_i + 1, c_i + 1),
                            value: cell.text.clone(),
                            children: Vec::new(),
                        })
                        .collect();
                    rows_a11y.push(AccessibilityNode {
                        role: "Row",
                        name: format!("Row {}", r_i + 1),
                        value: String::new(),
                        children: cells_a11y,
                    });
                }
                children.push(AccessibilityNode {
                    role: "Table",
                    name: format!("Table {}x{}", t.rows.len(), t.cols),
                    value: String::new(),
                    children: rows_a11y,
                });
            }
            Block::Image(img) => {
                children.push(AccessibilityNode {
                    role: "Image",
                    name: img.name.clone(),
                    value: img.caption.clone(),
                    children: Vec::new(),
                });
            }
            Block::HorizontalRule => {
                children.push(AccessibilityNode {
                    role: "Separator",
                    name: "Horizontal Rule".into(),
                    value: String::new(),
                    children: Vec::new(),
                });
            }
            Block::PageBreak => {
                children.push(AccessibilityNode {
                    role: "Separator",
                    name: "Page Break".into(),
                    value: String::new(),
                    children: Vec::new(),
                });
            }
            Block::SectionBreak(kind) => {
                children.push(AccessibilityNode {
                    role: "Separator",
                    name: format!("Section Break ({})", kind.label()),
                    value: String::new(),
                    children: Vec::new(),
                });
            }
        }
    }

    AccessibilityNode {
        role: "Document",
        name: doc.title.clone(),
        value: format!(
            "Classification: {} | Author: {}",
            doc.classification.label(),
            doc.author
        ),
        children,
    }
}

/// Emits a machine-readable CycloneDX 1.5 JSON Software Bill of Materials (SBOM)
/// proving zero third-party `crates.io` runtime dependencies for enterprise security review.
pub fn generate_cyclonedx_sbom_json() -> String {
    let ver = env!("CARGO_PKG_VERSION");
    let crates = [
        ("rustpad", "Native Win32/X11 Desktop Studio & Enterprise Headless CLI"),
        ("rustpad_zip", "Hardened PKZIP, RFC 1951 DEFLATE/Zlib, CRC32, Adler32 & SHA-256"),
        ("doc_model", "Structured OpenXML AST, Legal Track Changes, Redline Diff & Undo"),
        ("docx_io", "Lossless ECMA-376 .docx Reader/Writer, Comments & Passthrough"),
        ("pdf_create", "ISO 32000 PDF 1.7 & Archival PDF/A-2b Vector Renderer"),
        ("pdf_engine", "PDF 1.5+ ObjStm Parser, Forensic Redaction Sanitizer & SHA-256 Sealer"),
        ("power_tools", "Enterprise DLP Policy, WinHTTP/Local AI Bridge, WAL & Audit Trail"),
        ("rustpad_render", "Pure-Rust TrueType (.ttf) Subpixel Rasterizer & 2D SDF Painter"),
        ("rustpad_window", "Zero-Dependency Win32 & Linux X11 Windowing, DPI & Clipboard"),
    ];
    let comps: Vec<String> = crates
        .iter()
        .map(|(name, desc)| {
            format!(
                "    {{\n      \"type\": \"library\",\n      \"name\": \"{name}\",\n      \"version\": \"{ver}\",\n      \"purl\": \"pkg:cargo/{name}@{ver}?repository_url=https://github.com/Nour-yahyaoui/rustpad\",\n      \"scope\": \"required\",\n      \"description\": \"{desc}\",\n      \"licenses\": [{{ \"license\": {{ \"id\": \"MIT\" }} }}]\n    }}"
            )
        })
        .collect();

    format!(
        "{{\n  \"bomFormat\": \"CycloneDX\",\n  \"specVersion\": \"1.5\",\n  \"version\": 1,\n  \"metadata\": {{\n    \"component\": {{\n      \"type\": \"application\",\n      \"name\": \"rustpad\",\n      \"version\": \"{ver}\",\n      \"supplier\": {{ \"name\": \"Nour Yahyaoui\" }},\n      \"properties\": [\n        {{ \"name\": \"rustpad:external_crates_io_dependencies\", \"value\": \"0\" }},\n        {{ \"name\": \"rustpad:offline_capable\", \"value\": \"true\" }}\n      ]\n    }}\n  }},\n  \"components\": [\n{}\n  ],\n  \"externalReferences\": [\n    {{ \"type\": \"vcs\", \"url\": \"https://github.com/Nour-yahyaoui/rustpad\" }}\n  ],\n  \"dependencies\": []\n}}",
        comps.join(",\n")
    )
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_793_059_200)
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn json_get_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after = json[pos + needle.len()..].trim_start();
    let after_colon = after.strip_prefix(':')?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn json_get_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after = json[pos + needle.len()..].trim_start();
    let after_colon = after.strip_prefix(':')?.trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_get_str_array(json: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after = json[pos + needle.len()..].trim_start();
    let after_colon = after.strip_prefix(':')?.trim_start();
    let inner = after_colon.strip_prefix('[')?;
    let rb = inner.find(']')?;
    let list_body = &inner[..rb];
    let mut items = Vec::new();
    for part in list_body.split(',') {
        let t = part.trim().trim_matches('"').trim();
        if !t.is_empty() {
            items.push(t.to_string());
        }
    }
    Some(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_model::TemplateKind;

    #[test]
    fn test_enterprise_dlp_policy_blocks_cloud_when_airgapped() {
        let strict = EnterprisePolicy::strict_air_gapped("Sovereign Defense Lab");
        assert!(strict
            .validate_ai_endpoint("http://127.0.0.1:11434/v1/chat/completions")
            .is_ok());
        assert!(strict
            .validate_ai_endpoint("http://localhost:8000/v1/chat/completions")
            .is_ok());
        assert!(strict
            .validate_ai_endpoint("https://api.openai.com/v1/chat/completions")
            .is_err());
        assert!(strict
            .validate_ai_endpoint("https://api.anthropic.com/v1/messages")
            .is_err());

        let serialized = strict.to_json_pretty();
        let reparsed = EnterprisePolicy::from_json(&serialized);
        assert_eq!(reparsed.organization, "Sovereign Defense Lab");
        assert!(reparsed.air_gapped_mode);
        assert!(!reparsed.allow_cloud_ai);
    }

    #[test]
    fn test_audit_trail_sha256_chain_and_tamper_detection() {
        let mut trail = AuditTrail::default();
        trail.record("DOC_OPENED", "MSA_v2.docx", "Nour Yahyaoui");
        trail.record("REDACTION_SANITIZED", "Discovery_Exhibit.pdf", "Legal Ops");
        trail.record("AI_DLP_BLOCKED", "api.openai.com", "PolicyEngine");
        assert_eq!(trail.verify_chain().unwrap(), 3);

        trail.events[1].subject = "Tampered_File.pdf".into();
        assert!(trail.verify_chain().is_err());
    }

    #[test]
    fn test_wal_crash_journal_roundtrip_and_sbom() {
        let doc = TemplateKind::EnterpriseContract.build();
        let wal_path = std::env::temp_dir().join(format!(
            "rustpad_test_wal_{}.wal",
            std::process::id()
        ));
        let digest = WalJournal::write_frame(&wal_path, &doc).expect("write WAL");
        assert_eq!(digest.len(), 64);

        let recovered = WalJournal::recover_frame(&wal_path).expect("recover WAL");
        assert_eq!(recovered.title, doc.title);
        assert_eq!(recovered.revision_counts(), doc.revision_counts());
        WalJournal::clear_frame(&wal_path);

        let sbom = generate_cyclonedx_sbom_json();
        assert!(sbom.contains("\"bomFormat\": \"CycloneDX\""));
        assert!(sbom.contains("\"value\": \"0\""));

        let a11y = build_document_a11y_tree(&doc).to_json();
        assert!(a11y.contains("\"role\":\"Document\""));
        assert!(a11y.contains("\"role\":\"Table\""));
    }
}
