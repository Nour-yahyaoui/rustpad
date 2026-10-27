//! RustPad v2.0 Enterprise (`rustpad`) — Offline-First Pure-Rust Native Word (`.docx`),
//! Forensic Vector PDF (`.pdf`), DLP-Governed Local AI Desktop Suite & Enterprise CLI.
//! Author: Nour Yahyaoui <nourryahyaoui@gmail.com> · https://github.com/Nour-yahyaoui/rustpad

pub mod commands;
pub mod pdf_view;
pub mod state;
pub mod ui_paint;
pub mod word_view;

use crate::commands::CommandId;
use crate::state::{ActiveModal, PdfTabState, RustPadApp, StudioTab, WordTabState};
use crate::ui_paint::render_app;
use doc_model::{Document, RgbColor, TemplateKind};
use docx_io::{load_docx_file, save_docx_file};
use pdf_create::{export_document_to_pdf_file, PdfCreateOptions};
use pdf_engine::{
    load_pdf_file, save_pdf_file, seal_pdf_bytes, verify_pdf_seal_bytes, PdfAnnotation, PdfDocument,
};
use power_tools::{
    build_document_a11y_tree, convert_file, extract_text_ocr_or_pdf, generate_cyclonedx_sbom_json,
    parse_ai_output_to_blocks, run_ai_task, word_to_markdown, word_to_pdf_document, AiAgentConfig,
    AiProviderPreset, AiTask, EnterprisePolicy,
};
use rustpad_render::{encode_surface_png, DaylightTheme, Surface};
use rustpad_window::{
    run_native_window, AppControl, CursorShape, DesktopAppHandler, WindowEvent,
};
use std::path::{Path, PathBuf};

struct NativeAppRunner {
    app: RustPadApp,
    tick_counter: u32,
}

impl DesktopAppHandler for NativeAppRunner {
    fn on_event(&mut self, event: WindowEvent, surface: &mut Surface) -> AppControl {
        match event {
            WindowEvent::Resize { width, height } => {
                surface.resize(width, height, DaylightTheme::DESK_BG);
                AppControl::Redraw
            }
            WindowEvent::MouseMove { x, y, .. } => {
                if self.app.handle_mouse_move(x, y) {
                    AppControl::Redraw
                } else {
                    AppControl::Continue
                }
            }
            WindowEvent::MouseDown { x, y, .. } => {
                self.app.handle_mouse_down(x, y);
                AppControl::Redraw
            }
            WindowEvent::MouseUp { .. } => {
                self.app.handle_mouse_up();
                AppControl::Redraw
            }
            WindowEvent::Scroll { delta_y, mods } => {
                self.app.handle_scroll(delta_y, mods.ctrl);
                AppControl::Redraw
            }
            WindowEvent::KeyDown { key, mods } => {
                self.app.handle_key_down(key, mods);
                self.app.cursor_blink_on = true;
                AppControl::Redraw
            }
            WindowEvent::TextInput(ch) => {
                self.app.handle_text_input(ch);
                self.app.cursor_blink_on = true;
                AppControl::Redraw
            }
            WindowEvent::Tick => {
                let mut needs_redraw = self.app.poll_background_tasks();
                self.tick_counter = self.tick_counter.wrapping_add(1);
                if self.tick_counter % 10 == 0 {
                    self.app.cursor_blink_on = !self.app.cursor_blink_on;
                    needs_redraw = true;
                }
                if needs_redraw {
                    AppControl::Redraw
                } else {
                    AppControl::Continue
                }
            }
            WindowEvent::CloseRequested => AppControl::Exit,
        }
    }

    fn render(&mut self, surface: &mut Surface) {
        render_app(&mut self.app, surface);
    }

    fn desired_cursor(&self) -> CursorShape {
        self.app.desired_cursor
    }
}

