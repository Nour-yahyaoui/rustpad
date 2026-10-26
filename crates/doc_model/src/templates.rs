//! Built-in Starter Templates for RustPad (Welcome Guide, Resume, Report, Invoice, Letter).

use crate::{
    Alignment, Block, Document, FontFamily, ListKind, PageSetup, Paragraph, ParagraphStyle,
    RgbColor, SpanStyle, TableBlock, TableCell, TextSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Blank,
    WelcomeGuide,
    ModernResume,
    AcademicReport,
    BusinessInvoice,
    FormalLetter,
}

impl TemplateKind {
    pub const ALL: [TemplateKind; 6] = [
        Self::Blank,
        Self::WelcomeGuide,
        Self::ModernResume,
        Self::AcademicReport,
        Self::BusinessInvoice,
        Self::FormalLetter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blank => "Blank Document",
            Self::WelcomeGuide => "Welcome to RustPad",
            Self::ModernResume => "Modern Resume / CV",
            Self::AcademicReport => "Technical Report",
            Self::BusinessInvoice => "Business Invoice",
            Self::FormalLetter => "Executive Letter",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Blank => "Clean A4 sheet ready for writing",
            Self::WelcomeGuide => "Interactive tour of Word, PDF & Power Tools",
            Self::ModernResume => "Crisp 1-page developer & engineer CV",
            Self::AcademicReport => "Structured sections, metrics table & references",
            Self::BusinessInvoice => "Itemized billing table with summary totals",
            Self::FormalLetter => "Polished cover or business correspondence",
        }
    }

    pub fn build(self) -> Document {
        match self {
            Self::Blank => Document::blank(),
            Self::WelcomeGuide => build_welcome_guide(),
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
    setup.header_text = "RustPad — Pure-Rust Word & PDF Desktop Studio".into();
    setup.footer_text = "Created by Nour Yahyaoui · github.com/Nour-yahyaoui/rustpad".into();

    let mut intro = Paragraph::new(ParagraphStyle::Normal, "");
    intro.spans = vec![
        TextSpan::plain("Welcome to "),
        TextSpan::styled(
            "RustPad",
            SpanStyle {
                bold: true,
                color: RgbColor::COBALT,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(
            " — your 100% pure-Rust native desktop studio for creating & editing ",
        ),
        TextSpan::styled(
            "Microsoft Word (.docx)",
            SpanStyle {
                bold: true,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(" and "),
        TextSpan::styled(
            "PDF (.pdf)",
            SpanStyle {
                bold: true,
                color: RgbColor::CORAL,
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(" files offline with a calm "),
        TextSpan::styled(
            "Paper & Daylight",
            SpanStyle {
                highlight: Some(RgbColor::AMBER_HIGHLIGHT),
                ..SpanStyle::default()
            },
        ),
        TextSpan::plain(" light interface."),
    ];

    let mut table = TableBlock::new(4, 3);
    table.rows[0] = vec![
        TableCell::header("Studio Pillar"),
        TableCell::header("Key Capabilities"),
        TableCell::header("Quick Shortcut"),
    ];
    table.rows[1] = vec![
        TableCell::plain("Word Studio (.docx)"),
        TableCell::plain("Paginated A4/Letter sheets, styles, tables, headers & spellcheck"),
        TableCell::plain("Ctrl+N / Ctrl+S"),
    ];
    table.rows[2] = vec![
        TableCell::plain("PDF Studio (.pdf)"),
        TableCell::plain("Create PDF, edit text blocks, highlight, sign, merge, split & redact"),
        TableCell::plain("Ctrl+E / Ctrl+M"),
    ];
    table.rows[3] = vec![
        TableCell::plain("Power Tools"),
        TableCell::plain("DOCX <-> PDF converter, offline OCR, Split View & Time-Machine"),
        TableCell::plain("Ctrl+K"),
    ];

    Document {
        title: "Welcome to RustPad".into(),
        author: "Nour Yahyaoui".into(),
        page_setup: setup,
        blocks: vec![
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Title,
                "RustPad Document & PDF Studio",
            )),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Subtitle,
                "Fast · Lightweight · 100% Native Rust · Designed by Nour Yahyaoui",
            )),
            Block::Paragraph(intro),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading1,
                "1. Core Capabilities at Your Fingertips",
            )),
            bullet("Open, create, format, and save standards-compliant Microsoft Word (.docx) files."),
            bullet("Export print-ready vector PDFs in one click or open any existing PDF to edit, merge, split, sign, or compress."),
            bullet("Press Ctrl+K anywhere to launch the instant Command Palette across 35+ tools."),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "2. Quick Feature Matrix",
            )),
            Block::Table(table),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "3. Try Editing Right Now",
            )),
            checklist("Click anywhere on this A4 sheet and start typing", true),
            checklist("Use the top bar to switch styles, colors, lists, or insert a table", false),
            checklist("Click 'Export PDF' in the top-right to generate a crisp vector PDF", false),
        ],
        comments: Vec::new(),
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
        TableCell::plain("Rust, C++, Native GUI, High-Performance File Parsers"),
    ];
    skills.rows[2] = vec![
        TableCell::plain("Software Architecture"),
        TableCell::plain("OpenXML (.docx), PDF 1.7 Engines, Modular Release Pipelines"),
    ];

    Document {
        title: "Modern Resume".into(),
        author: "Nour Yahyaoui".into(),
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
                "Systems-minded Software Engineer building fast, clean, user-friendly open-source desktop software with native performance and thoughtful UI craftsmanship.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "Technical Skills")),
            Block::Table(skills),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "Featured Open-Source Work")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Heading2,
                "RustPad — Pure-Rust Word & PDF Desktop Studio",
            )),
            bullet("Architected a zero-bloat multi-crate Rust workspace for Word (.docx) and PDF (.pdf) creation & editing."),
            bullet("Designed the Paper & Daylight light theme for effortless 3-click document workflows."),
        ],
        comments: Vec::new(),
    }
}

