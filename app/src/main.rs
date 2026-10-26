//! RustPad (`rustpad`) — 100% Pure-Rust Native Word (`.docx`) & PDF (`.pdf`) Desktop Studio + CLI.
//! Author: Nour Yahyaoui <nourryahyaoui@gmail.com> · https://github.com/Nour-yahyaoui/rustpad

pub mod commands;
pub mod pdf_view;
pub mod state;
pub mod ui_paint;
pub mod word_view;

#[cfg(test)]
use crate::commands::CommandId;
use crate::state::{ActiveModal, PdfTabState, RustPadApp, StudioTab};
use crate::ui_paint::render_app;
use doc_model::{RgbColor, TemplateKind};
use docx_io::{load_docx_file, save_docx_file};
use pdf_create::{export_document_to_pdf_file, PdfCreateOptions};
use pdf_engine::{load_pdf_file, PdfAnnotation, PdfDocument};
use power_tools::{convert_file, extract_text_ocr_or_pdf, word_to_markdown, word_to_pdf_document};
use rustpad_render::{encode_surface_png, DaylightTheme, Surface};
use rustpad_window::{run_native_window, AppControl, DesktopAppHandler, WindowEvent};
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
                self.tick_counter = self.tick_counter.wrapping_add(1);
                if self.tick_counter % 30 == 0 {
                    self.app.cursor_blink_on = !self.app.cursor_blink_on;
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
}

fn print_help() {
    println!(
        "\
RustPad v{ver} — Pure-Rust Native Word (.docx) & PDF (.pdf) Desktop Studio
Author: Nour Yahyaoui <nourryahyaoui@gmail.com> · https://github.com/Nour-yahyaoui/rustpad

USAGE:
  rustpad [FILE]                        Launch the native desktop studio (optionally opening FILE)
  rustpad convert <INPUT> <OUTPUT>      Convert between .docx, .pdf, and .md offline
  rustpad inspect <FILE>                Print page, word, character, or PDF object metrics
  rustpad ocr <FILE>                    Extract searchable text from a PDF or scanned image
  rustpad merge <OUT.pdf> <IN1.pdf>...  Merge multiple PDFs into a single vector PDF
  rustpad demo-files <OUT_DIR>          Generate showcase .docx, .pdf, and .md sample documents
  rustpad --render-preview <OUT_DIR>    Render 1440x900 PNG studio screenshots & 1200x630 OG card
  rustpad --version                     Print version information
  rustpad --help                        Print this help guide",
        ver = env!("CARGO_PKG_VERSION")
    );
}

fn generate_demo_files(out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let welcome = TemplateKind::WelcomeGuide.build();
    let resume = TemplateKind::ModernResume.build();
    let report = TemplateKind::AcademicReport.build();
    let invoice = TemplateKind::BusinessInvoice.build();

    save_docx_file(out_dir.join("RustPad_Welcome_Guide.docx"), &welcome).map_err(|e| e.0)?;
    save_docx_file(out_dir.join("Nour_Yahyaoui_Modern_Resume.docx"), &resume).map_err(|e| e.0)?;
    save_docx_file(out_dir.join("Technical_Architecture_Report.docx"), &report).map_err(|e| e.0)?;

    export_document_to_pdf_file(
        out_dir.join("RustPad_Welcome_Guide.pdf"),
        &welcome,
        &PdfCreateOptions::default(),
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
        "Generated 6 sample documents (.docx, .pdf, .md) in '{}'",
        out_dir.display()
    );
    Ok(())
}