fn print_help() {
    println!(
        "\
RustPad v{ver} Enterprise — Offline-First Native Word (.docx), Forensic PDF & Local AI Suite
Author: Nour Yahyaoui <nourryahyaoui@gmail.com> · https://github.com/Nour-yahyaoui/rustpad

USAGE:
  rustpad [FILE]                              Launch native desktop studio (0 crates.io deps)
  rustpad redline <BASE.docx> <NEW.docx> -o <OUT.docx> [--author NAME]
                                              Generate ECMA-376 legal redline (<w:ins>/<w:del>)
  rustpad sanitize <IN.pdf> -o <OUT.pdf>      Forensically purge redacted PDF text & metadata
  rustpad sign <IN.pdf> -o <OUT.pdf> [--signer NAME] [--pdfa]
                                              Embed SHA-256 tamper-evident seal & PDF/A-2b XMP
  rustpad verify <FILE.pdf>                   Verify cryptographic SHA-256 PDF seal integrity
  rustpad sbom [-o sbom.cdx.json]             Emit CycloneDX 1.5 JSON Software Bill of Materials
  rustpad policy [--init-airgapped <OUT.json>] Inspect MDM DLP policy or generate air-gap profile
  rustpad ai <FILE> [OPTIONS]                 Run DLP-governed local (Ollama/nour-code) or cloud AI
      --task <grammar|summarize|description|sections|ask>
      --provider <ollama|nour-code|lm-studio|openai|claude>
      --url <ENDPOINT_URL>  --model <NAME>  --prompt <TEXT>  -o <OUTPUT.docx|.md>
  rustpad convert <INPUT> <OUTPUT>            Convert between .docx, .pdf, and .md offline
  rustpad inspect [--a11y] <FILE>             Inspect metrics, revisions, or UIA/AT-SPI a11y JSON
  rustpad ocr <FILE>                          Extract searchable text from a PDF or scanned image
  rustpad merge <OUT.pdf> <IN1.pdf>...        Merge multiple PDFs into a single vector PDF
  rustpad demo-files <OUT_DIR>                Generate enterprise .docx, .pdf, and .md samples
  rustpad --render-preview <OUT_DIR>          Render 1440x900 PNG studio screenshots & OG card
  rustpad --version | --help",
        ver = env!("CARGO_PKG_VERSION")
    );
}

fn run_cli_redline(args: &[String]) -> Result<(), String> {
    if args.len() < 4 {
        return Err(
            "Usage: rustpad redline <BASE.docx> <REVISED.docx> -o <OUT.docx> [--author NAME]"
                .into(),
        );
    }
    let base_path = PathBuf::from(&args[0]);
    let rev_path = PathBuf::from(&args[1]);
    let mut out_path: Option<PathBuf> = None;
    let mut author = String::from("Legal Review");

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("Missing output path after -o")?,
                ));
            }
            "--author" => {
                i += 1;
                author = args.get(i).ok_or("Missing name after --author")?.clone();
            }
            other => return Err(format!("Unrecognized flag '{other}' for `rustpad redline`")),
        }
        i += 1;
    }
    let out = out_path.ok_or("Missing required `-o <OUT.docx>` for `rustpad redline`")?;
    let base_doc = load_docx_file(&base_path).map_err(|e| e.0)?;
    let rev_doc = load_docx_file(&rev_path).map_err(|e| e.0)?;
    let redlined = Document::diff_redline(&base_doc, &rev_doc, &author, "2026-10-27T12:00:00Z");
    let (ins, del) = redlined.revision_counts();
    save_docx_file(&out, &redlined).map_err(|e| e.0)?;
    println!(
        "Generated ECMA-376 redline (+{ins} insertions, -{del} deletions) -> '{}'",
        out.display()
    );
    Ok(())
}

