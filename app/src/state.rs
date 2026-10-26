//! Application State, Multi-Tab Session Controller, Modals & Hit-Testing Actions.

use crate::commands::{filter_commands, CommandId};
use doc_model::{
    Alignment, Block, Document, FontFamily, ListKind, PaperSize, Paragraph, ParagraphStyle,
    RgbColor, TableBlock, TemplateKind, UndoHistory,
};
use docx_io::{load_docx_file, save_docx_file};
use pdf_create::{export_document_to_pdf_file, PdfCreateOptions};
use pdf_engine::{
    compress_pdf_bytes, load_pdf_file, save_pdf_bytes, save_pdf_file, PdfAnnotation, PdfDocument,
};
use power_tools::{
    markdown_to_word, pdf_to_word, word_to_markdown, word_to_pdf_document, SnapshotVault,
    SpellChecker,
};
use rustpad_render::{FontBook, Rect};
use rustpad_window::{try_native_open_dialog, try_native_save_dialog, Key, Modifiers};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfTool {
    SelectEdit,
    AddText,
    Highlight,
    StickyNote,
    Redact,
}

impl PdfTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::SelectEdit => "Select / Edit Text",
            Self::AddText => "+ Text Box",
            Self::Highlight => "Highlight",
            Self::StickyNote => "Sticky Note",
            Self::Redact => "Redact",
        }
    }
}

pub struct WordTabState {
    pub doc: Document,
    pub path: Option<PathBuf>,
    pub cursor_block: usize,
    pub cursor_char: usize,
    pub active_table_cell: Option<(usize, usize, usize)>, // (block_idx, row, col)
    pub scroll_y: f32,
    pub zoom: f32,
    pub history: UndoHistory,
    pub snapshots: SnapshotVault,
    pub spellcheck: bool,
    pub dirty: bool,
}

impl WordTabState {
    pub fn from_doc(doc: Document, path: Option<PathBuf>) -> Self {
        let mut snapshots = SnapshotVault::default();
        snapshots.capture(&doc, "Opened / Initial state");
        let cursor_char = doc
            .blocks
            .first()
            .and_then(|b| match b {
                Block::Paragraph(p) => Some(p.plain_text().chars().count()),
                _ => None,
            })
            .unwrap_or(0);
        Self {
            doc,
            path,
            cursor_block: 0,
            cursor_char,
            active_table_cell: None,
            scroll_y: 0.0,
            zoom: 1.0,
            history: UndoHistory::default(),
            snapshots,
            spellcheck: false,
            dirty: false,
        }
    }
}

pub struct PdfTabState {
    pub pdf: PdfDocument,
    pub path: Option<PathBuf>,
    pub current_page: usize,
    pub selected_text_id: Option<u32>,
    pub tool: PdfTool,
    pub scroll_y: f32,
    pub zoom: f32,
    pub dirty: bool,
}

impl PdfTabState {
    pub fn from_pdf(pdf: PdfDocument, path: Option<PathBuf>) -> Self {
        Self {
            pdf,
            path,
            current_page: 0,
            selected_text_id: None,
            tool: PdfTool::SelectEdit,
            scroll_y: 0.0,
            zoom: 1.0,
            dirty: false,
        }
    }
}

pub enum StudioTab {
    HomeHub,
    Word(WordTabState),
    Pdf(PdfTabState),
}

impl StudioTab {
    pub fn title(&self) -> String {
        match self {
            Self::HomeHub => "Home".into(),
            Self::Word(w) => {
                let mark = if w.dirty { " •" } else { "" };
                format!("{}{mark}", w.doc.title)
            }
            Self::Pdf(p) => {
                let mark = if p.dirty { " •" } else { "" };
                format!("{}{mark}", p.pdf.title)
            }
        }
    }

    pub fn badge(&self) -> (&'static str, bool) {
        match self {
            Self::HomeHub => ("HOME", false),
            Self::Word(_) => ("DOCX", false),
            Self::Pdf(_) => ("PDF", true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileModalKind {
    OpenAny,
    SaveActive,
    ExportPdf,
    MergePdf,
}

pub enum ActiveModal {
    CommandPalette {
        query: String,
        selected: usize,
    },
    FilePrompt {
        kind: FileModalKind,
        input_path: String,
        recent_candidates: Vec<PathBuf>,
    },
    SignaturePad {
        strokes: Vec<Vec<(f32, f32)>>,
        is_drawing: bool,
        pad_rect: Rect,
        signer_name: String,
    },
    FindReplace {
        find_text: String,
        replace_text: String,
        editing_replace: bool,
        matches_count: usize,
    },
    SplitRange {
        range_spec: String,
    },
    Watermark {
        text: String,
    },
    SnapshotsDrawer,
    About,
}

#[derive(Debug, Clone)]
pub enum UiAction {
    SwitchTab(usize),
    CloseTab(usize),
    OpenHomeTab,
    OpenCommandPalette,
    ExecCommand(CommandId),
    CreateFromTemplate(TemplateKind),
    OpenRecentPath(PathBuf),
    JumpOutline {
        tab_idx: usize,
        block_idx: usize,
        target_scroll_y: f32,
    },
    ClickWordParagraph {
        tab_idx: usize,
        block_idx: usize,
        char_idx: usize,
    },
    ToggleChecklistItem {
        tab_idx: usize,
        block_idx: usize,
    },
    ClickTableCell {
        tab_idx: usize,
        block_idx: usize,
        row: usize,
        col: usize,
    },
    SelectPdfPage {
        tab_idx: usize,
        page_idx: usize,
    },
    MovePdfPage {
        tab_idx: usize,
        from: usize,
        to: usize,
    },
    SetPdfTool {
        tab_idx: usize,
        tool: PdfTool,
    },
    ClickPdfTextItem {
        tab_idx: usize,
        page_idx: usize,
        item_id: u32,
    },
    ClickPdfPageCanvas {
        tab_idx: usize,
        page_idx: usize,
        pdf_x: f32,
        pdf_y: f32,
    },
    RestoreSnapshot(u64),
    ModalClose,
    ModalConfirm,
    SignatureClear,
    PalettePick(CommandId),
    FileCandidatePick(PathBuf),
    FindFocusField(bool),
    FindReplaceAll,
}

#[derive(Clone)]
pub struct HitRegion {
    pub rect: Rect,
    pub action: UiAction,
}

pub struct RustPadApp {
    pub fonts: FontBook,
    pub spellchecker: SpellChecker,
    pub tabs: Vec<StudioTab>,
    pub active_tab: usize,
    pub split_secondary_tab: Option<usize>,
    pub modal: Option<ActiveModal>,
    pub hit_regions: Vec<HitRegion>,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub cursor_blink_on: bool,
    pub status_toast: String,
    pub recent_files: Vec<PathBuf>,
}

impl Default for RustPadApp {
    fn default() -> Self {
        Self::new()
    }
}

impl RustPadApp {
    pub fn new() -> Self {
        let welcome_doc = TemplateKind::WelcomeGuide.build();
        let recent = discover_workspace_documents();
        Self {
            fonts: FontBook::new(),
            spellchecker: SpellChecker::new(),
            tabs: vec![
                StudioTab::HomeHub,
                StudioTab::Word(WordTabState::from_doc(welcome_doc, None)),
            ],
            active_tab: 1,
            split_secondary_tab: None,
            modal: None,
            hit_regions: Vec::with_capacity(128),
            mouse_x: -1.0,
            mouse_y: -1.0,
            cursor_blink_on: true,
            status_toast: "Ready · Paper & Daylight Native Rust Studio".into(),
            recent_files: recent,
        }
    }

    pub fn open_path(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let p = path.as_ref().to_path_buf();
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "docx" => {
                let doc = load_docx_file(&p).map_err(|e| e.0)?;
                self.tabs
                    .push(StudioTab::Word(WordTabState::from_doc(doc, Some(p.clone()))));
                self.active_tab = self.tabs.len() - 1;
                self.push_recent(p.clone());
                self.status_toast = format!("Opened Word document '{}'", p.display());
                Ok(())
            }
            "pdf" => {
                let pdf = load_pdf_file(&p)?;
                self.tabs
                    .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, Some(p.clone()))));
                self.active_tab = self.tabs.len() - 1;
                self.push_recent(p.clone());
                self.status_toast = format!("Opened PDF Studio tab '{}'", p.display());
                Ok(())
            }
            "md" | "markdown" | "txt" => {
                let raw = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
                let title = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Imported Note");
                let doc = markdown_to_word(&raw, title);
                self.tabs
                    .push(StudioTab::Word(WordTabState::from_doc(doc, None)));
                self.active_tab = self.tabs.len() - 1;
                self.push_recent(p.clone());
                self.status_toast = format!("Imported '{}' into Word Studio", p.display());
                Ok(())
            }
            other => Err(format!("Unsupported file format '.{other}'")),
        }
    }

