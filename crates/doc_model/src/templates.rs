//! Built-in Starter & Enterprise Templates for RustPad v2.0
//! (Welcome Guide, Enterprise MSA Redline, Modern Resume, Technical Report, Invoice, Letter).

use crate::{
    Alignment, Block, DocComment, Document, FontFamily, ListKind, PageSetup, Paragraph,
    ParagraphStyle, RevisionKind, RevisionMeta, RgbColor, SecurityClassification, SpanStyle,
    TableBlock, TableCell, TextSpan,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Blank,
    WelcomeGuide,
    EnterpriseContract,
    ModernResume,
    AcademicReport,
    BusinessInvoice,
    FormalLetter,
}

impl TemplateKind {
    pub const ALL: [TemplateKind; 7] = [
        Self::Blank,
        Self::WelcomeGuide,
        Self::EnterpriseContract,
        Self::ModernResume,
        Self::AcademicReport,
        Self::BusinessInvoice,
        Self::FormalLetter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blank => "Blank Document",
            Self::WelcomeGuide => "Welcome to RustPad v3.0",
            Self::EnterpriseContract => "Enterprise MSA Redline",
            Self::ModernResume => "Modern Resume / CV",
            Self::AcademicReport => "Technical Architecture Spec",
            Self::BusinessInvoice => "Enterprise Invoice",
            Self::FormalLetter => "Executive Correspondence",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Blank => "Clean A4 sheet ready for writing",
            Self::WelcomeGuide => "Tour of Offline Enterprise Word, PDF & DLP AI",
            Self::EnterpriseContract => "Live ECMA-376 Track Changes (w:ins/w:del) & legal comments",
            Self::ModernResume => "Crisp 1-page systems & software engineer CV",
            Self::AcademicReport => "Structured sections, benchmark matrix & governance",
            Self::BusinessInvoice => "Itemized billing schedule with summary totals",
            Self::FormalLetter => "Formal executive or legal cover correspondence",
        }
    }

    pub fn build(self) -> Document {
        match self {
            Self::Blank => Document::blank(),
            Self::WelcomeGuide => build_welcome_guide(),
            Self::EnterpriseContract => build_enterprise_contract(),
            Self::ModernResume => build_modern_resume(),
            Self::AcademicReport => build_academic_report(),
            Self::BusinessInvoice => build_invoice(),
            Self::FormalLetter => build_letter(),
        }
    }
}

fn bullet(text: &str) -> Block {
    let mut p = Paragraph::new(ParagraphStyle::Normal, text);
    p.list_kind = ListKind::Bullet;
    Block::Paragraph(p)
}

fn checklist(text: &str, done: bool) -> Block {
    let mut p = Paragraph::new(ParagraphStyle::Normal, text);
    p.list_kind = ListKind::Checklist(done);
    Block::Paragraph(p)
}