fn run_cli_sanitize(args: &[String]) -> Result<(), String> {
    if args.len() < 3 {
        return Err("Usage: rustpad sanitize <INPUT.pdf> -o <SANITIZED.pdf>".into());
    }
    let in_path = PathBuf::from(&args[0]);
    let mut out_path: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("Missing output path after -o")?,
                ));
            }
            other => return Err(format!("Unrecognized flag '{other}' for `rustpad sanitize`")),
        }
        i += 1;
    }
    let out = out_path.ok_or("Missing `-o <SANITIZED.pdf>`")?;
    let mut pdf = load_pdf_file(&in_path)?;
    let report = pdf.sanitize_redactions(true);
    save_pdf_file(&out, &pdf, true)?;
    println!(
        "Forensic PDF Sanitize complete: {} redaction box(es), {} underlying text run(s) purged -> '{}'",
        report.redaction_boxes,
        report.purged_text_runs,
        out.display()
    );
    Ok(())
}

fn run_cli_sign(args: &[String]) -> Result<(), String> {
    if args.len() < 3 {
        return Err(
            "Usage: rustpad sign <INPUT.pdf> -o <SEALED.pdf> [--signer NAME] [--pdfa]".into(),
        );
    }
    let in_path = PathBuf::from(&args[0]);
    let mut out_path: Option<PathBuf> = None;
    let mut signer = String::from("Nour Yahyaoui");
    let mut pdfa = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("Missing output path after -o")?,
                ));
            }
            "--signer" => {
                i += 1;
                signer = args.get(i).ok_or("Missing signer after --signer")?.clone();
            }
            "--pdfa" => pdfa = true,
            other => return Err(format!("Unrecognized flag '{other}' for `rustpad sign`")),
        }
        i += 1;
    }
    let out = out_path.ok_or("Missing `-o <SEALED.pdf>`")?;
    let pdf = load_pdf_file(&in_path)?;
    let (bytes, manifest) = seal_pdf_bytes(&pdf, &signer, "2026-10-27T12:00:00Z", pdfa);
    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    println!(
        "Sealed PDF (SHA-256: {} | PDF/A-2b: {}) -> '{}'",
        manifest.content_sha256,
        manifest.pdfa_conformance,
        out.display()
    );
    Ok(())
}

fn run_cli_verify(args: &[String]) -> Result<(), String> {
    let path = args
        .first()
        .ok_or("Usage: rustpad verify <SEALED.pdf>")?;
    let raw = std::fs::read(path).map_err(|e| format!("Failed reading '{path}': {e}"))?;
    let manifest = verify_pdf_seal_bytes(&raw)?;
    println!("STATUS:      VALID (Untampered SHA-256 Content Digest)");
    println!("Signer:      {}", manifest.signer);
    println!("Timestamp:   {}", manifest.date_iso);
    println!("PDF/A-2b:    {}", manifest.pdfa_conformance);
    println!("SHA-256:     {}", manifest.content_sha256);
    Ok(())
}

fn run_cli_sbom(args: &[String]) -> Result<(), String> {
    let sbom = generate_cyclonedx_sbom_json();
    if args.len() >= 2 && (args[0] == "-o" || args[0] == "--out") {
        std::fs::write(&args[1], &sbom).map_err(|e| e.to_string())?;
        println!("Wrote CycloneDX 1.5 SBOM -> '{}'", args[1]);
    } else {
        println!("{sbom}");
    }
    Ok(())
}

fn run_cli_policy(args: &[String]) -> Result<(), String> {
    if args.len() >= 2 && args[0] == "--init-airgapped" {
        let strict = EnterprisePolicy::strict_air_gapped("Enterprise Sovereign Deployment");
        std::fs::write(&args[1], strict.to_json_pretty()).map_err(|e| e.to_string())?;
        println!("Wrote strict Air-Gapped MDM policy -> '{}'", args[1]);
        return Ok(());
    }
    let (pol, src) = EnterprisePolicy::load_system_or_default();
    if let Some(p) = src {
        eprintln!("# Loaded policy from: {}", p.display());
    } else {
        eprintln!("# Loaded default workstation policy (no system policy.json bound)");
    }
    println!("{}", pol.to_json_pretty());
    Ok(())
}