fn build_academic_report() -> Document {
    let mut results = TableBlock::new(3, 3);
    results.rows[0] = vec![
        TableCell::header("Metric"),
        TableCell::header("Legacy Office Suite"),
        TableCell::header("RustPad Native"),
    ];
    results.rows[1] = vec![
        TableCell::plain("Cold Startup Latency"),
        TableCell::plain("2,400 ms"),
        TableCell::plain("< 100 ms"),
    ];
    results.rows[2] = vec![
        TableCell::plain("Idle Memory Footprint"),
        TableCell::plain("340 MB"),
        TableCell::plain("~32 MB"),
    ];

    Document {
        title: "Technical Architecture Report".into(),
        author: "Nour Yahyaoui".into(),
        page_setup: PageSetup {
            header_text: "Technical Report — Document Systems".into(),
            footer_text: "Confidentiality: Public Open-Source".into(),
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
                "Modern document workflows demand instant startup, lossless structural editing, and zero cloud lock-in across both .docx and .pdf formats.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "2. Benchmark Summary")),
            Block::Table(results),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "3. Conclusion")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "Unifying paginated OpenXML editing with vector PDF generation in pure Rust yields an order-of-magnitude reduction in memory and installer size.",
            )),
        ],
        comments: Vec::new(),
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
        TableCell::plain("Desktop Architecture & UI Engineering"),
        TableCell::plain("1"),
        TableCell::plain("$1,200.00"),
        TableCell::plain("$1,200.00"),
    ];
    items.rows[2] = vec![
        TableCell::plain("Document Engine & PDF Automation"),
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
        "Invoice #INV-2026-001 · Date: October 26, 2026",
    );
    right_date.alignment = Alignment::Right;

    Document {
        title: "Invoice INV-2026-001".into(),
        author: "Nour Yahyaoui".into(),
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
        page_setup: PageSetup::default(),
        blocks: vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "October 26, 2026")),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Dear Colleague,")),
            Block::Paragraph(Paragraph::new(
                ParagraphStyle::Normal,
                "I am pleased to share this proposal created directly inside RustPad. Every paragraph, margin, and typographic detail exports cleanly to both Microsoft Word (.docx) and print-ready vector PDF.",
            )),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Warm regards,")),
            Block::Paragraph(mono_sig),
        ],
        comments: Vec::new(),
    }
}