fn build_welcome_guide() -> Document {
    let mut setup = PageSetup::default();
    setup.header_text = "RustPad v3.0 — Lossless OOXML Intelligence, Dark Mode & 60–120Hz Suite".into();
    setup.footer_text = "Open-Source MIT · github.com/Nour-yahyaoui/rustpad".into();

    let mut intro = Paragraph::new(ParagraphStyle::Normal, "");
    intro.spans = vec![
        TextSpan::plain("Welcome to "),
        TextSpan::styled(
            "RustPad v3.0",
            SpanStyle {
                bold: true,
                color: RgbColor::COBALT,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(
            " — crafted from the ground up so ultra-smooth 60–120Hz responsiveness and human UX are the #1 feature across ",
        ),
        TextSpan::styled(
            "ECMA-376 Word (.docx)",
            SpanStyle {
                bold: true,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(" with live range selection, floating popover formatting & Track Changes, and "),
        TextSpan::styled(
            "ISO 32000 PDF (.pdf)",
            SpanStyle {
                bold: true,
                color: RgbColor::CORAL,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(" with forensic redaction sanitization & SHA-256 tamper seals."),
    ];

    let mut table = TableBlock::new(5, 3);
    table.rows[0] = vec![
        TableCell::header("Core Pillar"),
        TableCell::header("v3.0 OOXML Intelligence, Dark Mode & Effortless Studio"),
        TableCell::header("Shortcut / CLI"),
    ];
    table.rows[1] = vec![
        TableCell::plain("Effortless Table Creator"),
        TableCell::plain("Visual 8×8 grid picker, 1-click Invoice/Tracker presets, /table 4x5, | A | B | & TSV paste"),
        TableCell::plain("Ctrl+Shift+T"),
    ];
    table.rows[2] = vec![
        TableCell::plain("Smooth .docx Figures"),
        TableCell::plain("Native PNG/JPEG/BMP decoder, Image button, /image, Ctrl+V screenshot paste & corner resize"),
        TableCell::plain("Ctrl+Shift+I"),
    ];
    table.rows[3] = vec![
        TableCell::plain("OOXML & Forensic PDF"),
        TableCell::plain("Lossless .docx passthrough, per-clause redlines, PDF ObjStm + glyph-scrub redaction"),
        TableCell::plain("Ctrl+Shift+E"),
    ];
    table.rows[4] = vec![
        TableCell::plain("Air-Gapped AI + DLP Policy"),
        TableCell::plain("Selection AI Polish via Ollama / nour-code + MDM policy.json cloud lockout"),
        TableCell::plain("Ctrl+J / policy"),
    ];

    Document {
        title: "Welcome to RustPad".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Internal,
        track_changes: false,
        page_setup: setup,
        blocks: vec![
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Title,
                "RustPad Document & PDF Studio",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Subtitle,
                "v3.0 Lossless OOXML Intelligence · Native Dark Mode · 60–120Hz UX · Zero Dependencies",
            )),
            Block::Paragraph(intro),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "1. Enterprise Offline Capabilities",
            )),
            bullet("Lossless OpenXML (.docx) passthrough preserves corporate themes, customXml schemas, Track Changes (<w:ins>/<w:del>), and threaded review comments."),
            bullet("Forensic PDF sanitization strips underlying glyph streams inside redaction bounds, exports archival PDF/A-2b, and embeds verifiable SHA-256 document seals."),
            bullet("IT MDM policy.json governance locks AI tasks strictly to on-prem / localhost endpoints (Ollama, nour-code, vLLM) with hash-chained audit logging."),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "2. Subsystem Capability Matrix",
            )),
            Block::Table(table),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "3. Interactive Studio Verification",
            )),
            checklist("Toggle Track Changes (Ctrl+Shift+E) to inspect live legal redline insertions & deletions", true),
            checklist("Click inside any table cell to wrap multi-line text or drag column dividers", true),
            checklist("Open the AI Dock (Ctrl+J) or Governance Policy inspector (Ctrl+K -> Policy)", false),
        ],
        comments: vec![DocComment {
            id: 1,
            block_index: 4,
            author: "Security Architecture".into(),
            date_iso: "2026-10-27T09:30:00Z".into(),
            text: "Verified 0 crates.io runtime dependencies via `rustpad sbom` CycloneDX audit.".into(),
            resolved: false,
        }],
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}