fn run_cli_ai(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: rustpad ai <FILE> [--task summarize] [--provider ollama|nour-code|openai|claude] [--model NAME] [-o OUT]".into());
    }
    let input_path = PathBuf::from(&args[0]);
    let mut cfg = AiAgentConfig::default();
    let mut task = AiTask::SummarizeDocument;
    let mut custom_prompt = String::new();
    let mut out_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--task" | "-t" => {
                i += 1;
                let v = args.get(i).ok_or("Missing value after --task")?;
                task = AiTask::from_cli_name(v).ok_or_else(|| {
                    format!(
                        "Unknown --task '{v}' (expected grammar, summarize, description, sections, ask)"
                    )
                })?;
            }
            "--provider" | "-p" => {
                i += 1;
                let v = args.get(i).ok_or("Missing value after --provider")?;
                cfg.apply_preset(AiProviderPreset::from_slug(v));
            }
            "--url" => {
                i += 1;
                cfg.endpoint_url = args.get(i).ok_or("Missing value after --url")?.clone();
            }
            "--model" | "-m" => {
                i += 1;
                cfg.model = args.get(i).ok_or("Missing value after --model")?.clone();
            }
            "--prompt" => {
                i += 1;
                custom_prompt = args.get(i).ok_or("Missing value after --prompt")?.clone();
            }
            "-o" | "--out" => {
                i += 1;
                out_path = Some(PathBuf::from(args.get(i).ok_or("Missing value after -o")?));
            }
            flag => return Err(format!("Unrecognized flag '{flag}' for `rustpad ai`")),
        }
        i += 1;
    }

    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (mut maybe_doc, context) = match ext.as_str() {
        "docx" => {
            let doc = load_docx_file(&input_path).map_err(|e| e.0)?;
            let md = word_to_markdown(&doc);
            (Some(doc), md)
        }
        "pdf" => {
            let (doc, text) = extract_text_ocr_or_pdf(&input_path)?;
            (Some(doc), text)
        }
        _ => {
            let raw = std::fs::read_to_string(&input_path)
                .map_err(|e| format!("Failed reading '{}': {e}", input_path.display()))?;
            (None, raw)
        }
    };

    let res = run_ai_task(&cfg, task, &context, &custom_prompt)?;
    if let Some(out) = out_path {
        let out_ext = out
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if out_ext == "docx" {
            let mut doc = maybe_doc
                .take()
                .unwrap_or_else(|| power_tools::markdown_to_word(&context, "AI Document"));
            doc.blocks.extend(parse_ai_output_to_blocks(&res.content));
            save_docx_file(&out, &doc).map_err(|e| e.0)?;
            eprintln!(
                "AI ({} · {}ms) appended structured sections -> '{}'",
                res.model_used,
                res.latency_ms,
                out.display()
            );
        } else {
            std::fs::write(&out, &res.content).map_err(|e| e.to_string())?;
            eprintln!(
                "AI ({} · {}ms) wrote output -> '{}'",
                res.model_used,
                res.latency_ms,
                out.display()
            );
        }
    } else {
        println!("{}", res.content);
    }
    Ok(())
}