fn render_studio_previews(out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut app = RustPadApp::new();
    let mut surface = Surface::new(1440, 900, DaylightTheme::DESK_BG);

    // 1. Word Studio Preview (Welcome Guide)
    app.active_tab = 1;
    render_app(&mut app, &mut surface);
    std::fs::write(out_dir.join("word-studio.png"), encode_surface_png(&surface))
        .map_err(|e| e.to_string())?;

    // 2. Home Hub Preview
    app.active_tab = 0;
    render_app(&mut app, &mut surface);
    std::fs::write(out_dir.join("home-hub.png"), encode_surface_png(&surface))
        .map_err(|e| e.to_string())?;

    // 3. PDF Studio Preview (with Stamp, Highlight, Sticky Note & Ink Signature)
    let report_doc = TemplateKind::AcademicReport.build();
    let mut pdf_doc = word_to_pdf_document(&report_doc)?;
    if let Some(pg) = pdf_doc.pages.first_mut() {
        pg.annotations.push(PdfAnnotation::Stamp {
            x: pg.width_pt - 175.0,
            y: pg.height_pt - 86.0,
            label: "APPROVED".into(),
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
            note: "Verified pure-Rust vector PDF stream".into(),
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

    // 4. Side-by-Side Split Workspace + Ctrl+K Command Palette Preview
    app.active_tab = 1;
    app.split_secondary_tab = Some(pdf_tab_idx);
    app.modal = Some(ActiveModal::CommandPalette {
        query: "pdf".into(),
        selected: 0,
    });
    render_app(&mut app, &mut surface);
    std::fs::write(
        out_dir.join("split-command-palette.png"),
        encode_surface_png(&surface),
    )
    .map_err(|e| e.to_string())?;

    // 5. Crisp 1200x630 Open Graph Preview Image (clean Split Studio without modal)
    app.modal = None;
    let mut og_surface = Surface::new(1200, 630, DaylightTheme::DESK_BG);
    render_app(&mut app, &mut og_surface);
    std::fs::write(out_dir.join("og.png"), encode_surface_png(&og_surface))
        .map_err(|e| e.to_string())?;

    println!(
        "Rendered 5 studio PNG screenshots in '{}'",
        out_dir.display()
    );
    Ok(())
}

fn inspect_file(path: &Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "docx" => {
            let doc = load_docx_file(path).map_err(|e| e.0)?;
            let st = doc.stats();
            println!("Format:      ECMA-376 Word (.docx)");
            println!("Title:       {}", doc.title);
            println!("Author:      {}", doc.author);
            println!("Paper Size:  {}", doc.page_setup.paper_size.label());
            println!("Words:       {}", st.words);
            println!("Characters:  {}", st.chars);
            println!("Paragraphs:  {}", st.paragraphs);
            println!("Tables:      {}", st.tables);
            println!("Outline H1+: {}", doc.outline().len());
            Ok(())
        }
        "pdf" => {
            let pdf = load_pdf_file(path)?;
            let text_blocks: usize = pdf.pages.iter().map(|p| p.text_items.len()).sum();
            let annotations: usize = pdf.pages.iter().map(|p| p.annotations.len()).sum();
            println!("Format:      ISO 32000 PDF (.pdf)");
            println!("Title:       {}", pdf.title);
            println!("Pages:       {}", pdf.pages.len());
            println!("Text Items:  {text_blocks}");
            println!("Annotations: {annotations}");
            Ok(())
        }
        other => Err(format!("Unsupported inspect format '.{other}' (expected .docx or .pdf)")),
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
                    return Err("Usage: rustpad inspect <FILE>".into());
                }
                return inspect_file(Path::new(&args[1]));
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
                pdf_engine::save_pdf_file(&args[1], &merged, true)?;
                println!(
                    "Merged {} PDFs ({} total pages) -> '{}'",
                    inputs.len(),
                    merged.pages.len(),
                    args[1]
                );
                return Ok(());
            }
            "demo-files" => {
                let dir = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("examples"));
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
        "RustPad — Pure-Rust Word (.docx) & PDF (.pdf) Studio",
        1360,
        860,
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
    fn test_app_render_and_command_execution() {
        let mut app = RustPadApp::new();
        let mut surface = Surface::new(1280, 800, DaylightTheme::DESK_BG);
        render_app(&mut app, &mut surface);
        assert!(!app.hit_regions.is_empty());

        app.execute_command(CommandId::ToggleBold);
        app.execute_command(CommandId::ConvertPdfToWord);
        app.execute_command(CommandId::StampApproved);
        app.execute_command(CommandId::ToggleSplitView);
        render_app(&mut app, &mut surface);
        let png = encode_surface_png(&surface);
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(png.len() > 8_000);
    }
}