fn build_enterprise_contract() -> Document {
    let mut setup = PageSetup::default();
    setup.header_text = "MASTER SERVICES & DATA SOVEREIGNTY AGREEMENT — CONFIDENTIAL".into();
    setup.footer_text = "Legal Redline Draft v2.4 · ECMA-376 Tracked Revisions Active".into();
    setup.watermark = Some("CONFIDENTIAL".into());

    let mut clause_payment = Paragraph::new(ParagraphStyle::Normal, "");
    clause_payment.spans = vec![
        TextSpan::plain("2.1 Invoicing & Settlement. Client shall remit undisputed fees within "),
        TextSpan::tracked(
            "forty-five (45) calendar",
            SpanStyle::default(),
            RevisionMeta {
                id: 1,
                kind: RevisionKind::Deletion,
                author: "General Counsel".into(),
                date_iso: "2026-10-27T11:15:00Z".into(),
            },
        ),
        TextSpan::plain(" "),
        TextSpan::tracked(
            "twenty (20) business",
            SpanStyle {
                bold: true,
                ..SpanStyle::default()
            },
            RevisionMeta {
                id: 2,
                kind: RevisionKind::Insertion,
                author: "General Counsel".into(),
                date_iso: "2026-10-27T11:15:00Z".into(),
            },
        ),
        TextSpan::plain(" days following receipt of a cryptographically sealed PDF/A-2b invoice."),
    ];

    let mut clause_dlp = Paragraph::new(ParagraphStyle::Normal, "");
    clause_dlp.spans = vec![
        TextSpan::plain("3.2 Sovereign Processing. All automated document analysis shall execute "),
        TextSpan::tracked(
            "exclusively on Customer-controlled on-premises hardware (127.0.0.1 / Sovereign VPC)",
            SpanStyle {
                bold: true,
                ..SpanStyle::default()
            },
            RevisionMeta {
                id: 3,
                kind: RevisionKind::Insertion,
                author: "CISO Office".into(),
                date_iso: "2026-10-27T11:42:00Z".into(),
            },
        ),
        TextSpan::plain(" with external public cloud AI endpoints disabled via MDM policy.json."),
    ];

    let mut sla_table = TableBlock::new(4, 3);
    sla_table.rows[0] = vec![
        TableCell::header("Control Domain"),
        TableCell::header("Contractual Standard"),
        TableCell::header("Verification Artifact"),
    ];
    sla_table.rows[1] = vec![
        TableCell::plain("Supply-Chain Integrity"),
        TableCell::plain("Zero third-party crates.io runtime dependencies"),
        TableCell::plain("CycloneDX 1.5 JSON SBOM"),
    ];
    sla_table.rows[2] = vec![
        TableCell::plain("Redaction Sanitization"),
        TableCell::plain("Physical removal of underlying PDF glyph streams"),
        TableCell::plain("Forensic Sanitize Report"),
    ];
    sla_table.rows[3] = vec![
        TableCell::plain("AI Data Loss Prevention"),
        TableCell::plain("Hard-locked to local Ollama / vLLM endpoints"),
        TableCell::plain("SHA-256 Chained Audit Log"),
    ];

    Document {
        title: "Enterprise MSA Redline".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Confidential,
        track_changes: true,
        page_setup: setup,
        blocks: vec![
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Title,
                "Master Services & Data Sovereignty Agreement",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Subtitle,
                "Counterparty Review Draft · Track Changes Enabled (Accept / Reject in Review Bar)",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "1. Scope & Definitions",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "This Agreement governs the offline deployment of RustPad Enterprise Suite across air-gapped engineering, legal, and regulated financial workstations.",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "2. Commercial Settlement (Redlined)",
            )),
            Block::Paragraph(clause_payment),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "3. AI Data Loss Prevention & Auditability",
            )),
            Block::Paragraph(clause_dlp),
            Block::Table(sla_table),
        ],
        comments: vec![
            DocComment {
                id: 1,
                block_index: 5,
                author: "General Counsel".into(),
                date_iso: "2026-10-27T11:16:00Z".into(),
                text: "Tightened net payment window from 45 calendar to 20 business days per Q4 treasury policy.".into(),
                resolved: false,
            },
            DocComment {
                id: 2,
                block_index: 7,
                author: "CISO Office".into(),
                date_iso: "2026-10-27T11:44:00Z".into(),
                text: "Confirmed `allow_cloud_ai: false` in enterprise policy.json blocks external egress.".into(),
                resolved: false,
            },
        ],
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}

fn build_modern_resume() -> Document {
    let mut skills = TableBlock::new(3, 2);
    skills.rows[0] = vec![
        TableCell::header("Domain"),
        TableCell::header("Technologies & Strengths"),
    ];
    skills.rows[1] = vec![
        TableCell::plain("Systems & Desktop"),
        TableCell::plain("Rust, C++, Native Win32/X11 GUI, Zero-Dependency Parsers"),
    ];
    skills.rows[2] = vec![
        TableCell::plain("Document & Security Engines"),
        TableCell::plain("ECMA-376 OpenXML (.docx), ISO 32000 PDF 1.7, SHA-256 Seals, Local AI"),
    ];

    Document {
        title: "Modern Resume".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Unclassified,
        track_changes: false,
        page_setup: PageSetup {
            header_text: "CURRICULUM VITAE".into(),
            footer_text: "nourryahyaoui@gmail.com · github.com/Nour-yahyaoui".into(),
            ..PageSetup::default()
        },
        blocks: vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Title, "Nour Yahyaoui")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Subtitle,
                "Software Engineer & Open-Source Author · github.com/Nour-yahyaoui · nourryahyaoui@gmail.com",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "Profile Summary")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "Systems-minded Software Engineer building fast, auditable, offline-first open-source desktop software with native OS performance and disciplined UI craftsmanship.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "Technical Skills")),
            Block::Table(skills),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "Featured Open-Source Work")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "RustPad v2.0 — Enterprise Offline Word & PDF Desktop Suite",
            )),
            bullet("Architected a 9-crate zero-dependency Rust workspace for lossless OpenXML (.docx) redlining and forensic PDF (.pdf) sanitization."),
            bullet("Engineered MDM DLP policy governance, SHA-256 document sealing, and local/cloud AI orchestration."),
        ],
        comments: Vec::new(),
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}