fn generate_demo_files(out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let welcome = TemplateKind::WelcomeGuide.build();
    let msa = TemplateKind::EnterpriseContract.build();
    let resume = TemplateKind::ModernResume.build();
    let report = TemplateKind::AcademicReport.build();
    let invoice = TemplateKind::BusinessInvoice.build();

    save_docx_file(out_dir.join("RustPad_Welcome_Guide.docx"), &welcome).map_err(|e| e.0)?;
    save_docx_file(out_dir.join("Enterprise_MSA_Redline.docx"), &msa).map_err(|e| e.0)?;
    save_docx_file(out_dir.join("Nour_Yahyaoui_Modern_Resume.docx"), &resume).map_err(|e| e.0)?;
    save_docx_file(out_dir.join("Technical_Architecture_Report.docx"), &report).map_err(|e| e.0)?;

    export_document_to_pdf_file(
        out_dir.join("Enterprise_MSA_Redline_PDFA.pdf"),
        &msa,
        &PdfCreateOptions {
            pdfa_conformance: true,
            show_revisions: true,
            ..PdfCreateOptions::default()
        },
    )?;
    export_document_to_pdf_file(
        out_dir.join("Business_Invoice_Sample.pdf"),
        &invoice,
        &PdfCreateOptions::default(),
    )?;
    std::fs::write(
        out_dir.join("RustPad_Welcome_Guide.md"),
        word_to_markdown(&welcome),
    )
    .map_err(|e| e.to_string())?;

    println!(
        "Generated 7 enterprise sample documents (.docx, PDF/A-2b .pdf, .md) in '{}'",
        out_dir.display()
    );
    Ok(())
}

fn render_studio_previews(out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut app = RustPadApp::new();
    let mut surface = Surface::new(1440, 900, DaylightTheme::DESK_BG);

    // 1. Word Studio: Enterprise MSA Legal Redline + Review Comments + Local AI Agent Dock
    let msa_doc = TemplateKind::EnterpriseContract.build();
    app.tabs
        .push(StudioTab::Word(WordTabState::from_doc(msa_doc, None)));
    let msa_tab_idx = app.tabs.len() - 1;
    app.active_tab = msa_tab_idx;
    app.ai_dock.open = true;
    app.ai_dock.output_text = "## Redline Risk Review\n- Liability cap increased to 24 months (Section 3, emerald w:ins).\n- Payment window extended Net 30 -> Net 45 (crimson w:del / emerald w:ins).\n- Zero cloud egress: verified via local nour-code / Ollama DLP policy.".into();
    app.ai_dock.status_line = "Completed via local nour-code (DLP Verified · 162 ms).".into();
    render_app(&mut app, &mut surface);
    std::fs::write(out_dir.join("word-studio.png"), encode_surface_png(&surface))
        .map_err(|e| e.to_string())?;
    app.ai_dock.open = false;

    // 2. Asymmetric Editorial Home Hub Preview
    app.active_tab = 0;
    render_app(&mut app, &mut surface);
    std::fs::write(out_dir.join("home-hub.png"), encode_surface_png(&surface))
        .map_err(|e| e.to_string())?;

    // 3. PDF Studio Preview (with Forensic Sanitize, SHA-256 Seal, Stamp & Ink Signature)
    let report_doc = TemplateKind::AcademicReport.build();
    let mut pdf_doc = word_to_pdf_document(&report_doc)?;
    if let Some(pg) = pdf_doc.pages.first_mut() {
        pg.annotations.push(PdfAnnotation::Stamp {
            x: pg.width_pt - 175.0,
            y: pg.height_pt - 86.0,
            label: "SHA-256 SEALED".into(),
            color: RgbColor::EMERALD,
        });
        pg.annotations.push(PdfAnnotation::Highlight {
            x: 64.0,
            y: pg.height_pt - 165.0,
            w: 240.0,
            h: 16.0,
            color: RgbColor::AMBER_HIGHLIGHT,
        });
        pg.annotations.push(PdfAnnotation::StickyNote {
            x: pg.width_pt - 210.0,
            y: pg.height_pt - 230.0,
            author: "Nour Yahyaoui".into(),
            note: "PDF/A-2b + Forensic Redaction Ready".into(),
        });
        pg.annotations.push(PdfAnnotation::Signature {
            strokes: vec![vec![
                (pg.width_pt - 220.0, 112.0),
                (pg.width_pt - 188.0, 138.0),
                (pg.width_pt - 164.0, 108.0),
                (pg.width_pt - 128.0, 134.0),
                (pg.width_pt - 88.0, 116.0),
            ]],
            signer: "Nour Yahyaoui".into(),
            color: RgbColor::COBALT,
        });
    }
    app.tabs
        .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf_doc, None)));
    let pdf_tab_idx = app.tabs.len() - 1;
    app.active_tab = pdf_tab_idx;
    render_app(&mut app, &mut surface);
    std::fs::write(out_dir.join("pdf-studio.png"), encode_surface_png(&surface))
        .map_err(|e| e.to_string())?;

    // 4. Side-by-Side Split Workspace + Enterprise Governance / DLP Audit Modal
    app.active_tab = msa_tab_idx;
    app.split_secondary_tab = Some(pdf_tab_idx);
    app.execute_command(CommandId::OpenGovernanceModal);
    render_app(&mut app, &mut surface);
    std::fs::write(
        out_dir.join("split-command-palette.png"),
        encode_surface_png(&surface),
    )
    .map_err(|e| e.to_string())?;

    // 5. Crisp 1200x630 Open Graph Preview Image (clean Split Redline + PDF Studio)
    app.modal = None;
    let mut og_surface = Surface::new(1200, 630, DaylightTheme::DESK_BG);
    render_app(&mut app, &mut og_surface);
    std::fs::write(out_dir.join("og.png"), encode_surface_png(&og_surface))
        .map_err(|e| e.to_string())?;

    println!(
        "Rendered 5 enterprise studio PNG screenshots in '{}'",
        out_dir.display()
    );
    Ok(())
}