    fn push_recent(&mut self, p: PathBuf) {
        self.recent_files.retain(|x| x != &p);
        self.recent_files.insert(0, p);
        if self.recent_files.len() > 8 {
            self.recent_files.truncate(8);
        }
    }

    pub fn save_active_to(&mut self, target: PathBuf) -> Result<(), String> {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return Err("No active tab".into());
        };
        let ext = target
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match tab {
            StudioTab::Word(w) => {
                if ext == "pdf" {
                    export_document_to_pdf_file(&target, &w.doc, &PdfCreateOptions::default())?;
                    self.status_toast = format!("Exported crisp vector PDF '{}'", target.display());
                } else if ext == "md" {
                    let md = word_to_markdown(&w.doc);
                    std::fs::write(&target, md).map_err(|e| e.to_string())?;
                    self.status_toast = format!("Saved Markdown '{}'", target.display());
                } else {
                    let final_path = if ext.is_empty() {
                        target.with_extension("docx")
                    } else {
                        target
                    };
                    if let Some(stem) = final_path.file_stem().and_then(|s| s.to_str()) {
                        w.doc.title = stem.to_string();
                    }
                    save_docx_file(&final_path, &w.doc).map_err(|e| e.0)?;
                    w.path = Some(final_path.clone());
                    w.dirty = false;
                    w.snapshots.capture(&w.doc, "Saved to disk");
                    self.push_recent(final_path.clone());
                    self.status_toast = format!("Saved Word (.docx) '{}'", final_path.display());
                }
                Ok(())
            }
            StudioTab::Pdf(p) => {
                let final_path = if ext == "pdf" {
                    target
                } else {
                    target.with_extension("pdf")
                };
                if let Some(stem) = final_path.file_stem().and_then(|s| s.to_str()) {
                    p.pdf.title = stem.to_string();
                }
                save_pdf_file(&final_path, &p.pdf, true)?;
                p.path = Some(final_path.clone());
                p.dirty = false;
                self.push_recent(final_path.clone());
                self.status_toast = format!("Saved PDF '{}'", final_path.display());
                Ok(())
            }
            StudioTab::HomeHub => Err("Switch to a Word or PDF tab to save".into()),
        }
    }

    pub fn prompt_or_open_modal(&mut self, kind: FileModalKind, default_name: &str) {
        match kind {
            FileModalKind::OpenAny | FileModalKind::MergePdf => {
                if let Some(picked) = try_native_open_dialog("Open in RustPad") {
                    self.apply_file_modal_choice(kind, picked);
                    return;
                }
            }
            FileModalKind::SaveActive | FileModalKind::ExportPdf => {
                if let Some(picked) = try_native_save_dialog("Save from RustPad", default_name) {
                    self.apply_file_modal_choice(kind, picked);
                    return;
                }
            }
        }
        let candidates = discover_workspace_documents();
        self.modal = Some(ActiveModal::FilePrompt {
            kind,
            input_path: default_name.to_string(),
            recent_candidates: candidates,
        });
    }

    pub fn apply_file_modal_choice(&mut self, kind: FileModalKind, path: PathBuf) {
        match kind {
            FileModalKind::OpenAny => {
                if let Err(e) = self.open_path(&path) {
                    self.status_toast = format!("Open error: {e}");
                }
            }
            FileModalKind::SaveActive => {
                if let Err(e) = self.save_active_to(path) {
                    self.status_toast = format!("Save error: {e}");
                }
            }
            FileModalKind::ExportPdf => {
                let pdf_path = if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                    path
                } else {
                    path.with_extension("pdf")
                };
                if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                    match export_document_to_pdf_file(
                        &pdf_path,
                        &w.doc,
                        &PdfCreateOptions::default(),
                    ) {
                        Ok(()) => {
                            let _ = self.open_path(&pdf_path);
                            self.status_toast = format!(
                                "Exported & opened vector PDF '{}'",
                                pdf_path.display()
                            );
                        }
                        Err(e) => self.status_toast = format!("PDF export error: {e}"),
                    }
                }
            }
            FileModalKind::MergePdf => match load_pdf_file(&path) {
                Ok(other) => {
                    if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                        let added = other.pages.len();
                        p.pdf.pages.extend(other.pages);
                        p.dirty = true;
                        self.status_toast =
                            format!("Merged {added} page(s) from '{}'", path.display());
                    }
                }
                Err(e) => self.status_toast = format!("Merge PDF error: {e}"),
            },
        }
    }

    pub fn execute_command(&mut self, cmd: CommandId) {
        match cmd {
            CommandId::NewWordBlank => {
                self.tabs.push(StudioTab::Word(WordTabState::from_doc(
                    Document::blank(),
                    None,
                )));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = "Created blank A4 Word document".into();
            }
            CommandId::NewWordWelcome => {
                self.tabs.push(StudioTab::Word(WordTabState::from_doc(
                    TemplateKind::WelcomeGuide.build(),
                    None,
                )));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = "Opened Welcome Guide document".into();
            }
            CommandId::NewPdfBlank => {
                let pdf = PdfDocument::new_blank("Untitled_PDF", 1, PaperSize::A4);
                self.tabs
                    .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, None)));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = "Created new Blank PDF Studio tab".into();
            }
            CommandId::OpenFile => {
                self.prompt_or_open_modal(FileModalKind::OpenAny, "");
            }
            CommandId::SaveFile => {
                let existing = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => w.path.clone(),
                    Some(StudioTab::Pdf(p)) => p.path.clone(),
                    _ => None,
                };
                if let Some(p) = existing {
                    let _ = self.save_active_to(p);
                } else {
                    self.execute_command(CommandId::SaveAsFile);
                }
            }
            CommandId::SaveAsFile => {
                let def = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => {
                        format!("{}.docx", sanitize_filename(&w.doc.title))
                    }
                    Some(StudioTab::Pdf(p)) => format!("{}.pdf", sanitize_filename(&p.pdf.title)),
                    _ => "Document.docx".into(),
                };
                self.prompt_or_open_modal(FileModalKind::SaveActive, &def);
            }
            CommandId::ExportWordToPdf => {
                if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                    let def = format!("{}.pdf", sanitize_filename(&w.doc.title));
                    self.prompt_or_open_modal(FileModalKind::ExportPdf, &def);
                } else {
                    self.status_toast = "Switch to a Word (.docx) tab to export PDF".into();
                }
            }
            CommandId::ConvertPdfToWord => {
                let converted = if let Some(StudioTab::Pdf(p)) = self.tabs.get(self.active_tab) {
                    Some(pdf_to_word(&p.pdf))
                } else {
                    None
                };
                if let Some(mut doc) = converted {
                    doc.title = format!("{}_Editable", doc.title);
                    self.tabs
                        .push(StudioTab::Word(WordTabState::from_doc(doc, None)));
                    self.active_tab = self.tabs.len() - 1;
                    self.status_toast =
                        "Converted PDF into a fully editable Word (.docx) tab!".into();
                } else if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                    if let Ok(pdf) = word_to_pdf_document(&w.doc) {
                        self.tabs
                            .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, None)));
                        self.active_tab = self.tabs.len() - 1;
                        self.status_toast =
                            "Opened Word document directly inside PDF Studio!".into();
                    }
                }
            }
            CommandId::ToggleBold
            | CommandId::ToggleItalic
            | CommandId::ToggleUnderline
            | CommandId::ToggleStrike
            | CommandId::CycleStyle
            | CommandId::CycleFontFamily
            | CommandId::FontSizeUp
            | CommandId::FontSizeDown
            | CommandId::CycleTextColor
            | CommandId::ToggleHighlight
            | CommandId::AlignLeft
            | CommandId::AlignCenter
            | CommandId::AlignRight
            | CommandId::ToggleBulletList
            | CommandId::ToggleChecklist
            | CommandId::InsertTable
            | CommandId::TableAddRow
            | CommandId::TableAddCol
            | CommandId::InsertPageBreak
            | CommandId::CyclePaperSize
            | CommandId::ToggleSpellcheck => {
                self.exec_word_command(cmd);
            }
            CommandId::PdfToolSelect => self.set_active_pdf_tool(PdfTool::SelectEdit),
            CommandId::PdfToolAddText => self.set_active_pdf_tool(PdfTool::AddText),
            CommandId::PdfToolHighlight => self.set_active_pdf_tool(PdfTool::Highlight),
            CommandId::PdfToolSticky => self.set_active_pdf_tool(PdfTool::StickyNote),
            CommandId::PdfToolRedact => self.set_active_pdf_tool(PdfTool::Redact),
            CommandId::OpenSignaturePad => {
                // Ensure we're on a PDF tab (or convert Word -> PDF Studio tab if on Word!)
                if matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_))) {
                    self.execute_command(CommandId::ConvertPdfToWord);
                }
                if matches!(self.tabs.get(self.active_tab), Some(StudioTab::Pdf(_))) {
                    self.modal = Some(ActiveModal::SignaturePad {
                        strokes: Vec::new(),
                        is_drawing: false,
                        pad_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                        signer_name: "Nour Yahyaoui".into(),
                    });
                }
            }
            CommandId::StampApproved => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                        page.annotations.push(PdfAnnotation::Stamp {
                            x: page.width_pt - 165.0,
                            y: page.height_pt - 86.0,
                            label: "APPROVED".into(),
                            color: RgbColor::EMERALD,
                        });
                        p.dirty = true;
                        self.status_toast = "Placed [APPROVED] vector stamp on PDF page".into();
                    }
                }
            }
            CommandId::PdfMergeFile => {
                if matches!(self.tabs.get(self.active_tab), Some(StudioTab::Pdf(_))) {
                    self.prompt_or_open_modal(FileModalKind::MergePdf, "");
                } else {
                    self.status_toast = "Open or switch to a PDF tab first to merge PDFs".into();
                }
            }
            CommandId::PdfSplitModal => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get(self.active_tab) {
                    let spec = format!("1-{}", p.pdf.pages.len());
                    self.modal = Some(ActiveModal::SplitRange { range_spec: spec });
                }
            }
            CommandId::PdfCompress => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let raw = save_pdf_bytes(&p.pdf, false);
                    if let Ok((_comp, before, after)) = compress_pdf_bytes(&raw) {
                        let pct = if before > 0 {
                            100usize.saturating_sub((after * 100) / before)
                        } else {
                            0
                        };
                        self.status_toast = format!(
                            "Compressed PDF streams: {:.1} KB -> {:.1} KB ({}% smaller!)",
                            before as f32 / 1024.0,
                            after as f32 / 1024.0,
                            pct
                        );
                    }
                }
            }
            CommandId::PdfRotatePage => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let idx = p.current_page;
                    p.pdf.rotate_page(idx, 90);
                    p.dirty = true;
                    self.status_toast = format!("Rotated Page {} by 90°", idx + 1);
                }
            }
            CommandId::PdfAddBlankPage => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let idx = p.current_page;
                    p.pdf.insert_blank_page(idx);
                    p.current_page = idx + 1;
                    p.dirty = true;
                    self.status_toast = format!("Inserted blank Page {}", p.current_page + 1);
                }
            }
            CommandId::PdfDeletePage => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let idx = p.current_page;
                    if p.pdf.delete_page(idx) {
                        p.current_page = p.current_page.min(p.pdf.pages.len().saturating_sub(1));
                        p.dirty = true;
                        self.status_toast = "Deleted PDF page".into();
                    } else {
                        self.status_toast = "Cannot delete the only page in a PDF".into();
                    }
                }
            }
            CommandId::OpenWatermarkModal => {
                let current_wm = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => {
                        w.doc.page_setup.watermark.clone().unwrap_or_default()
                    }
                    Some(StudioTab::Pdf(p)) => p
                        .pdf
                        .pages
                        .first()
                        .and_then(|pg| pg.watermark.clone())
                        .unwrap_or_default(),
                    _ => "CONFIDENTIAL".into(),
                };
                self.modal = Some(ActiveModal::Watermark {
                    text: if current_wm.is_empty() {
                        "CONFIDENTIAL".into()
                    } else {
                        current_wm
                    },
                });
            }
            CommandId::ToggleSplitView => {
                if self.split_secondary_tab.is_some() {
                    self.split_secondary_tab = None;
                    self.status_toast = "Closed Side-by-Side Split View".into();
                } else {
                    // Pick another non-Home tab, or spin up a companion PDF Studio tab!
                    let alt = self
                        .tabs
                        .iter()
                        .enumerate()
                        .find(|(idx, t)| *idx != self.active_tab && !matches!(t, StudioTab::HomeHub))
                        .map(|(idx, _)| idx);
                    if let Some(sec) = alt {
                        self.split_secondary_tab = Some(sec);
                    } else {
                        let pdf = if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                            word_to_pdf_document(&w.doc)
                                .unwrap_or_else(|_| PdfDocument::new_blank("Preview", 1, PaperSize::A4))
                        } else {
                            PdfDocument::new_blank("Reference PDF", 1, PaperSize::A4)
                        };
                        self.tabs
                            .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, None)));
                        self.split_secondary_tab = Some(self.tabs.len() - 1);
                    }
                    self.status_toast =
                        "Enabled Side-by-Side Split Workspace (Word + PDF)".into();
                }
            }
            CommandId::OpenSnapshotsDrawer => {
                self.modal = Some(ActiveModal::SnapshotsDrawer);
            }
            CommandId::CaptureSnapshot => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    let id = w.snapshots.capture(&w.doc, "Manual checkpoint");
                    self.status_toast =
                        format!("Saved local Time-Machine Snapshot #{id}");
                }
            }
            CommandId::OpenFindReplace => {
                self.modal = Some(ActiveModal::FindReplace {
                    find_text: String::new(),
                    replace_text: String::new(),
                    editing_replace: false,
                    matches_count: 0,
                });
            }
            CommandId::ZoomIn => self.adjust_zoom(0.1),
            CommandId::ZoomOut => self.adjust_zoom(-0.1),
            CommandId::ZoomReset => self.set_zoom(1.0),
            CommandId::OpenAbout => {
                self.modal = Some(ActiveModal::About);
            }
        }
    }

    fn set_active_pdf_tool(&mut self, tool: PdfTool) {
        if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
            p.tool = tool;
            self.status_toast = format!("PDF Studio Tool: {}", tool.label());
        }
    }

    fn adjust_zoom(&mut self, delta: f32) {
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                w.zoom = (w.zoom + delta).clamp(0.6, 1.8);
                self.status_toast = format!("Zoom: {}%", (w.zoom * 100.0).round() as i32);
            }
            Some(StudioTab::Pdf(p)) => {
                p.zoom = (p.zoom + delta).clamp(0.6, 1.8);
                self.status_toast = format!("Zoom: {}%", (p.zoom * 100.0).round() as i32);
            }
            _ => {}
        }
    }

    fn set_zoom(&mut self, z: f32) {
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => w.zoom = z,
            Some(StudioTab::Pdf(p)) => p.zoom = z,
            _ => {}
        }
        self.status_toast = "Zoom reset to 100%".into();
    }

    fn exec_word_command(&mut self, cmd: CommandId) {
        let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        w.history.record(&w.doc);
        w.dirty = true;
        let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));

        match cmd {
            CommandId::ToggleSpellcheck => {
                w.spellcheck = !w.spellcheck;
                self.status_toast = if w.spellcheck {
                    "Live offline spellcheck enabled".into()
                } else {
                    "Live offline spellcheck hidden".into()
                };
                return;
            }
            CommandId::CyclePaperSize => {
                w.doc.page_setup.paper_size = match w.doc.page_setup.paper_size {
                    PaperSize::A4 => PaperSize::Letter,
                    PaperSize::Letter => PaperSize::Legal,
                    PaperSize::Legal => PaperSize::A4,
                };
                self.status_toast =
                    format!("Paper size: {}", w.doc.page_setup.paper_size.label());
                return;
            }
            CommandId::InsertTable => {
                let insert_at = (idx + 1).min(w.doc.blocks.len());
                w.doc
                    .blocks
                    .insert(insert_at, Block::Table(TableBlock::new(3, 3)));
                w.active_table_cell = Some((insert_at, 1, 0));
                w.cursor_block = insert_at;
                self.status_toast = "Inserted 3×3 styled table".into();
                return;
            }
            CommandId::TableAddRow => {
                if let Some((tb_idx, _, _)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.add_row();
                        self.status_toast = "Added row to table".into();
                        return;
                    }
                }
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(idx) {
                    t.add_row();
                }
                return;
            }
            CommandId::TableAddCol => {
                if let Some((tb_idx, _, _)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.add_column();
                        self.status_toast = "Added column to table".into();
                        return;
                    }
                }
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(idx) {
                    t.add_column();
                }
                return;
            }
            CommandId::InsertPageBreak => {
                let insert_at = (idx + 1).min(w.doc.blocks.len());
                w.doc.blocks.insert(insert_at, Block::PageBreak);
                w.doc.blocks.insert(
                    insert_at + 1,
                    Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")),
                );
                w.cursor_block = insert_at + 1;
                w.cursor_char = 0;
                self.status_toast = "Inserted physical Page Break".into();
                return;
            }
            _ => {}
        }

        if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
            match cmd {
                CommandId::ToggleBold => {
                    let target = !p.spans.first().map(|s| s.style.bold).unwrap_or(false);
                    for s in &mut p.spans {
                        s.style.bold = target;
                    }
                }
                CommandId::ToggleItalic => {
                    let target = !p.spans.first().map(|s| s.style.italic).unwrap_or(false);
                    for s in &mut p.spans {
                        s.style.italic = target;
                    }
                }
                CommandId::ToggleUnderline => {
                    let target = !p.spans.first().map(|s| s.style.underline).unwrap_or(false);
                    for s in &mut p.spans {
                        s.style.underline = target;
                    }
                }
                CommandId::ToggleStrike => {
                    let target = !p
                        .spans
                        .first()
                        .map(|s| s.style.strikethrough)
                        .unwrap_or(false);
                    for s in &mut p.spans {
                        s.style.strikethrough = target;
                    }
                }
                CommandId::CycleStyle => {
                    p.style = match p.style {
                        ParagraphStyle::Normal => ParagraphStyle::Heading1,
                        ParagraphStyle::Heading1 => ParagraphStyle::Heading2,
                        ParagraphStyle::Heading2 => ParagraphStyle::Heading3,
                        ParagraphStyle::Heading3 => ParagraphStyle::Title,
                        ParagraphStyle::Title => ParagraphStyle::Quote,
                        ParagraphStyle::Quote | ParagraphStyle::Subtitle => ParagraphStyle::Normal,
                    };
                    let sz = p.style.default_font_size();
                    let b = p.style.is_bold_by_default();
                    for s in &mut p.spans {
                        s.style.font_size_pt = sz;
                        s.style.bold = b;
                        s.style.italic = p.style == ParagraphStyle::Quote;
                    }
                    self.status_toast = format!("Style: {}", p.style.label());
                }
                CommandId::CycleFontFamily => {
                    let next = match p
                        .spans
                        .first()
                        .map(|s| s.style.font_family)
                        .unwrap_or(FontFamily::Sans)
                    {
                        FontFamily::Sans => FontFamily::Serif,
                        FontFamily::Serif => FontFamily::Mono,
                        FontFamily::Mono => FontFamily::Sans,
                    };
                    for s in &mut p.spans {
                        s.style.font_family = next;
                    }
                    self.status_toast = format!("Font: {}", next.label());
                }
                CommandId::FontSizeUp => {
                    for s in &mut p.spans {
                        s.style.font_size_pt = (s.style.font_size_pt + 1.5).min(48.0);
                    }
                }
                CommandId::FontSizeDown => {
                    for s in &mut p.spans {
                        s.style.font_size_pt = (s.style.font_size_pt - 1.5).max(8.0);
                    }
                }
                CommandId::CycleTextColor => {
                    let cur = p
                        .spans
                        .first()
                        .map(|s| s.style.color)
                        .unwrap_or(RgbColor::INK_SLATE);
                    let next = if cur == RgbColor::INK_SLATE {
                        RgbColor::COBALT
                    } else if cur == RgbColor::COBALT {
                        RgbColor::CORAL
                    } else if cur == RgbColor::CORAL {
                        RgbColor::EMERALD
                    } else {
                        RgbColor::INK_SLATE
                    };
                    for s in &mut p.spans {
                        s.style.color = next;
                    }
                }
                CommandId::ToggleHighlight => {
                    let cur = p.spans.first().and_then(|s| s.style.highlight);
                    let next = match cur {
                        None => Some(RgbColor::AMBER_HIGHLIGHT),
                        Some(c) if c == RgbColor::AMBER_HIGHLIGHT => {
                            Some(RgbColor::MINT_HIGHLIGHT)
                        }
                        Some(c) if c == RgbColor::MINT_HIGHLIGHT => Some(RgbColor::SKY_HIGHLIGHT),
                        _ => None,
                    };
                    for s in &mut p.spans {
                        s.style.highlight = next;
                    }
                }
                CommandId::AlignLeft => p.alignment = Alignment::Left,
                CommandId::AlignCenter => p.alignment = Alignment::Center,
                CommandId::AlignRight => p.alignment = Alignment::Right,
                CommandId::ToggleBulletList => {
                    p.list_kind = match p.list_kind {
                        ListKind::Bullet => ListKind::None,
                        _ => ListKind::Bullet,
                    };
                }
                CommandId::ToggleChecklist => {
                    p.list_kind = match p.list_kind {
                        ListKind::Checklist(_) => ListKind::None,
                        _ => ListKind::Checklist(false),
                    };
                }
                _ => {}
            }
        }
    }

    pub fn dispatch_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::SwitchTab(idx) => {
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                }
            }
            UiAction::CloseTab(idx) => {
                if idx > 0 && idx < self.tabs.len() {
                    self.tabs.remove(idx);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len().saturating_sub(1);
                    }
                    if let Some(sec) = self.split_secondary_tab {
                        if sec == idx || sec >= self.tabs.len() {
                            self.split_secondary_tab = None;
                        }
                    }
                }
            }
            UiAction::OpenHomeTab => {
                self.active_tab = 0;
            }
            UiAction::OpenCommandPalette => {
                self.modal = Some(ActiveModal::CommandPalette {
                    query: String::new(),
                    selected: 0,
                });
            }
            UiAction::ExecCommand(cmd) => {
                self.execute_command(cmd);
            }
            UiAction::CreateFromTemplate(tpl) => {
                let doc = tpl.build();
                let label = tpl.label();
                self.tabs
                    .push(StudioTab::Word(WordTabState::from_doc(doc, None)));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = format!("Created document from '{label}' template");
            }
            UiAction::OpenRecentPath(path) => {
                if let Err(e) = self.open_path(&path) {
                    self.status_toast = format!("Could not open '{}': {e}", path.display());
                }
            }
            UiAction::JumpOutline {
                tab_idx,
                block_idx,
                target_scroll_y,
            } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.cursor_block = block_idx;
                    w.cursor_char = 0;
                    w.active_table_cell = None;
                    w.scroll_y = target_scroll_y.max(0.0);
                }
            }
            UiAction::ClickWordParagraph {
                tab_idx,
                block_idx,
                char_idx,
            } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.cursor_block = block_idx;
                    w.cursor_char = char_idx;
                    w.active_table_cell = None;
                }
            }
            UiAction::ToggleChecklistItem { tab_idx, block_idx } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.history.record(&w.doc);
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(block_idx) {
                        if let ListKind::Checklist(done) = p.list_kind {
                            p.list_kind = ListKind::Checklist(!done);
                            w.dirty = true;
                        }
                    }
                }
            }
            UiAction::ClickTableCell {
                tab_idx,
                block_idx,
                row,
                col,
            } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.cursor_block = block_idx;
                    w.active_table_cell = Some((block_idx, row, col));
                }
            }
            UiAction::SelectPdfPage { tab_idx, page_idx } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(tab_idx) {
                    if page_idx < p.pdf.pages.len() {
                        p.current_page = page_idx;
                        p.selected_text_id = None;
                    }
                }
            }
            UiAction::MovePdfPage { tab_idx, from, to } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(tab_idx) {
                    p.pdf.move_page(from, to);
                    p.current_page = to.min(p.pdf.pages.len().saturating_sub(1));
                    p.dirty = true;
                    self.status_toast = format!("Moved PDF page {} -> {}", from + 1, to + 1);
                }
            }
            UiAction::SetPdfTool { tab_idx, tool } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(tab_idx) {
                    p.tool = tool;
                    self.status_toast = format!("PDF Tool: {}", tool.label());
                }
            }
            UiAction::ClickPdfTextItem {
                tab_idx,
                page_idx,
                item_id,
            } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(tab_idx) {
                    p.current_page = page_idx;
                    p.selected_text_id = Some(item_id);
                    self.status_toast = "Editing PDF text block inline — type or Backspace".into();
                }
            }
            UiAction::ClickPdfPageCanvas {
                tab_idx,
                page_idx,
                pdf_x,
                pdf_y,
            } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(tab_idx) {
                    p.current_page = page_idx;
                    let tool = p.tool;
                    if let Some(page) = p.pdf.pages.get_mut(page_idx) {
                        match tool {
                            PdfTool::SelectEdit => {
                                p.selected_text_id = None;
                            }
                            PdfTool::AddText => {
                                let id = page.add_text_box(
                                    pdf_x,
                                    pdf_y,
                                    "New PDF Text",
                                    12.0,
                                    RgbColor::INK_SLATE,
                                );
                                p.selected_text_id = Some(id);
                                p.dirty = true;
                                p.tool = PdfTool::SelectEdit;
                                self.status_toast =
                                    "Inserted PDF Text Box — type to customize".into();
                            }
                            PdfTool::Highlight => {
                                page.annotations.push(PdfAnnotation::Highlight {
                                    x: pdf_x,
                                    y: (pdf_y - 4.0).max(8.0),
                                    w: 175.0,
                                    h: 18.0,
                                    color: RgbColor::AMBER_HIGHLIGHT,
                                });
                                p.dirty = true;
                                self.status_toast = "Added PDF highlight annotation".into();
                            }
                            PdfTool::StickyNote => {
                                page.annotations.push(PdfAnnotation::StickyNote {
                                    x: pdf_x,
                                    y: pdf_y,
                                    author: "Nour Yahyaoui".into(),
                                    note: "Review note added in RustPad".into(),
                                });
                                p.dirty = true;
                                self.status_toast = "Pinned Sticky Note on PDF page".into();
                            }
                            PdfTool::Redact => {
                                let scrubbed = page.apply_redaction(pdf_x, pdf_y - 8.0, 160.0, 22.0);
                                p.dirty = true;
                                self.status_toast = format!(
                                    "Applied true security redaction ({scrubbed} text object(s) scrubbed)"
                                );
                            }
                        }
                    }
                }
            }
            UiAction::RestoreSnapshot(id) => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    match w.snapshots.restore(id) {
                        Ok(restored) => {
                            w.history.record(&w.doc);
                            w.doc = restored;
                            w.cursor_block = 0;
                            w.cursor_char = 0;
                            w.dirty = true;
                            self.modal = None;
                            self.status_toast = format!("Restored Snapshot #{id}");
                        }
                        Err(e) => self.status_toast = format!("Snapshot error: {e}"),
                    }
                }
            }
            UiAction::ModalClose => {
                self.modal = None;
            }
            UiAction::ModalConfirm => {
                self.confirm_active_modal();
            }
            UiAction::SignatureClear => {
                if let Some(ActiveModal::SignaturePad { strokes, .. }) = &mut self.modal {
                    strokes.clear();
                }
            }
            UiAction::PalettePick(cmd) => {
                self.modal = None;
                self.execute_command(cmd);
            }
            UiAction::FileCandidatePick(path) => {
                if let Some(ActiveModal::FilePrompt { kind, .. }) = self.modal {
                    let k = kind;
                    self.modal = None;
                    self.apply_file_modal_choice(k, path);
                }
            }
            UiAction::FindFocusField(replace_focus) => {
                if let Some(ActiveModal::FindReplace {
                    editing_replace, ..
                }) = &mut self.modal
                {
                    *editing_replace = replace_focus;
                }
            }
            UiAction::FindReplaceAll => {
                let pair = if let Some(ActiveModal::FindReplace {
                    find_text,
                    replace_text,
                    ..
                }) = &self.modal
                {
                    Some((find_text.clone(), replace_text.clone()))
                } else {
                    None
                };
                if let Some((needle, repl)) = pair {
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                        w.history.record(&w.doc);
                        let replaced = w.doc.replace_all(&needle, &repl);
                        if replaced > 0 {
                            w.dirty = true;
                        }
                        self.status_toast = format!("Replaced {replaced} occurrence(s)");
                        if let Some(ActiveModal::FindReplace { matches_count, .. }) =
                            &mut self.modal
                        {
                            *matches_count = w.doc.find_occurrences(&needle).len();
                        }
                    }
                }
            }
        }
    }

    pub fn confirm_active_modal(&mut self) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        match modal {
            ActiveModal::CommandPalette { query, selected } => {
                let items = filter_commands(&query);
                if let Some(spec) = items.get(selected).or_else(|| items.first()) {
                    self.execute_command(spec.id);
                }
            }
            ActiveModal::FilePrompt {
                kind, input_path, ..
            } => {
                let trimmed = input_path.trim();
                if !trimmed.is_empty() {
                    self.apply_file_modal_choice(kind, PathBuf::from(trimmed));
                }
            }
            ActiveModal::SignaturePad {
                strokes,
                pad_rect,
                signer_name,
                ..
            } => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                        let mapped: Vec<Vec<(f32, f32)>> = if strokes.is_empty() {
                            // Stylish default cursive stroke if user clicks Confirm immediately
                            vec![vec![
                                (page.width_pt - 210.0, 96.0),
                                (page.width_pt - 180.0, 122.0),
                                (page.width_pt - 155.0, 94.0),
                                (page.width_pt - 122.0, 118.0),
                                (page.width_pt - 84.0, 102.0),
                            ]]
                        } else {
                            let pw = pad_rect.w.max(1.0);
                            let ph = pad_rect.h.max(1.0);
                            strokes
                                .iter()
                                .filter(|s| s.len() >= 2)
                                .map(|s| {
                                    s.iter()
                                        .map(|(sx, sy)| {
                                            let nx = ((sx - pad_rect.x) / pw).clamp(0.0, 1.0);
                                            let ny = ((sy - pad_rect.y) / ph).clamp(0.0, 1.0);
                                            (
                                                page.width_pt - 220.0 + nx * 155.0,
                                                78.0 + (1.0 - ny) * 58.0,
                                            )
                                        })
                                        .collect()
                                })
                                .collect()
                        };
                        page.annotations.push(PdfAnnotation::Signature {
                            strokes: mapped,
                            signer: signer_name,
                            color: RgbColor::COBALT,
                        });
                        p.dirty = true;
                        self.status_toast = "Placed vector ink signature on PDF page".into();
                    }
                }
            }
            ActiveModal::SplitRange { range_spec } => {
                let extracted = if let Some(StudioTab::Pdf(p)) = self.tabs.get(self.active_tab) {
                    Some(p.pdf.extract_ranges(&range_spec))
                } else {
                    None
                };
                match extracted {
                    Some(Ok(new_pdf)) => {
                        let count = new_pdf.pages.len();
                        self.tabs
                            .push(StudioTab::Pdf(PdfTabState::from_pdf(new_pdf, None)));
                        self.active_tab = self.tabs.len() - 1;
                        self.status_toast =
                            format!("Extracted {count} page(s) into a new PDF Studio tab");
                    }
                    Some(Err(e)) => self.status_toast = format!("Split range error: {e}"),
                    None => {}
                }
            }
            ActiveModal::Watermark { text } => {
                let wm = if text.trim().is_empty() {
                    None
                } else {
                    Some(text.trim().to_string())
                };
                match self.tabs.get_mut(self.active_tab) {
                    Some(StudioTab::Word(w)) => {
                        w.doc.page_setup.watermark = wm.clone();
                        w.dirty = true;
                    }
                    Some(StudioTab::Pdf(p)) => {
                        p.pdf.apply_watermark_all(wm.clone());
                        p.dirty = true;
                    }
                    _ => {}
                }
                self.status_toast = match wm {
                    Some(s) => format!("Applied diagonal watermark '{s}'"),
                    None => "Cleared page watermark".into(),
                };
            }
            ActiveModal::FindReplace { .. }
            | ActiveModal::SnapshotsDrawer
            | ActiveModal::About => {}
        }
    }

    pub fn handle_mouse_down(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;

        if let Some(ActiveModal::SignaturePad {
            strokes,
            is_drawing,
            pad_rect,
            ..
        }) = &mut self.modal
        {
            if pad_rect.contains(x, y) {
                *is_drawing = true;
                strokes.push(vec![(x, y)]);
                return;
            }
        }

        // Hit-test from top-most registered region backwards
        if let Some(hit) = self
            .hit_regions
            .iter()
            .rev()
            .find(|h| h.rect.contains(x, y))
            .cloned()
        {
            self.dispatch_ui_action(hit.action);
        }
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> bool {
        self.mouse_x = x;
        self.mouse_y = y;
        if let Some(ActiveModal::SignaturePad {
            strokes,
            is_drawing,
            pad_rect,
            ..
        }) = &mut self.modal
        {
            if *is_drawing && pad_rect.contains(x, y) {
                if let Some(last) = strokes.last_mut() {
                    let append = last
                        .last()
                        .map(|(lx, ly)| ((x - lx).powi(2) + (y - ly).powi(2)) >= 4.0)
                        .unwrap_or(true);
                    if append {
                        last.push((x, y));
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn handle_mouse_up(&mut self) {
        if let Some(ActiveModal::SignaturePad { is_drawing, .. }) = &mut self.modal {
            *is_drawing = false;
        }
    }

    pub fn handle_scroll(&mut self, delta_y: f32, ctrl: bool) {
        if ctrl {
            self.adjust_zoom(if delta_y < 0.0 { 0.1 } else { -0.1 });
            return;
        }
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                w.scroll_y = (w.scroll_y + delta_y * 42.0).max(0.0);
            }
            Some(StudioTab::Pdf(p)) => {
                p.scroll_y = (p.scroll_y + delta_y * 42.0).max(0.0);
            }
            _ => {}
        }
    }

    pub fn handle_key_down(&mut self, key: Key, mods: Modifiers) {
        // Global keyboard shortcuts
        if mods.ctrl {
            match key {
                Key::Char('k') | Key::Char('K') | Key::Char('p') | Key::Char('P') => {
                    self.dispatch_ui_action(UiAction::OpenCommandPalette);
                    return;
                }
                Key::Char('n') | Key::Char('N') => {
                    self.execute_command(CommandId::NewWordBlank);
                    return;
                }
                Key::Char('o') | Key::Char('O') => {
                    self.execute_command(CommandId::OpenFile);
                    return;
                }
                Key::Char('s') | Key::Char('S') => {
                    self.execute_command(CommandId::SaveFile);
                    return;
                }
                Key::Char('e') | Key::Char('E') => {
                    self.execute_command(CommandId::ExportWordToPdf);
                    return;
                }
                Key::Char('f') | Key::Char('F') => {
                    self.execute_command(CommandId::OpenFindReplace);
                    return;
                }
                Key::Char('b') | Key::Char('B') => {
                    self.execute_command(CommandId::ToggleBold);
                    return;
                }
                Key::Char('i') | Key::Char('I') => {
                    self.execute_command(CommandId::ToggleItalic);
                    return;
                }
                Key::Char('u') | Key::Char('U') => {
                    self.execute_command(CommandId::ToggleUnderline);
                    return;
                }
                Key::Char('\\') => {
                    self.execute_command(CommandId::ToggleSplitView);
                    return;
                }
                Key::Char('=') | Key::Char('+') => {
                    self.execute_command(CommandId::ZoomIn);
                    return;
                }
                Key::Char('-') => {
                    self.execute_command(CommandId::ZoomOut);
                    return;
                }
                Key::Char('0') => {
                    self.execute_command(CommandId::ZoomReset);
                    return;
                }
                Key::Char('z') | Key::Char('Z') => {
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                        if mods.shift {
                            if w.history.redo(&mut w.doc) {
                                self.status_toast = "Redo".into();
                            }
                        } else if w.history.undo(&mut w.doc) {
                            self.status_toast = "Undo".into();
                        }
                    }
                    return;
                }
                Key::Char('y') | Key::Char('Y') => {
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                        if w.history.redo(&mut w.doc) {
                            self.status_toast = "Redo".into();
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        // Modal keyboard navigation
        if self.modal.is_some() {
            match key {
                Key::Escape => {
                    self.modal = None;
                    return;
                }
                Key::Enter => {
                    if matches!(self.modal, Some(ActiveModal::FindReplace { .. })) {
                        self.dispatch_ui_action(UiAction::FindReplaceAll);
                    } else {
                        self.confirm_active_modal();
                    }
                    return;
                }
                Key::Tab => {
                    if let Some(ActiveModal::FindReplace {
                        editing_replace, ..
                    }) = &mut self.modal
                    {
                        *editing_replace = !*editing_replace;
                    }
                    return;
                }
                Key::Up => {
                    if let Some(ActiveModal::CommandPalette { selected, .. }) = &mut self.modal {
                        *selected = selected.saturating_sub(1);
                    }
                    return;
                }
                Key::Down => {
                    if let Some(ActiveModal::CommandPalette { query, selected }) = &mut self.modal {
                        let len = filter_commands(query).len();
                        if len > 0 {
                            *selected = (*selected + 1).min(len - 1);
                        }
                    }
                    return;
                }
                Key::Backspace => {
                    self.backspace_modal();
                    return;
                }
                _ => return,
            }
        }

        // Active document editing
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => Self::word_key_down(w, key),
            Some(StudioTab::Pdf(p)) => Self::pdf_key_down(p, key),
            _ => {}
        }
    }

    fn backspace_modal(&mut self) {
        match &mut self.modal {
            Some(ActiveModal::CommandPalette { query, selected }) => {
                query.pop();
                *selected = 0;
            }
            Some(ActiveModal::FilePrompt { input_path, .. }) => {
                input_path.pop();
            }
            Some(ActiveModal::FindReplace {
                find_text,
                replace_text,
                editing_replace,
                matches_count,
            }) => {
                if *editing_replace {
                    replace_text.pop();
                } else {
                    find_text.pop();
                    if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                        *matches_count = w.doc.find_occurrences(find_text).len();
                    }
                }
            }
            Some(ActiveModal::SplitRange { range_spec }) => {
                range_spec.pop();
            }
            Some(ActiveModal::Watermark { text }) => {
                text.pop();
            }
            _ => {}
        }
    }

    pub fn handle_text_input(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        if let Some(modal) = &mut self.modal {
            match modal {
                ActiveModal::CommandPalette { query, selected } => {
                    query.push(ch);
                    *selected = 0;
                }
                ActiveModal::FilePrompt { input_path, .. } => {
                    input_path.push(ch);
                }
                ActiveModal::FindReplace {
                    find_text,
                    replace_text,
                    editing_replace,
                    matches_count,
                } => {
                    if *editing_replace {
                        replace_text.push(ch);
                    } else {
                        find_text.push(ch);
                        if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                            *matches_count = w.doc.find_occurrences(find_text).len();
                        }
                    }
                }
                ActiveModal::SplitRange { range_spec } => range_spec.push(ch),
                ActiveModal::Watermark { text } => text.push(ch),
                _ => {}
            }
            return;
        }

        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                w.history.record(&w.doc);
                w.dirty = true;
                if let Some((b_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(b_idx) {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            cell.text.push(ch);
                            return;
                        }
                    }
                }
                w.doc.ensure_non_empty();
                let b_idx = w.cursor_block.min(w.doc.blocks.len() - 1);
                if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(b_idx) {
                    insert_char_in_paragraph(p, w.cursor_char, ch);
                    w.cursor_char += 1;
                }
            }
            Some(StudioTab::Pdf(p)) => {
                if let Some(sel_id) = p.selected_text_id {
                    if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                        if let Some(item) = page.text_items.iter_mut().find(|i| i.id == sel_id) {
                            item.text.push(ch);
                            p.dirty = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn word_key_down(w: &mut WordTabState, key: Key) {
        if let Some((b_idx, r, c)) = w.active_table_cell {
            if let Some(Block::Table(t)) = w.doc.blocks.get_mut(b_idx) {
                match key {
                    Key::Backspace => {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            cell.text.pop();
                            w.dirty = true;
                        }
                    }
                    Key::Tab => {
                        let next_c = (c + 1) % t.cols.max(1);
                        let next_r = if next_c == 0 {
                            (r + 1) % t.rows.len().max(1)
                        } else {
                            r
                        };
                        w.active_table_cell = Some((b_idx, next_r, next_c));
                    }
                    Key::Escape => w.active_table_cell = None,
                    _ => {}
                }
                return;
            }
        }

        w.doc.ensure_non_empty();
        let b_idx = w.cursor_block.min(w.doc.blocks.len() - 1);
        w.cursor_block = b_idx;

        match key {
            Key::Left => {
                if w.cursor_char > 0 {
                    w.cursor_char -= 1;
                } else if b_idx > 0 {
                    w.cursor_block = b_idx - 1;
                    w.cursor_char = block_char_len(&w.doc.blocks[w.cursor_block]);
                }
            }
            Key::Right => {
                let len = block_char_len(&w.doc.blocks[b_idx]);
                if w.cursor_char < len {
                    w.cursor_char += 1;
                } else if b_idx + 1 < w.doc.blocks.len() {
                    w.cursor_block = b_idx + 1;
                    w.cursor_char = 0;
                }
            }
            Key::Up => {
                if b_idx > 0 {
                    w.cursor_block = b_idx - 1;
                    let len = block_char_len(&w.doc.blocks[w.cursor_block]);
                    w.cursor_char = w.cursor_char.min(len);
                }
            }
            Key::Down => {
                if b_idx + 1 < w.doc.blocks.len() {
                    w.cursor_block = b_idx + 1;
                    let len = block_char_len(&w.doc.blocks[w.cursor_block]);
                    w.cursor_char = w.cursor_char.min(len);
                }
            }
            Key::Home => w.cursor_char = 0,
            Key::End => w.cursor_char = block_char_len(&w.doc.blocks[b_idx]),
            Key::PageUp => w.scroll_y = (w.scroll_y - 360.0).max(0.0),
            Key::PageDown => w.scroll_y += 360.0,
            Key::Backspace => {
                w.history.record(&w.doc);
                if w.cursor_char > 0 {
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(b_idx) {
                        remove_char_in_paragraph(p, w.cursor_char - 1);
                        w.cursor_char -= 1;
                        w.dirty = true;
                    }
                } else if b_idx > 0 {
                    let cur_text = match &w.doc.blocks[b_idx] {
                        Block::Paragraph(p) => Some(p.plain_text()),
                        _ => None,
                    };
                    if let Some(tail) = cur_text {
                        if let Some(Block::Paragraph(prev_p)) =
                            w.doc.blocks.get_mut(b_idx - 1)
                        {
                            let join_col = prev_p.plain_text().chars().count();
                            if let Some(last) = prev_p.spans.last_mut() {
                                last.text.push_str(&tail);
                            }
                            w.doc.blocks.remove(b_idx);
                            w.cursor_block = b_idx - 1;
                            w.cursor_char = join_col;
                            w.dirty = true;
                        }
                    }
                }
            }
            Key::Delete => {
                w.history.record(&w.doc);
                let len = block_char_len(&w.doc.blocks[b_idx]);
                if w.cursor_char < len {
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(b_idx) {
                        remove_char_in_paragraph(p, w.cursor_char);
                        w.dirty = true;
                    }
                }
            }
            Key::Enter => {
                w.history.record(&w.doc);
                if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(b_idx) {
                    let full = p.plain_text();
                    let chars: Vec<char> = full.chars().collect();
                    let split = w.cursor_char.min(chars.len());
                    let left: String = chars[..split].iter().collect();
                    let right: String = chars[split..].iter().collect();
                    let base_style = p.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                    let next_list = match p.list_kind {
                        ListKind::Bullet => ListKind::Bullet,
                        ListKind::Checklist(_) => ListKind::Checklist(false),
                        ListKind::Numbered(n) => ListKind::Numbered(n + 1),
                        ListKind::None => ListKind::None,
                    };
                    p.spans = vec![doc_model::TextSpan::styled(left, base_style.clone())];
                    let mut new_para = Paragraph::new(ParagraphStyle::Normal, right);
                    new_para.list_kind = next_list;
                    if let Some(first_span) = new_para.spans.first_mut() {
                        first_span.style.font_family = base_style.font_family;
                    }
                    w.doc.blocks.insert(b_idx + 1, Block::Paragraph(new_para));
                    w.cursor_block = b_idx + 1;
                    w.cursor_char = 0;
                    w.dirty = true;
                }
            }
            _ => {}
        }
    }

    fn pdf_key_down(p: &mut PdfTabState, key: Key) {
        match key {
            Key::Backspace => {
                if let Some(sel_id) = p.selected_text_id {
                    if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                        if let Some(item) = page.text_items.iter_mut().find(|i| i.id == sel_id) {
                            item.text.pop();
                            p.dirty = true;
                        }
                    }
                }
            }
            Key::Escape => {
                p.selected_text_id = None;
                p.tool = PdfTool::SelectEdit;
            }
            Key::PageDown | Key::Down if p.selected_text_id.is_none() => {
                if p.current_page + 1 < p.pdf.pages.len() {
                    p.current_page += 1;
                }
            }
            Key::PageUp | Key::Up if p.selected_text_id.is_none() => {
                p.current_page = p.current_page.saturating_sub(1);
            }
            _ => {}
        }
    }
}

fn block_char_len(b: &Block) -> usize {
    match b {
        Block::Paragraph(p) => p.plain_text().chars().count(),
        _ => 0,
    }
}

fn insert_char_in_paragraph(p: &mut Paragraph, char_idx: usize, ch: char) {
    p.normalize_spans();
    let mut rem = char_idx;
    for span in &mut p.spans {
        let count = span.text.chars().count();
        if rem <= count {
            let byte_idx = span
                .text
                .char_indices()
                .nth(rem)
                .map(|(i, _)| i)
                .unwrap_or(span.text.len());
            span.text.insert(byte_idx, ch);
            return;
        }
        rem -= count;
    }
    if let Some(last) = p.spans.last_mut() {
        last.text.push(ch);
    }
}

fn remove_char_in_paragraph(p: &mut Paragraph, char_idx: usize) {
    p.normalize_spans();
    let mut rem = char_idx;
    for span in &mut p.spans {
        let count = span.text.chars().count();
        if rem < count {
            if let Some((byte_idx, _)) = span.text.char_indices().nth(rem) {
                span.text.remove(byte_idx);
            }
            p.normalize_spans();
            return;
        }
        rem -= count;
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.trim_matches('_').is_empty() {
        "Document".into()
    } else {
        cleaned.trim_matches('_').to_string()
    }
}

fn discover_workspace_documents() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [".", "examples"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    let low = ext.to_ascii_lowercase();
                    if matches!(low.as_str(), "docx" | "pdf") {
                        found.push(p);
                    }
                }
            }
        }
    }
    found.sort();
    found.truncate(8);
    found
}