fn build_academic_report() -> Document {
    let mut results = TableBlock::new(4, 3);
    results.rows[0] = vec![
        TableCell::header("Metric"),
        TableCell::header("Legacy Electron / Office Suite"),
        TableCell::header("RustPad v2.0 Native"),
    ];
    results.rows[1] = vec![
        TableCell::plain("Cold Startup Latency"),
        TableCell::plain("2,400 ms"),
        TableCell::plain("< 50 ms"),
    ];
    results.rows[2] = vec![
        TableCell::plain("Idle Memory Footprint"),
        TableCell::plain("340 MB"),
        TableCell::plain("~28 MB"),
    ];
    results.rows[3] = vec![
        TableCell::plain("Third-Party Runtime Crates"),
        TableCell::plain("400+ transitive packages"),
        TableCell::plain("0 crates.io deps"),
    ];

    Document {
        title: "Technical Architecture Report".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Internal,
        track_changes: false,
        page_setup: PageSetup {
            header_text: "Technical Report — Document Systems".into(),
            footer_text: "Classification: Public Open-Source Architecture".into(),
            ..PageSetup::default()
        },
        blocks: vec![
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Title,
                "High-Speed Native Document Processing",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Subtitle,
                "Author: Nour Yahyaoui · October 2026",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "1. Abstract")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Quote,
                "Regulated enterprise workflows require instant native startup, lossless OpenXML round-tripping, forensic PDF sanitization, and zero supply-chain exposure.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "2. Benchmark Summary")),
            Block::Table(results),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "3. Conclusion")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "Unifying paginated OpenXML redlining with forensic vector PDF generation in pure Rust yields an order-of-magnitude advantage in auditability, memory footprint, and cold-start speed.",
            )),
        ],
        comments: Vec::new(),
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}

fn build_invoice() -> Document {
    let mut items = TableBlock::new(4, 4);
    items.rows[0] = vec![
        TableCell::header("Description"),
        TableCell::header("Qty"),
        TableCell::header("Unit Price"),
        TableCell::header("Amount"),
    ];
    items.rows[1] = vec![
        TableCell::plain("Enterprise Desktop Architecture & Security Engineering"),
        TableCell::plain("1"),
        TableCell::plain("$1,200.00"),
        TableCell::plain("$1,200.00"),
    ];
    items.rows[2] = vec![
        TableCell::plain("OpenXML Redline & Forensic PDF Engine"),
        TableCell::plain("1"),
        TableCell::plain("$850.00"),
        TableCell::plain("$850.00"),
    ];
    items.rows[3] = vec![
        TableCell::header("Total Due"),
        TableCell::plain(""),
        TableCell::plain(""),
        TableCell::header("$2,050.00"),
    ];

    let mut right_date = Paragraph::new(
        ParagraphStyle::Normal,
        "Invoice #INV-2026-001 · Date: October 27, 2026",
    );
    right_date.alignment = Alignment::Right;

    Document {
        title: "Invoice INV-2026-001".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Confidential,
        track_changes: false,
        page_setup: PageSetup::default(),
        blocks: vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Title, "INVOICE")),
            Block::Paragraph(right_date),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "Bill To: Client Organization",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "From: Nour Yahyaoui (nourryahyaoui@gmail.com)",
            )),
            Block::Table(items),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "Payment Terms: Net 14 days. Thank you for your partnership!",
            )),
        ],
        comments: Vec::new(),
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}

fn build_letter() -> Document {
    let mut mono_sig = Paragraph::new(ParagraphStyle::Normal, "Nour Yahyaoui");
    mono_sig.spans[0].style.bold = true;
    mono_sig.spans[0].style.font_family = FontFamily::Serif;
    mono_sig.spans[0].style.font_size_pt = 14.0;

    Document {
        title: "Executive Letter".into(),
        author: "Nour Yahyaoui".into(),
        classification: SecurityClassification::Unclassified,
        track_changes: false,
        page_setup: PageSetup::default(),
        blocks: vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "October 27, 2026")),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Dear Colleague,")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "I am pleased to share this proposal authored directly inside RustPad v2.0. Every paragraph, redline revision, table, and margin exports cleanly to both ECMA-376 Word (.docx) and archival vector PDF.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Warm regards,")),
            Block::Paragraph(mono_sig),
        ],
        comments: Vec::new(),
        passthrough_parts: BTreeMap::new(),
        ..Document::blank()
    }
}