fn inspect_file(path: &Path, emit_a11y_json: bool) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "docx" => {
            let doc = load_docx_file(path).map_err(|e| e.0)?;
            if emit_a11y_json {
                println!("{}", build_document_a11y_tree(&doc).to_json());
                return Ok(());
            }
            let st = doc.stats();
            let (ins, del) = doc.revision_counts();
            println!("Format:          ECMA-376 Word (.docx)");
            println!("Title:           {}", doc.title);
            println!("Author:          {}", doc.author);
            println!("Classification:  {}", doc.classification.label());
            println!("Track Changes:   {} (+{ins} ins / -{del} del)", doc.track_changes);
            println!("Comments:        {}", doc.comments.len());
            println!("Passthrough XML: {} preserved part(s)", doc.passthrough_parts.len());
            println!("Paper Size:      {}", doc.page_setup.paper_size.label());
            println!("Words:           {}", st.words);
            println!("Characters:      {}", st.chars);
            println!("Paragraphs:      {}", st.paragraphs);
            println!("Tables:          {}", st.tables);
            println!("Outline H1+:     {}", doc.outline().len());
            Ok(())
        }
        "pdf" => {
            let pdf = load_pdf_file(path)?;
            let raw = std::fs::read(path).unwrap_or_default();
            let seal_state = match verify_pdf_seal_bytes(&raw) {
                Ok(m) => format!("VALID SHA-256 ({}…)", &m.content_sha256[..16]),
                Err(_) => "Unsigned".into(),
            };
            let text_blocks: usize = pdf.pages.iter().map(|p| p.text_items.len()).sum();
            let annotations: usize = pdf.pages.iter().map(|p| p.annotations.len()).sum();
            println!("Format:          ISO 32000 PDF (.pdf)");
            println!("Title:           {}", pdf.title);
            println!("Pages:           {}", pdf.pages.len());
            println!("Text Items:      {text_blocks}");
            println!("Annotations:     {annotations}");
            println!("Integrity Seal:  {seal_state}");
            Ok(())
        }
        other => Err(format!(
            "Unsupported inspect format '.{other}' (expected .docx or .pdf)"
        )),
    }
}

fn run_cli() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = args.first().map(|s| s.as_str()) {
        match first {
            "-h" | "--help" | "help" => {
                print_help();
                return Ok(());
            }
            "-V" | "--version" | "version" => {
                println!("rustpad {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "redline" => return run_cli_redline(&args[1..]),
            "sanitize" => return run_cli_sanitize(&args[1..]),
            "sign" => return run_cli_sign(&args[1..]),
            "verify" => return run_cli_verify(&args[1..]),
            "sbom" => return run_cli_sbom(&args[1..]),
            "policy" => return run_cli_policy(&args[1..]),
            "ai" => return run_cli_ai(&args[1..]),
            "convert" => {
                if args.len() < 3 {
                    return Err("Usage: rustpad convert <INPUT> <OUTPUT>".into());
                }
                convert_file(&args[1], &args[2])?;
                println!("Converted '{}' -> '{}'", args[1], args[2]);
                return Ok(());
            }
            "inspect" => {
                if args.len() < 2 {
                    return Err("Usage: rustpad inspect [--a11y] <FILE>".into());
                }
                if args[1] == "--a11y" {
                    let f = args.get(2).ok_or("Missing <FILE> after --a11y")?;
                    return inspect_file(Path::new(f), true);
                }
                return inspect_file(Path::new(&args[1]), false);
            }
            "ocr" => {
                if args.len() < 2 {
                    return Err("Usage: rustpad ocr <FILE>".into());
                }
                let (_doc, text) = extract_text_ocr_or_pdf(Path::new(&args[1]))?;
                println!("{text}");
                return Ok(());
            }
            "merge" => {
                if args.len() < 4 {
                    return Err("Usage: rustpad merge <OUT.pdf> <IN1.pdf> <IN2.pdf> [...]".into());
                }
                let inputs: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
                let merged = PdfDocument::merge_files(&inputs, "Merged_Document")?;
                save_pdf_file(&args[1], &merged, true)?;
                println!(
                    "Merged {} PDFs ({} total pages) -> '{}'",
                    inputs.len(),
                    merged.pages.len(),
                    args[1]
                );
                return Ok(());
            }
            "demo-files" => {
                let dir = args
                    .get(1)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("examples"));
                return generate_demo_files(&dir);
            }
            "--render-preview" => {
                let dir = args
                    .get(1)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("docs/screenshots"));
                return render_studio_previews(&dir);
            }
            _ => {}
        }
    }

    let mut app = RustPadApp::new();
    if let Some(open_arg) = args.first() {
        app.open_path(open_arg)?;
    }
    let mut runner = NativeAppRunner {
        app,
        tick_counter: 0,
    };
    run_native_window(
        "RustPad v2.0 Enterprise — Pure-Rust Offline Word (.docx), Forensic PDF & AI Suite",
        1380,
        880,
        &mut runner,
    )
}

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("rustpad: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_v2_render_redline_governance_and_pdf_forensics() {
        std::env::set_var("RUSTPAD_HEADLESS_PRINT", "1");
        let mut app = RustPadApp::new();
        let mut surface = Surface::new(1360, 860, DaylightTheme::DESK_BG);
        render_app(&mut app, &mut surface);
        assert!(!app.hit_regions.is_empty());

        app.execute_command(CommandId::NewWordEnterpriseMsa);
        render_app(&mut app, &mut surface);

        app.execute_command(CommandId::CycleClassification);
        app.execute_command(CommandId::PrintDocument);
        app.execute_command(CommandId::InsertTable);
        app.execute_command(CommandId::TableAddRow);
        app.execute_command(CommandId::TableAddCol);
        app.execute_command(CommandId::ToggleAiDock);
        app.execute_command(CommandId::ConvertPdfToWord);
        app.execute_command(CommandId::PdfSanitizeRedactions);
        app.execute_command(CommandId::PdfSealDocument);
        app.execute_command(CommandId::ToggleSplitView);
        app.execute_command(CommandId::OpenGovernanceModal);
        render_app(&mut app, &mut surface);
        let png = encode_surface_png(&surface);
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(png.len() > 8_000);
    }
}
