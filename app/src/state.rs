//! Application State, Multi-Tab Session Controller, Resizable Panels, Interactive Tables,
//! Non-Blocking Pluggable AI Agent Dock, Clipboard & Hit-Testing Actions.

use crate::commands::{filter_commands, CommandId};
use doc_model::{
    next_word_char_idx, prev_word_char_idx, word_range_at_char, Alignment, Block, Document,
    FontFamily, ImageBlock, ListKind, PaperSize, Paragraph, ParagraphStyle, RevisionKind,
    RevisionMeta, RgbColor, SpanStyle, TableBlock, TablePreset, TableTheme, TemplateKind, TextSpan,
    UndoHistory,
};
use docx_io::{load_docx_file, save_docx_file};
use pdf_create::{export_document_to_pdf_file, PdfCreateOptions};
use pdf_engine::{
    compress_pdf_bytes, load_pdf_file, save_pdf_bytes, save_pdf_file, seal_pdf_bytes,
    PdfAnnotation, PdfDocument,
};
use power_tools::{
    discover_local_ollama_models, markdown_to_word, parse_ai_output_to_blocks, pdf_to_word,
    run_ai_task_with_policy, word_to_markdown, word_to_pdf_document, AdvisoryFileLock,
    AiAgentConfig, AiProviderPreset, AiResponse, AiTask, AuditTrail, EnterprisePolicy,
    SnapshotVault, SpellChecker, WalJournal,
};
use rustpad_render::{decode_any_image_rgba, encode_rgba_png, FontBook, Rect};
use rustpad_window::{
    detect_dpi_scale, get_clipboard_image_bytes, get_clipboard_text, set_clipboard_text,
    spool_pdf_to_os_printer, try_native_open_dialog, try_native_open_image_dialog,
    try_native_save_dialog, CursorShape, Key, Modifiers,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

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
            Self::SelectEdit => "Select / Edit",
            Self::AddText => "+ Text Box",
            Self::Highlight => "Highlight",
            Self::StickyNote => "Note",
            Self::Redact => "Redact",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordLineHitBox {
    pub tab_idx: usize,
    pub block_idx: usize,
    pub rect: Rect,
    pub start_char: usize,
    pub char_xs: Vec<f32>,
}

impl WordLineHitBox {
    pub fn hit_test_char(&self, mouse_x: f32) -> usize {
        if self.char_xs.is_empty() {
            return self.start_char;
        }
        let mut best_rel = 0usize;
        let mut best_dist = f32::MAX;
        for (i, &cx) in self.char_xs.iter().enumerate() {
            let d = (mouse_x - cx).abs();
            if d < best_dist {
                best_dist = d;
                best_rel = i;
            }
        }
        self.start_char + best_rel
    }
}

pub struct WordTabState {
    pub doc: Document,
    pub path: Option<PathBuf>,
    pub cursor_block: usize,
    pub cursor_char: usize,
    pub selection_anchor: Option<(usize, usize)>, // (block_idx, char_idx)
    pub active_table_cell: Option<(usize, usize, usize)>, // (block_idx, row, col)
    pub selected_image_block: Option<usize>,
    pub table_cursor_char: usize,
    pub table_col_weights: Vec<(usize, Vec<f32>)>,
    pub image_rgba_cache: Vec<(u64, u32, u32, Vec<u8>)>, // (content_hash, w, h, rgba)
    pub scroll_y: f32,
    pub target_scroll_y: f32,
    pub max_scroll_y: f32,
    pub scroll_to_caret: bool,
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
            selection_anchor: None,
            active_table_cell: None,
            selected_image_block: None,
            table_cursor_char: 0,
            table_col_weights: Vec::new(),
            image_rgba_cache: Vec::new(),
            scroll_y: 0.0,
            target_scroll_y: 0.0,
            max_scroll_y: 0.0,
            scroll_to_caret: false,
            zoom: 1.12, // Comfortable 112% editorial default zoom
            history: UndoHistory::default(),
            snapshots,
            spellcheck: false,
            dirty: false,
        }
    }

    /// Returns ordered `((start_block, start_char), (end_block, end_char))` when a non-empty range is selected.
    pub fn ordered_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let (ab, ac) = self.selection_anchor?;
        let (fb, fc) = (self.cursor_block, self.cursor_char);
        if (ab, ac) == (fb, fc) {
            return None;
        }
        if (ab, ac) < (fb, fc) {
            Some(((ab, ac), (fb, fc)))
        } else {
            Some(((fb, fc), (ab, ac)))
        }
    }

    pub fn has_selection(&self) -> bool {
        self.ordered_selection().is_some()
    }

    /// Returns `(start_char, end_char)` within `block_idx` if covered by the active selection.
    pub fn block_selection_char_range(
        &self,
        block_idx: usize,
        block_char_len: usize,
    ) -> Option<(usize, usize)> {
        let ((b0, c0), (b1, c1)) = self.ordered_selection()?;
        if block_idx < b0 || block_idx > b1 {
            return None;
        }
        let s = if block_idx == b0 {
            c0.min(block_char_len)
        } else {
            0
        };
        let e = if block_idx == b1 {
            c1.min(block_char_len)
        } else {
            block_char_len
        };
        if s < e || (b0 < b1 && block_idx < b1) {
            Some((s, e))
        } else {
            None
        }
    }

    /// Extracts the plain text of the active selection across single or multiple paragraphs.
    pub fn selected_text(&self) -> Option<String> {
        let ((b0, c0), (b1, c1)) = self.ordered_selection()?;
        let mut parts: Vec<String> = Vec::new();
        for bi in b0..=b1.min(self.doc.blocks.len().saturating_sub(1)) {
            match self.doc.blocks.get(bi) {
                Some(Block::Paragraph(p)) => {
                    let full = p.plain_text();
                    let len = full.chars().count();
                    let s = if bi == b0 { c0.min(len) } else { 0 };
                    let e = if bi == b1 { c1.min(len) } else { len };
                    if s <= e {
                        let sub: String = full.chars().skip(s).take(e - s).collect();
                        parts.push(sub);
                    }
                }
                Some(Block::Table(t)) => {
                    for row in &t.rows {
                        let line: Vec<&str> = row.iter().map(|c| c.text.as_str()).collect();
                        parts.push(line.join("\t"));
                    }
                }
                _ => {}
            }
        }
        let out = parts.join("\n");
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Selects the entire document from the first paragraph to the final character.
    pub fn select_all(&mut self) {
        if self.doc.blocks.is_empty() {
            return;
        }
        self.active_table_cell = None;
        let last_b = self.doc.blocks.len().saturating_sub(1);
        let last_c = match self.doc.blocks.get(last_b) {
            Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
            _ => 0,
        };
        self.selection_anchor = Some((0, 0));
        self.cursor_block = last_b;
        self.cursor_char = last_c;
    }

    /// Deletes the active selection atomically (respecting Track Changes within single paragraphs).
    pub fn delete_selection(&mut self) -> bool {
        let Some(((b0, c0), (b1, c1))) = self.ordered_selection() else {
            return false;
        };
        if b0 >= self.doc.blocks.len() {
            self.selection_anchor = None;
            return false;
        }
        let b1 = b1.min(self.doc.blocks.len() - 1);
        let track = self.doc.track_changes;
        let author = self.doc.author.clone();
        let rev_id = if track { self.doc.next_revision_id() } else { 1 };

        if b0 == b1 {
            if let Some(Block::Paragraph(p)) = self.doc.blocks.get_mut(b0) {
                p.delete_char_range(c0, c1, track, rev_id, &author);
            }
            self.cursor_block = b0;
            self.cursor_char = c0;
            self.selection_anchor = None;
            self.dirty = true;
            return true;
        }

        // Multi-block selection: preserve prefix of b0 + suffix of b1, remove intermediate blocks
        let right_tail_spans: Vec<TextSpan> = match self.doc.blocks.get(b1) {
            Some(Block::Paragraph(p_end)) => {
                let mut tail = Vec::new();
                let mut cur = 0usize;
                for sp in &p_end.spans {
                    let len = sp.text.chars().count();
                    if cur + len > c1 {
                        let skip = c1.saturating_sub(cur);
                        let sub: String = sp.text.chars().skip(skip).collect();
                        if !sub.is_empty() {
                            tail.push(TextSpan {
                                text: sub,
                                style: sp.style.clone(),
                                revision: sp.revision.clone(),
                            });
                        }
                    }
                    cur += len;
                }
                tail
            }
            _ => Vec::new(),
        };

        if let Some(Block::Paragraph(p_start)) = self.doc.blocks.get_mut(b0) {
            let start_len = p_start.plain_text().chars().count();
            if c0 < start_len {
                p_start.delete_char_range(c0, start_len, false, rev_id, &author);
            }
            if p_start.plain_text().is_empty() && !right_tail_spans.is_empty() {
                p_start.spans = right_tail_spans;
            } else {
                p_start.spans.extend(right_tail_spans);
            }
            p_start.normalize_spans();
        }

        for remove_idx in ((b0 + 1)..=b1).rev() {
            if remove_idx < self.doc.blocks.len() {
                self.doc.blocks.remove(remove_idx);
            }
        }
        self.doc.ensure_non_empty();
        self.cursor_block = b0.min(self.doc.blocks.len() - 1);
        self.cursor_char = c0;
        self.selection_anchor = None;
        self.image_rgba_cache.clear();
        self.dirty = true;
        true
    }

    pub fn normalized_col_weights(&self, block_idx: usize, cols: usize) -> Vec<f32> {
        let c = cols.max(1);
        if let Some((_, w)) = self.table_col_weights.iter().find(|(b, _)| *b == block_idx) {
            if w.len() == c {
                let sum: f32 = w.iter().copied().sum::<f32>().max(0.01);
                return w.iter().map(|v| v / sum).collect();
            }
        }
        vec![1.0 / (c as f32); c]
    }

    pub fn adjust_col_divider(
        &mut self,
        block_idx: usize,
        cols: usize,
        col_idx: usize,
        delta_ratio: f32,
    ) {
        if cols < 2 || col_idx + 1 >= cols {
            return;
        }
        let mut weights = self.normalized_col_weights(block_idx, cols);
        let min_w = (0.10f32).min(0.65 / cols as f32);
        let pair_sum = weights[col_idx] + weights[col_idx + 1];
        let next_left = (weights[col_idx] + delta_ratio).clamp(min_w, (pair_sum - min_w).max(min_w));
        let next_right = (pair_sum - next_left).max(min_w);
        weights[col_idx] = next_left;
        weights[col_idx + 1] = next_right;

        if let Some((_, slot)) = self
            .table_col_weights
            .iter_mut()
            .find(|(b, _)| *b == block_idx)
        {
            *slot = weights;
        } else {
            self.table_col_weights.push((block_idx, weights));
        }
    }

    /// Insert a `TableBlock` smoothly at the cursor (replacing the current block if it's an
    /// empty paragraph), preserve `table_col_weights` alignment, and guarantee a trailing
    /// prose paragraph below the table so `Esc` / `Down` exits without friction.
    pub fn insert_table_smart(
        &mut self,
        table: TableBlock,
        col_weights: Option<Vec<f32>>,
        replace_if_empty_para: bool,
        focus_row: usize,
    ) -> usize {
        let rows_len = table.rows.len().max(1);
        let idx = self.cursor_block.min(self.doc.blocks.len().saturating_sub(1));
        let replace_here = replace_if_empty_para
            && matches!(
                self.doc.blocks.get(idx),
                Some(Block::Paragraph(p))
                    if p.list_kind == ListKind::None && p.plain_text().trim().is_empty()
            );

        let tb_idx = if replace_here {
            self.table_col_weights.retain(|(b, _)| *b != idx);
            self.doc.blocks[idx] = Block::Table(table);
            idx
        } else {
            let insert_at = (idx + 1).min(self.doc.blocks.len());
            for (b_idx, _) in &mut self.table_col_weights {
                if *b_idx >= insert_at {
                    *b_idx += 1;
                }
            }
            self.doc.blocks.insert(insert_at, Block::Table(table));
            insert_at
        };

        if let Some(weights) = col_weights {
            self.table_col_weights.retain(|(b, _)| *b != tb_idx);
            self.table_col_weights.push((tb_idx, weights));
        }

        if tb_idx + 1 >= self.doc.blocks.len() {
            self.doc
                .blocks
                .push(Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")));
        }

        let target_r = focus_row.min(rows_len - 1);
        self.cursor_block = tb_idx;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.selected_image_block = None;
        self.active_table_cell = Some((tb_idx, target_r, 0));
        self.table_cursor_char = 0;
        self.dirty = true;
        self.scroll_to_caret = true;
        tb_idx
    }

    pub fn content_width_pt(&self) -> f32 {
        let (pw, _) = self
            .doc
            .page_setup
            .paper_size
            .dimensions_pt(self.doc.page_setup.landscape);
        (pw - self.doc.page_setup.margin_left_pt - self.doc.page_setup.margin_right_pt).max(160.0)
    }

    /// Insert an `ImageBlock` smoothly at the cursor (replacing an empty paragraph in-place),
    /// preserve `table_col_weights`, ensure a trailing prose paragraph below, and select the figure.
    pub fn insert_image_smart(&mut self, img: ImageBlock, replace_if_empty_para: bool) -> usize {
        let idx = self.cursor_block.min(self.doc.blocks.len().saturating_sub(1));
        let replace_here = replace_if_empty_para
            && matches!(
                self.doc.blocks.get(idx),
                Some(Block::Paragraph(p))
                    if p.list_kind == ListKind::None && p.plain_text().trim().is_empty()
            );

        let img_idx = if replace_here {
            self.doc.blocks[idx] = Block::Image(img);
            idx
        } else {
            let insert_at = (idx + 1).min(self.doc.blocks.len());
            for (b_idx, _) in &mut self.table_col_weights {
                if *b_idx >= insert_at {
                    *b_idx += 1;
                }
            }
            self.doc.blocks.insert(insert_at, Block::Image(img));
            insert_at
        };

        if img_idx + 1 >= self.doc.blocks.len() {
            self.doc
                .blocks
                .push(Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")));
        }

        self.cursor_block = img_idx;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.active_table_cell = None;
        self.selected_image_block = Some(img_idx);
        self.dirty = true;
        self.scroll_to_caret = true;
        img_idx
    }

    pub fn delete_image_at(&mut self, img_idx: usize) -> bool {
        if !matches!(self.doc.blocks.get(img_idx), Some(Block::Image(_))) {
            return false;
        }
        self.doc.blocks.remove(img_idx);
        for (b_idx, _) in &mut self.table_col_weights {
            if *b_idx > img_idx {
                *b_idx -= 1;
            }
        }
        self.doc.ensure_non_empty();
        self.selected_image_block = None;
        self.cursor_block = img_idx.min(self.doc.blocks.len().saturating_sub(1));
        self.cursor_char = 0;
        self.dirty = true;
        true
    }
}

pub struct PdfTabState {
    pub pdf: PdfDocument,
    pub path: Option<PathBuf>,
    pub current_page: usize,
    pub selected_text_id: Option<u32>,
    pub tool: PdfTool,
    pub scroll_y: f32,
    pub target_scroll_y: f32,
    pub max_scroll_y: f32,
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
            target_scroll_y: 0.0,
            max_scroll_y: 0.0,
            zoom: 1.08,
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
    InsertImage,
    ReplaceImage,
}

#[derive(Clone)]
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
    AddComment {
        text: String,
    },
    EditImageCaption {
        block_idx: usize,
        text: String,
    },
    SnapshotsDrawer,
    Governance,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarDropdown {
    ParagraphStyle { anchor_x: i32, anchor_y: i32 },
    FontFamily { anchor_x: i32, anchor_y: i32 },
    TableCreator {
        anchor_x: i32,
        anchor_y: i32,
        rows: usize,
        cols: usize,
    },
}

impl ToolbarDropdown {
    pub fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::ParagraphStyle { .. }, Self::ParagraphStyle { .. })
                | (Self::FontFamily { .. }, Self::FontFamily { .. })
                | (Self::TableCreator { .. }, Self::TableCreator { .. })
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiInputField {
    EndpointUrl,
    Model,
    ApiKey,
    CustomPrompt,
}

pub struct AiDockState {
    pub open: bool,
    pub width: f32,
    pub config: AiAgentConfig,
    pub selected_task: AiTask,
    pub focused_field: Option<AiInputField>,
    pub custom_prompt: String,
    pub output_text: String,
    pub status_line: String,
    pub is_error: bool,
    pub last_latency_ms: Option<u128>,
    pub discovered_models: Vec<String>,
    pub output_scroll_y: f32,
    pub worker_rx: Option<Receiver<Result<AiResponse, String>>>,
    pub started_at: Option<Instant>,
}

impl Default for AiDockState {
    fn default() -> Self {
        Self {
            open: false,
            width: 344.0,
            config: AiAgentConfig::default(),
            selected_task: AiTask::FixGrammar,
            focused_field: None,
            custom_prompt: String::new(),
            output_text: String::new(),
            status_line: "Ready · Local-first (Ollama / nour-code / Cloud)".into(),
            is_error: false,
            last_latency_ms: None,
            discovered_models: Vec::new(),
            output_scroll_y: 0.0,
            worker_rx: None,
            started_at: None,
        }
    }
}

impl AiDockState {
    pub fn is_busy(&self) -> bool {
        self.worker_rx.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragInteraction {
    None,
    WordOutlineSplitter { pane_left: f32 },
    AiDockSplitter { window_right: f32 },
    PdfLeftSplitter { pane_left: f32 },
    PdfRightSplitter { pane_right: f32 },
    WorkspaceSplit { work_x: f32, work_w: f32 },
    TableColumn {
        tab_idx: usize,
        block_idx: usize,
        cols: usize,
        col_idx: usize,
        last_x: f32,
        table_px_w: f32,
    },
    Scrollbar {
        tab_idx: usize,
        track_y: f32,
        track_h: f32,
        max_scroll: f32,
    },
    WordTextSelection {
        tab_idx: usize,
    },
    ImageResize {
        tab_idx: usize,
        block_idx: usize,
        start_x: f32,
        start_width_pt: f32,
        scale: f32,
        anchor_factor: f32,
        max_width_pt: f32,
    },
}

#[derive(Debug, Clone)]
pub enum UiAction {
    SwitchTab(usize),
    CloseTab(usize),
    OpenHomeTab,
    OpenCommandPalette,
    ExecCommand(CommandId),
    ToggleDropdown(ToolbarDropdown),
    CloseDropdown,
    SelectParagraphStyle(ParagraphStyle),
    SelectFontFamily(FontFamily),
    InsertTableCustom { rows: usize, cols: usize },
    InsertTablePreset(TablePreset),
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
    SelectWordImage {
        tab_idx: usize,
        block_idx: usize,
    },
    AcceptRevisionById {
        tab_idx: usize,
        rev_id: u32,
    },
    RejectRevisionById {
        tab_idx: usize,
        rev_id: u32,
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
        char_idx: usize,
    },
    TableQuickAddRow {
        tab_idx: usize,
        block_idx: usize,
    },
    TableQuickAddCol {
        tab_idx: usize,
        block_idx: usize,
    },
    BeginDrag(DragInteraction),
    TogglePdfRightInspector,
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
    AiSelectProvider(AiProviderPreset),
    AiSelectTask(AiTask),
    AiFocusField(Option<AiInputField>),
    AiDetectOllamaModels,
    AiPickDiscoveredModel(String),
    AiRunCurrentTask,
    AiCancelTask,
    AiApplyOutput {
        replace_current: bool,
    },
    AiCopyOutput,
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
    pub cursor: CursorShape,
}

impl HitRegion {
    pub fn button(rect: Rect, action: UiAction) -> Self {
        Self {
            rect,
            action,
            cursor: CursorShape::Hand,
        }
    }

    pub fn text(rect: Rect, action: UiAction) -> Self {
        Self {
            rect,
            action,
            cursor: CursorShape::IBeam,
        }
    }

    pub fn splitter(rect: Rect, action: UiAction) -> Self {
        Self {
            rect,
            action,
            cursor: CursorShape::ResizeEW,
        }
    }
}

pub struct RustPadApp {
    pub fonts: FontBook,
    pub spellchecker: SpellChecker,
    pub policy: EnterprisePolicy,
    pub policy_path: Option<PathBuf>,
    pub audit_trail: AuditTrail,
    pub tabs: Vec<StudioTab>,
    pub active_tab: usize,
    pub split_secondary_tab: Option<usize>,
    pub split_ratio: f32,
    pub ui_scale: f32,
    pub focus_mode: bool,
    pub word_sidebar_open: bool,
    pub word_sidebar_w: f32,
    pub pdf_left_open: bool,
    pub pdf_left_w: f32,
    pub pdf_right_open: bool,
    pub pdf_right_w: f32,
    pub ai_dock: AiDockState,
    pub ai_dock_rect: Option<Rect>,
    pub open_dropdown: Option<ToolbarDropdown>,
    pub active_drag: DragInteraction,
    pub modal: Option<ActiveModal>,
    pub hit_regions: Vec<HitRegion>,
    pub word_line_boxes: Vec<WordLineHitBox>,
    pub hovered_hit_idx: Option<usize>,
    pub desired_cursor: CursorShape,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub last_click_at: Option<Instant>,
    pub last_click_pos: (f32, f32),
    pub last_click_mods: Modifiers,
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
        let dpi_scale = detect_dpi_scale().clamp(0.95, 1.45);
        let (policy, policy_path) = EnterprisePolicy::load_system_or_default();
        let mut audit_trail = AuditTrail::default();
        if policy.audit_log_enabled {
            audit_trail.record("STUDIO_BOOT", &policy.organization, "Nour Yahyaoui");
        }
        Self {
            fonts: FontBook::new(),
            spellchecker: SpellChecker::new(),
            policy,
            policy_path,
            audit_trail,
            tabs: vec![
                StudioTab::HomeHub,
                StudioTab::Word(WordTabState::from_doc(welcome_doc, None)),
            ],
            active_tab: 1,
            split_secondary_tab: None,
            split_ratio: 0.50,
            ui_scale: dpi_scale,
            focus_mode: false,
            word_sidebar_open: true,
            word_sidebar_w: 246.0,
            pdf_left_open: true,
            pdf_left_w: 196.0,
            pdf_right_open: true,
            pdf_right_w: 236.0,
            ai_dock: AiDockState::default(),
            ai_dock_rect: None,
            open_dropdown: None,
            active_drag: DragInteraction::None,
            modal: None,
            hit_regions: Vec::with_capacity(256),
            word_line_boxes: Vec::with_capacity(192),
            hovered_hit_idx: None,
            desired_cursor: CursorShape::Arrow,
            mouse_x: -1.0,
            mouse_y: -1.0,
            last_click_at: None,
            last_click_pos: (-100.0, -100.0),
            last_click_mods: Modifiers::default(),
            cursor_blink_on: true,
            status_toast: "Ready · RustPad v2.2 Effortless Table & Figure Studio · 60–120Hz Smooth UX".into(),
            recent_files: recent,
        }
    }

    /// Steps 60Hz kinetic spring scrolling AND polls non-blocking background AI worker threads.
    /// Returns `true` if a repaint is needed.
    pub fn poll_background_tasks(&mut self) -> bool {
        let mut needs_repaint = false;

        // 60Hz critically-damped spring interpolation for silky-smooth document & PDF scrolling
        for (ti, tab) in self.tabs.iter_mut().enumerate() {
            if ti != self.active_tab && self.split_secondary_tab != Some(ti) {
                continue;
            }
            match tab {
                StudioTab::Word(w) => {
                    let target = w.target_scroll_y.clamp(0.0, w.max_scroll_y.max(0.0));
                    w.target_scroll_y = target;
                    let diff = target - w.scroll_y;
                    if diff.abs() > 0.35 {
                        w.scroll_y += diff * 0.34;
                        needs_repaint = true;
                    } else if diff.abs() > 0.01 {
                        w.scroll_y = target;
                        needs_repaint = true;
                    }
                }
                StudioTab::Pdf(p) => {
                    let target = p.target_scroll_y.clamp(0.0, p.max_scroll_y.max(0.0));
                    p.target_scroll_y = target;
                    let diff = target - p.scroll_y;
                    if diff.abs() > 0.35 {
                        p.scroll_y += diff * 0.34;
                        needs_repaint = true;
                    } else if diff.abs() > 0.01 {
                        p.scroll_y = target;
                        needs_repaint = true;
                    }
                }
                StudioTab::HomeHub => {}
            }
        }

        let Some(rx) = &self.ai_dock.worker_rx else {
            return needs_repaint;
        };
        match rx.try_recv() {
            Ok(Ok(resp)) => {
                self.ai_dock.worker_rx = None;
                self.ai_dock.started_at = None;
                self.ai_dock.output_text = resp.content;
                self.ai_dock.output_scroll_y = 0.0;
                self.ai_dock.last_latency_ms = Some(resp.latency_ms);
                self.ai_dock.is_error = false;
                self.ai_dock.status_line = format!(
                    "Completed via {} ({:.2}s)",
                    resp.model_used,
                    resp.latency_ms as f32 / 1000.0
                );
                self.status_toast = format!(
                    "AI Agent finished '{}' in {:.2}s — review & click Insert or Replace",
                    self.ai_dock.selected_task.short_label(),
                    resp.latency_ms as f32 / 1000.0
                );
                true
            }
            Ok(Err(err)) => {
                self.ai_dock.worker_rx = None;
                self.ai_dock.started_at = None;
                self.ai_dock.is_error = true;
                self.ai_dock.status_line = err.clone();
                self.status_toast = format!("AI Agent error: {err}");
                true
            }
            Err(TryRecvError::Empty) => {
                if let Some(t0) = self.ai_dock.started_at {
                    let secs = t0.elapsed().as_secs_f32();
                    self.ai_dock.status_line = format!(
                        "Agent running on {}… ({:.1}s)",
                        self.ai_dock.config.preset.short_badge(),
                        secs
                    );
                }
                true
            }
            Err(TryRecvError::Disconnected) => {
                self.ai_dock.worker_rx = None;
                self.ai_dock.started_at = None;
                self.ai_dock.is_error = true;
                self.ai_dock.status_line = "AI worker thread disconnected unexpectedly.".into();
                true
            }
        }
    }

    pub fn start_ai_task(&mut self, task: AiTask) {
        self.ai_dock.open = true;
        self.ai_dock.selected_task = task;
        self.ai_dock.focused_field = None;
        self.ai_dock.config.save_non_secret();

        let context = self.extract_active_ai_context(task);
        if context.trim().is_empty() && self.ai_dock.custom_prompt.trim().is_empty() {
            self.ai_dock.is_error = true;
            self.ai_dock.status_line =
                "Open a Word/PDF document or type an instruction first.".into();
            return;
        }

        let cfg = self.ai_dock.config.clone();
        if let Err(dlp_err) = self.policy.validate_ai_endpoint(&cfg.endpoint_url) {
            if self.policy.audit_log_enabled {
                self.audit_trail
                    .record("AI_DLP_BLOCKED", &cfg.endpoint_url, "PolicyEngine");
            }
            self.ai_dock.is_error = true;
            self.ai_dock.status_line = dlp_err.clone();
            self.status_toast = dlp_err;
            return;
        }
        if self.policy.audit_log_enabled {
            self.audit_trail
                .record("AI_TASK_DISPATCHED", cfg.preset.short_badge(), "Nour Yahyaoui");
        }
        let policy_snapshot = self.policy.clone();
        let prompt = self.ai_dock.custom_prompt.clone();
        let (tx, rx) = mpsc::channel();
        self.ai_dock.worker_rx = Some(rx);
        self.ai_dock.started_at = Some(Instant::now());
        self.ai_dock.is_error = false;
        self.ai_dock.status_line = format!(
            "Connecting to {} ({})…",
            cfg.preset.short_badge(),
            cfg.model
        );
        self.status_toast = format!(
            "Running '{}' via {}…",
            task.label(),
            cfg.preset.short_badge()
        );

        std::thread::spawn(move || {
            let res = run_ai_task_with_policy(&cfg, &policy_snapshot, task, &context, &prompt);
            let _ = tx.send(res);
        });
    }

    fn extract_active_ai_context(&self, task: AiTask) -> String {
        match self.tabs.get(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                if let Some(sel) = w.selected_text() {
                    if !sel.trim().is_empty() {
                        return sel;
                    }
                }
                if task == AiTask::FixGrammar {
                    if let Some((tb_idx, r, c)) = w.active_table_cell {
                        if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
                            if let Some(cell) = t.rows.get(r).and_then(|row| row.get(c)) {
                                if !cell.text.trim().is_empty() {
                                    return cell.text.clone();
                                }
                            }
                        }
                    }
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
                        let cur = p.plain_text();
                        if cur.split_whitespace().count() >= 4 {
                            return cur;
                        }
                    }
                }
                word_to_markdown(&w.doc)
            }
            Some(StudioTab::Pdf(p)) => p.pdf.extract_all_text(),
            _ => String::new(),
        }
    }

    pub fn apply_ai_output_to_document(&mut self, replace_current: bool) {
        let out = self.ai_dock.output_text.trim().to_string();
        if out.is_empty() {
            self.status_toast = "Run an AI task first to generate content.".into();
            return;
        }

        enum ActiveKind {
            Word,
            Pdf,
            HomeOrNone,
        }
        let kind = match self.tabs.get(self.active_tab) {
            Some(StudioTab::Word(_)) => ActiveKind::Word,
            Some(StudioTab::Pdf(_)) => ActiveKind::Pdf,
            Some(StudioTab::HomeHub) | None => ActiveKind::HomeOrNone,
        };

        match kind {
            ActiveKind::Word => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.history.record(&w.doc);
                    w.snapshots.capture(&w.doc, "Before AI Agent apply");
                    w.dirty = true;

                    if replace_current && w.has_selection() {
                        w.delete_selection();
                        let lines: Vec<&str> = out.split('\n').collect();
                        for (li, line) in lines.iter().enumerate() {
                            if li > 0 {
                                let _ = word_enter(w);
                            }
                            for ch in line.chars() {
                                word_insert_char_no_record(w, ch);
                            }
                        }
                        w.scroll_to_caret = true;
                        self.status_toast =
                            "Replaced selected text with AI output (Ctrl+Z to undo)".into();
                        return;
                    }

                    if replace_current {
                        if let Some((tb_idx, r, c)) = w.active_table_cell {
                            if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                                if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c))
                                {
                                    cell.text = out.replace('\n', " ");
                                    w.table_cursor_char = cell.text.chars().count();
                                    self.status_toast =
                                        "Replaced active table cell with AI output (Ctrl+Z to undo)"
                                            .into();
                                    return;
                                }
                            }
                        }
                        if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) {
                            let first_line = out.lines().next().unwrap_or(&out);
                            if out.lines().count() <= 1 {
                                let st = p
                                    .spans
                                    .first()
                                    .map(|s| s.style.clone())
                                    .unwrap_or_default();
                                p.spans = vec![TextSpan::styled(first_line, st)];
                                w.cursor_char = first_line.chars().count();
                                self.status_toast =
                                    "Replaced paragraph with AI revision (Ctrl+Z to undo)".into();
                                return;
                            }
                        }
                        let new_blocks = parse_ai_output_to_blocks(&out);
                        if !new_blocks.is_empty() {
                            let at = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                            w.doc.blocks.remove(at);
                            for (offset, b) in new_blocks.into_iter().enumerate() {
                                w.doc.blocks.insert(at + offset, b);
                            }
                            self.status_toast =
                                "Replaced section with AI output (Ctrl+Z to undo)".into();
                        }
                    } else {
                        let new_blocks = parse_ai_output_to_blocks(&out);
                        let count = new_blocks.len();
                        let insert_at = (w.cursor_block + 1).min(w.doc.blocks.len());
                        for (offset, b) in new_blocks.into_iter().enumerate() {
                            w.doc.blocks.insert(insert_at + offset, b);
                        }
                        w.cursor_block = (insert_at + count.saturating_sub(1))
                            .min(w.doc.blocks.len().saturating_sub(1));
                        w.cursor_char = 0;
                        w.active_table_cell = None;
                        w.scroll_to_caret = true;
                        self.status_toast =
                            format!("Inserted {count} AI-drafted block(s) (Ctrl+Z to undo)");
                    }
                }
            }
            ActiveKind::Pdf => {
                // Convert active PDF + AI blocks into a fresh editable Word Studio tab
                self.execute_command(CommandId::ConvertPdfToWord);
                self.apply_ai_output_to_document(false);
            }
            ActiveKind::HomeOrNone => {
                let mut doc = markdown_to_word(&out, "AI Drafted Document");
                doc.author = "Nour Yahyaoui".into();
                self.tabs
                    .push(StudioTab::Word(WordTabState::from_doc(doc, None)));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = "Opened AI output in a new Word (.docx) tab".into();
            }
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
                // Recover from SHA-256 verified WAL crash frame if a newer journal exists
                let wal_p = WalJournal::journal_path_for(Some(&p), 0);
                let (doc, recovered_from_wal) = if wal_p.exists() {
                    match WalJournal::recover_frame(&wal_p) {
                        Ok(wal_doc) => (wal_doc, true),
                        Err(_) => (load_docx_file(&p).map_err(|e| e.0)?, false),
                    }
                } else {
                    (load_docx_file(&p).map_err(|e| e.0)?, false)
                };
                let _ = AdvisoryFileLock::acquire(&p, "Nour Yahyaoui");
                if self.policy.audit_log_enabled {
                    self.audit_trail
                        .record("DOC_OPENED", &p.display().to_string(), "Nour Yahyaoui");
                }
                self.tabs
                    .push(StudioTab::Word(WordTabState::from_doc(doc, Some(p.clone()))));
                self.active_tab = self.tabs.len() - 1;
                self.push_recent(p.clone());
                self.status_toast = if recovered_from_wal {
                    format!("Recovered WAL crash journal for '{}'", p.display())
                } else {
                    format!("Opened Word document '{}'", p.display())
                };
                Ok(())
            }
            "pdf" => {
                let pdf = load_pdf_file(&p)?;
                if self.policy.audit_log_enabled {
                    self.audit_trail
                        .record("PDF_OPENED", &p.display().to_string(), "Nour Yahyaoui");
                }
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
            "png" | "jpg" | "jpeg" | "bmp" => {
                self.ensure_active_word_tab();
                self.insert_image_from_path(&p, None)
            }
            other => Err(format!("Unsupported file format '.{other}'")),
        }
    }

    pub fn ensure_active_word_tab(&mut self) {
        if matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_))) {
            return;
        }
        if let Some(existing_word_idx) = self
            .tabs
            .iter()
            .position(|t| matches!(t, StudioTab::Word(_)))
        {
            self.active_tab = existing_word_idx;
        } else {
            self.tabs
                .push(StudioTab::Word(WordTabState::from_doc(Document::blank(), None)));
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn insert_image_from_bytes(
        &mut self,
        name: &str,
        raw_bytes: Vec<u8>,
        replace_block: Option<usize>,
    ) -> Result<(), String> {
        let (iw, ih, rgba) = decode_any_image_rgba(&raw_bytes)?;
        // If clipboard provided a headerless Win32 DIB, encode lossless PNG so `.docx` saves valid media
        let is_file_encoded = (raw_bytes.len() >= 8
            && raw_bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10])
            || (raw_bytes.len() >= 3
                && raw_bytes[0] == 0xFF
                && raw_bytes[1] == 0xD8
                && raw_bytes[2] == 0xFF)
            || (raw_bytes.len() >= 2 && &raw_bytes[0..2] == b"BM");
        let persisted_bytes = if is_file_encoded {
            raw_bytes
        } else {
            encode_rgba_png(iw, ih, &rgba)
        };

        self.ensure_active_word_tab();
        let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) else {
            return Err("Open a Word (.docx) tab to insert an image".into());
        };

        w.history.record(&w.doc);
        let content_w = w.content_width_pt();
        let mut img = ImageBlock::from_decoded(name, iw, ih, rgba, persisted_bytes, content_w);

        if let Some(rep_idx) =
            replace_block.filter(|&b| matches!(w.doc.blocks.get(b), Some(Block::Image(_))))
        {
            if let Some(Block::Image(prev)) = w.doc.blocks.get(rep_idx) {
                img.caption = prev.caption.clone();
                img.alignment = prev.alignment;
                img.resize_preserving_aspect(prev.width_pt, content_w);
            }
            w.doc.blocks[rep_idx] = Block::Image(img);
            w.selected_image_block = Some(rep_idx);
            w.cursor_block = rep_idx;
            w.dirty = true;
            self.status_toast = format!(
                "Replaced image with '{name}' ({iw}×{ih} px) — Ctrl+Z to undo"
            );
        } else {
            w.insert_image_smart(img, true);
            self.status_toast = format!(
                "Inserted '{name}' ({iw}×{ih} px) — drag corner handle or use Image bar to resize"
            );
        }
        Ok(())
    }

    pub fn insert_image_from_path(
        &mut self,
        path: &Path,
        replace_block: Option<usize>,
    ) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot read '{}': {e}", path.display()))?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Figure");
        self.insert_image_from_bytes(name, bytes, replace_block)
    }

    pub fn handle_dropped_file(&mut self, path: PathBuf) {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp") {
            if let Err(e) = self.insert_image_from_path(&path, None) {
                self.status_toast = format!("Image drop error: {e}");
            }
        } else if let Err(e) = self.open_path(&path) {
            self.status_toast = format!("Drop open error: {e}");
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
                    self.status_toast = format!("Exported vector PDF '{}'", target.display());
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
                    WalJournal::clear_frame(&WalJournal::journal_path_for(Some(&final_path), self.active_tab));
                    let _ = AdvisoryFileLock::acquire(&final_path, &w.doc.author);
                    w.path = Some(final_path.clone());
                    w.dirty = false;
                    w.snapshots.capture(&w.doc, "Saved to disk");
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "DOC_SAVED",
                            &final_path.display().to_string(),
                            &w.doc.author,
                        );
                    }
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
            FileModalKind::InsertImage | FileModalKind::ReplaceImage => {
                let title = if kind == FileModalKind::ReplaceImage {
                    "Replace Image in RustPad"
                } else {
                    "Insert Image into RustPad"
                };
                if let Some(picked) = try_native_open_image_dialog(title) {
                    self.apply_file_modal_choice(kind, picked);
                    return;
                }
                let candidates = discover_workspace_images();
                self.modal = Some(ActiveModal::FilePrompt {
                    kind,
                    input_path: default_name.to_string(),
                    recent_candidates: candidates,
                });
                return;
            }
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
            FileModalKind::InsertImage => {
                if let Err(e) = self.insert_image_from_path(&path, None) {
                    self.status_toast = format!("Insert image error: {e}");
                }
            }
            FileModalKind::ReplaceImage => {
                let rep = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => w.selected_image_block.or(Some(w.cursor_block)),
                    _ => None,
                };
                if let Err(e) = self.insert_image_from_path(&path, rep) {
                    self.status_toast = format!("Replace image error: {e}");
                }
            }
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
                let export_res = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => Some(export_document_to_pdf_file(
                        &pdf_path,
                        &w.doc,
                        &PdfCreateOptions::default(),
                    )),
                    _ => None,
                };
                match export_res {
                    Some(Ok(())) => {
                        let _ = self.open_path(&pdf_path);
                        self.status_toast =
                            format!("Exported & opened vector PDF '{}'", pdf_path.display());
                    }
                    Some(Err(e)) => self.status_toast = format!("PDF export error: {e}"),
                    None => {}
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
        self.open_dropdown = None;
        match cmd {
            CommandId::NewWordBlank => {
                let mut doc = Document::blank();
                doc.classification = self.policy.default_classification;
                self.tabs.push(StudioTab::Word(WordTabState::from_doc(
                    doc,
                    None,
                )));
                self.active_tab = self.tabs.len() - 1;
                self.status_toast = "Created blank A4 Word document".into();
            }
            CommandId::NewWordEnterpriseMsa => {
                self.tabs.push(StudioTab::Word(WordTabState::from_doc(
                    TemplateKind::EnterpriseContract.build(),
                    None,
                )));
                self.active_tab = self.tabs.len() - 1;
                if self.policy.audit_log_enabled {
                    self.audit_trail.record(
                        "TEMPLATE_CREATED",
                        "Enterprise MSA Redline",
                        "Nour Yahyaoui",
                    );
                }
                self.status_toast =
                    "Opened Enterprise MSA Contract with ECMA-376 Track Changes & Review Comments"
                        .into();
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
                let def_opt = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => {
                        Some(format!("{}.pdf", sanitize_filename(&w.doc.title)))
                    }
                    _ => None,
                };
                if let Some(def) = def_opt {
                    self.prompt_or_open_modal(FileModalKind::ExportPdf, &def);
                } else {
                    self.status_toast = "Switch to a Word (.docx) tab to export PDF".into();
                }
            }
            CommandId::ConvertPdfToWord => {
                let converted_doc = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Pdf(p)) => Some(pdf_to_word(&p.pdf)),
                    _ => None,
                };
                let converted_pdf = if converted_doc.is_none() {
                    match self.tabs.get(self.active_tab) {
                        Some(StudioTab::Word(w)) => word_to_pdf_document(&w.doc).ok(),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(mut doc) = converted_doc {
                    doc.title = format!("{}_Editable", doc.title);
                    self.tabs
                        .push(StudioTab::Word(WordTabState::from_doc(doc, None)));
                    self.active_tab = self.tabs.len() - 1;
                    self.status_toast =
                        "Converted PDF into an editable Word (.docx) tab".into();
                } else if let Some(pdf) = converted_pdf {
                    self.tabs
                        .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, None)));
                    self.active_tab = self.tabs.len() - 1;
                    self.status_toast =
                        "Opened Word document directly inside PDF Studio".into();
                }
            }
            CommandId::Undo => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    if w.history.undo(&mut w.doc) {
                        w.active_table_cell = None;
                        w.selected_image_block = None;
                        w.selection_anchor = None;
                        w.image_rgba_cache.clear();
                        w.cursor_block = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                        w.cursor_char = 0;
                        self.status_toast = "Undid last edit".into();
                    } else {
                        self.status_toast = "Nothing to undo".into();
                    }
                }
            }
            CommandId::Redo => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    if w.history.redo(&mut w.doc) {
                        w.active_table_cell = None;
                        w.selected_image_block = None;
                        w.selection_anchor = None;
                        w.image_rgba_cache.clear();
                        w.cursor_block = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                        w.cursor_char = 0;
                        self.status_toast = "Redid edit".into();
                    } else {
                        self.status_toast = "Nothing to redo".into();
                    }
                }
            }
            CommandId::SelectAll => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.select_all();
                    let words = w
                        .selected_text()
                        .map(|t| t.split_whitespace().count())
                        .unwrap_or(0);
                    self.status_toast =
                        format!("Selected entire document ({words} words) — format, copy, or AI polish");
                }
            }
            CommandId::ToggleFocusMode => {
                self.focus_mode = !self.focus_mode;
                self.open_dropdown = None;
                self.status_toast = if self.focus_mode {
                    "Entered Distraction-Free Writer Focus Mode (Ctrl+Shift+F to restore chrome)"
                        .into()
                } else {
                    "Exited Focus Mode — restored Studio ribbon & sidebars".into()
                };
            }
            CommandId::ToggleLeftSidebar => {
                match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Pdf(_)) => {
                        self.pdf_left_open = !self.pdf_left_open;
                        self.status_toast = if self.pdf_left_open {
                            "Showed PDF Pages sidebar".into()
                        } else {
                            "Collapsed PDF Pages sidebar".into()
                        };
                    }
                    _ => {
                        self.word_sidebar_open = !self.word_sidebar_open;
                        self.status_toast = if self.word_sidebar_open {
                            "Showed Document Outline sidebar".into()
                        } else {
                            "Collapsed Outline sidebar (Focus mode)".into()
                        };
                    }
                }
            }
            CommandId::ToggleAiDock => {
                self.ai_dock.open = !self.ai_dock.open;
                if self.ai_dock.open && self.ai_dock.discovered_models.is_empty() {
                    if self.ai_dock.config.preset == AiProviderPreset::OllamaLocal {
                        self.ai_dock.discovered_models =
                            discover_local_ollama_models(&self.ai_dock.config.endpoint_url);
                    }
                }
                self.status_toast = if self.ai_dock.open {
                    "Opened Pluggable AI Agent Studio Dock (drag edge to resize)".into()
                } else {
                    "Closed AI Agent Dock".into()
                };
            }
            CommandId::AiRunFixGrammar => self.start_ai_task(AiTask::FixGrammar),
            CommandId::AiRunSummarize => self.start_ai_task(AiTask::SummarizeDocument),
            CommandId::AiRunDescribe => self.start_ai_task(AiTask::WriteDescription),
            CommandId::AiRunAddSections => self.start_ai_task(AiTask::AddSections),
            CommandId::OpenTableCreator => {
                if !matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_))) {
                    self.execute_command(CommandId::NewWordBlank);
                }
                let ax = if self.mouse_x > 40.0 {
                    (self.mouse_x as i32 - 90).max(24)
                } else {
                    420
                };
                let ay = if self.focus_mode { 28 } else { 88 };
                self.open_dropdown = Some(ToolbarDropdown::TableCreator {
                    anchor_x: ax,
                    anchor_y: ay,
                    rows: 3,
                    cols: 3,
                });
                self.status_toast =
                    "Choose grid size (Arrow keys + Enter or hover/click) or pick a styled preset"
                        .into();
            }
            CommandId::InsertImage => {
                self.ensure_active_word_tab();
                self.prompt_or_open_modal(FileModalKind::InsertImage, "");
            }
            CommandId::ImageReplace => {
                self.ensure_active_word_tab();
                self.prompt_or_open_modal(FileModalKind::ReplaceImage, "");
            }
            CommandId::ImageEditCaption => {
                if let Some(StudioTab::Word(w)) = self.tabs.get(self.active_tab) {
                    let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                    let target_idx = w.selected_image_block.unwrap_or(idx);
                    if let Some(Block::Image(img)) = w.doc.blocks.get(target_idx) {
                        self.modal = Some(ActiveModal::EditImageCaption {
                            block_idx: target_idx,
                            text: img.caption.clone(),
                        });
                    } else {
                        self.status_toast =
                            "Click an image figure first to add or edit its caption".into();
                    }
                } else {
                    self.status_toast = "Open a Word (.docx) tab to edit figure captions".into();
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
            | CommandId::TableInsertRowAbove
            | CommandId::TableDeleteRow
            | CommandId::TableAddCol
            | CommandId::TableInsertColLeft
            | CommandId::TableDeleteCol
            | CommandId::TableToggleCellBold
            | CommandId::TableCycleCellFill
            | CommandId::TableToggleZebra
            | CommandId::TableCycleTheme
            | CommandId::TableEqualizeCols
            | CommandId::TableDelete
            | CommandId::ImageScale25
            | CommandId::ImageScale50
            | CommandId::ImageScale75
            | CommandId::ImageScale100
            | CommandId::ImageDelete
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
                            "Compressed PDF streams: {:.1} KB -> {:.1} KB ({}% smaller)",
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
                            word_to_pdf_document(&w.doc).unwrap_or_else(|_| {
                                PdfDocument::new_blank("Preview", 1, PaperSize::A4)
                            })
                        } else {
                            PdfDocument::new_blank("Reference PDF", 1, PaperSize::A4)
                        };
                        self.tabs
                            .push(StudioTab::Pdf(PdfTabState::from_pdf(pdf, None)));
                        self.split_secondary_tab = Some(self.tabs.len() - 1);
                    }
                    self.status_toast =
                        "Enabled Side-by-Side Split Workspace (drag center bar to resize)".into();
                }
            }
            CommandId::OpenSnapshotsDrawer => {
                self.modal = Some(ActiveModal::SnapshotsDrawer);
            }
            CommandId::PrintDocument => {
                let spool_path = std::env::temp_dir().join("rustpad_print_spool.pdf");
                let prep = match self.tabs.get(self.active_tab) {
                    Some(StudioTab::Word(w)) => {
                        export_document_to_pdf_file(&spool_path, &w.doc, &PdfCreateOptions::default())
                    }
                    Some(StudioTab::Pdf(p)) => save_pdf_file(&spool_path, &p.pdf, true),
                    _ => Err("Open a Word or PDF document to print".into()),
                };
                match prep.and_then(|()| spool_pdf_to_os_printer(&spool_path)) {
                    Ok(msg) => {
                        if self.policy.audit_log_enabled {
                            self.audit_trail
                                .record("OS_PRINT_SPOOL", &spool_path.display().to_string(), "Nour Yahyaoui");
                        }
                        self.status_toast = msg;
                    }
                    Err(e) => self.status_toast = format!("Print error: {e}"),
                }
            }
            CommandId::ToggleTrackChanges => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.doc.track_changes = !w.doc.track_changes;
                    w.dirty = true;
                    let state_lbl = if w.doc.track_changes { "ON" } else { "OFF" };
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "TRACK_CHANGES_TOGGLED",
                            &format!("{}: {state_lbl}", w.doc.title),
                            &w.doc.author,
                        );
                    }
                    self.status_toast = format!(
                        "Legal Track Changes {state_lbl} — edits record ECMA-376 <w:ins> / <w:del>"
                    );
                }
            }
            CommandId::AcceptAllRevisions => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.history.record(&w.doc);
                    let n = w.doc.accept_all_revisions();
                    w.dirty = true;
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "REVISIONS_ACCEPTED_ALL",
                            &format!("{} ({n} changes)", w.doc.title),
                            &w.doc.author,
                        );
                    }
                    self.status_toast = format!("Accepted {n} tracked revision(s)");
                }
            }
            CommandId::RejectAllRevisions => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.history.record(&w.doc);
                    let n = w.doc.reject_all_revisions();
                    w.dirty = true;
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "REVISIONS_REJECTED_ALL",
                            &format!("{} ({n} changes)", w.doc.title),
                            &w.doc.author,
                        );
                    }
                    self.status_toast = format!("Rejected {n} tracked revision(s)");
                }
            }
            CommandId::AddReviewComment => {
                if matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_))) {
                    self.modal = Some(ActiveModal::AddComment {
                        text: String::new(),
                    });
                } else {
                    self.status_toast = "Open a Word (.docx) tab to add threaded review comments".into();
                }
            }
            CommandId::CycleClassification => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.doc.classification = w.doc.classification.cycle_next();
                    w.dirty = true;
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "CLASSIFICATION_CHANGED",
                            &format!("{} -> {}", w.doc.title, w.doc.classification.label()),
                            &w.doc.author,
                        );
                    }
                    self.status_toast =
                        format!("Security Classification: {}", w.doc.classification.label());
                } else {
                    self.policy.default_classification =
                        self.policy.default_classification.cycle_next();
                    self.status_toast = format!(
                        "Default Policy Classification: {}",
                        self.policy.default_classification.label()
                    );
                }
            }
            CommandId::PdfSanitizeRedactions => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let report = p.pdf.sanitize_redactions(true);
                    p.dirty = true;
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "PDF_FORENSIC_SANITIZE",
                            &format!(
                                "{} (purged {} text runs across {} box(es))",
                                p.pdf.title, report.purged_text_runs, report.redaction_boxes
                            ),
                            "Legal Forensics",
                        );
                    }
                    self.status_toast = format!(
                        "Forensic Sanitize complete: purged {} underlying text run(s) & scrubbed metadata",
                        report.purged_text_runs
                    );
                } else {
                    self.status_toast = "Open a PDF tab to sanitize redactions".into();
                }
            }
            CommandId::PdfSealDocument => {
                if let Some(StudioTab::Pdf(p)) = self.tabs.get_mut(self.active_tab) {
                    let (_sealed_bytes, manifest) =
                        seal_pdf_bytes(&p.pdf, "Nour Yahyaoui", "2026-10-27T12:00:00Z", true);
                    if let Some(target) = &p.path {
                        let _ = std::fs::write(target, &_sealed_bytes);
                        p.dirty = false;
                    }
                    if self.policy.audit_log_enabled {
                        self.audit_trail.record(
                            "PDF_SHA256_SEALED",
                            &format!("{} ({})", p.pdf.title, &manifest.content_sha256[..16]),
                            &manifest.signer,
                        );
                    }
                    self.status_toast = format!(
                        "Embedded PDF/A-2b + SHA-256 Tamper Seal [{}…]",
                        &manifest.content_sha256[..16]
                    );
                } else {
                    self.status_toast = "Open a PDF tab to embed SHA-256 seal".into();
                }
            }
            CommandId::OpenGovernanceModal => {
                self.modal = Some(ActiveModal::Governance);
            }
            CommandId::ToggleAirGappedDlp => {
                self.policy.air_gapped_mode = !self.policy.air_gapped_mode;
                self.policy.allow_cloud_ai = !self.policy.air_gapped_mode;
                let mode_str = if self.policy.air_gapped_mode {
                    "STRICT AIR-GAPPED (Cloud AI Locked)"
                } else {
                    "STANDARD HYBRID (Local + Allowlisted Cloud)"
                };
                self.audit_trail
                    .record("DLP_MODE_SWITCHED", mode_str, "EnterpriseAdmin");
                self.status_toast = format!("DLP Policy Mode: {mode_str}");
            }
            CommandId::CaptureSnapshot => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    let id = w.snapshots.capture(&w.doc, "Manual checkpoint");
                    let wal_p = WalJournal::journal_path_for(w.path.as_deref(), self.active_tab);
                    let _ = WalJournal::write_frame(&wal_p, &w.doc);
                    self.status_toast =
                        format!("Saved Time-Machine Snapshot #{id} + SHA-256 WAL frame");
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
            CommandId::ZoomIn => self.adjust_zoom(0.10),
            CommandId::ZoomOut => self.adjust_zoom(-0.10),
            CommandId::ZoomReset => self.set_zoom(1.12),
            CommandId::UiScaleUp => {
                self.ui_scale = (self.ui_scale + 0.10).clamp(0.85, 1.55);
                self.status_toast =
                    format!("Interface Scale: {}%", (self.ui_scale * 100.0).round() as i32);
            }
            CommandId::UiScaleDown => {
                self.ui_scale = (self.ui_scale - 0.10).clamp(0.85, 1.55);
                self.status_toast =
                    format!("Interface Scale: {}%", (self.ui_scale * 100.0).round() as i32);
            }
            CommandId::UiScaleReset => {
                self.ui_scale = 1.0;
                self.status_toast = "Interface Scale reset to 100%".into();
            }
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
                w.zoom = (w.zoom + delta).clamp(0.65, 2.10);
                self.status_toast = format!("Paper Zoom: {}%", (w.zoom * 100.0).round() as i32);
            }
            Some(StudioTab::Pdf(p)) => {
                p.zoom = (p.zoom + delta).clamp(0.65, 2.10);
                self.status_toast = format!("PDF Zoom: {}%", (p.zoom * 100.0).round() as i32);
            }
            _ => {}
        }
    }

    fn set_zoom(&mut self, z: f32) {
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => w.zoom = z,
            Some(StudioTab::Pdf(p)) => p.zoom = 1.08,
            _ => {}
        }
        self.status_toast = format!("Zoom reset to {}%", (z * 100.0).round() as i32);
    }

    pub fn apply_paragraph_style(&mut self, style: ParagraphStyle) {
        if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
            let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
            if matches!(w.doc.blocks.get(idx), Some(Block::Paragraph(_))) {
                w.history.record(&w.doc);
            }
            if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
                w.dirty = true;
                p.style = style;
                let sz = style.default_font_size();
                let b = style.is_bold_by_default();
                let color = if matches!(style, ParagraphStyle::Subtitle | ParagraphStyle::Quote) {
                    RgbColor::MUTED_GRAY
                } else {
                    RgbColor::INK_SLATE
                };
                if p.spans.is_empty() {
                    p.spans.push(TextSpan::styled(
                        "",
                        SpanStyle {
                            font_size_pt: sz,
                            bold: b,
                            italic: style == ParagraphStyle::Quote,
                            color,
                            ..SpanStyle::default()
                        },
                    ));
                } else {
                    for s in &mut p.spans {
                        s.style.font_size_pt = sz;
                        s.style.bold = b;
                        s.style.italic = style == ParagraphStyle::Quote;
                        s.style.color = color;
                    }
                }
                self.status_toast = format!("Paragraph Style: {}", style.label());
            }
        }
    }

    pub fn apply_font_family(&mut self, fam: FontFamily) {
        if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
            if let Some(((b0, c0), (b1, c1))) = w.ordered_selection() {
                w.history.record(&w.doc);
                w.dirty = true;
                for bi in b0..=b1.min(w.doc.blocks.len().saturating_sub(1)) {
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(bi) {
                        let len = p.plain_text().chars().count();
                        let s = if bi == b0 { c0.min(len) } else { 0 };
                        let e = if bi == b1 { c1.min(len) } else { len };
                        p.transform_range(s, e, |st| st.font_family = fam);
                    }
                }
                self.status_toast = format!("Applied '{}' to selection", fam.label());
                return;
            }
            let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
            if matches!(w.doc.blocks.get(idx), Some(Block::Paragraph(_))) {
                w.history.record(&w.doc);
            }
            if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
                w.dirty = true;
                for s in &mut p.spans {
                    s.style.font_family = fam;
                }
                self.status_toast = format!("Font Family: {}", fam.label());
            }
        }
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
                w.table_cursor_char = 0;
                w.cursor_block = insert_at;
                w.scroll_to_caret = true;
                self.status_toast =
                    "Inserted 3×3 table — type in cells, Tab to advance, drag borders to resize"
                        .into();
                return;
            }
            CommandId::TableAddRow => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        let at = (r + 1).min(t.rows.len());
                        t.insert_row_at(at);
                        w.active_table_cell = Some((tb_idx, at, c.min(t.cols.saturating_sub(1))));
                        w.table_cursor_char = 0;
                        self.status_toast = format!("Inserted row {} below", at + 1);
                        return;
                    }
                }
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(idx) {
                    t.add_row();
                    self.status_toast = "Added row to table".into();
                }
                return;
            }
            CommandId::TableInsertRowAbove => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.insert_row_at(r);
                        w.active_table_cell = Some((tb_idx, r, c));
                        w.table_cursor_char = 0;
                        self.status_toast = format!("Inserted row above row {}", r + 1);
                    }
                }
                return;
            }
            CommandId::TableDeleteRow => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if t.delete_row(r) {
                            let nr = r.min(t.rows.len().saturating_sub(1));
                            let nc = c.min(t.cols.saturating_sub(1));
                            w.active_table_cell = Some((tb_idx, nr, nc));
                            w.table_cursor_char = 0;
                            self.status_toast = "Deleted table row".into();
                        } else {
                            self.status_toast = "Table must keep at least 1 row".into();
                        }
                    }
                }
                return;
            }
            CommandId::TableAddCol => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        let at = (c + 1).min(t.cols);
                        t.insert_column_at(at);
                        w.table_col_weights.retain(|(b, _)| *b != tb_idx);
                        w.active_table_cell = Some((tb_idx, r, at));
                        w.table_cursor_char = 0;
                        self.status_toast = format!("Inserted column {}", at + 1);
                        return;
                    }
                }
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(idx) {
                    t.add_column();
                    w.table_col_weights.retain(|(b, _)| *b != idx);
                    self.status_toast = "Added column to table".into();
                }
                return;
            }
            CommandId::TableInsertColLeft => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.insert_column_at(c);
                        w.table_col_weights.retain(|(b, _)| *b != tb_idx);
                        w.active_table_cell = Some((tb_idx, r, c));
                        w.table_cursor_char = 0;
                        self.status_toast = format!("Inserted column left of column {}", c + 1);
                    }
                }
                return;
            }
            CommandId::TableDeleteCol => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if t.delete_column(c) {
                            w.table_col_weights.retain(|(b, _)| *b != tb_idx);
                            let nc = c.min(t.cols.saturating_sub(1));
                            w.active_table_cell = Some((tb_idx, r, nc));
                            w.table_cursor_char = 0;
                            self.status_toast = "Deleted table column".into();
                        } else {
                            self.status_toast = "Table must keep at least 1 column".into();
                        }
                    }
                }
                return;
            }
            CommandId::TableToggleCellBold | CommandId::ToggleBold
                if w.active_table_cell.is_some() =>
            {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.toggle_cell_bold(r, c);
                        self.status_toast = "Toggled table cell bold".into();
                    }
                }
                return;
            }
            CommandId::TableCycleCellFill => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        t.cycle_cell_fill(r, c);
                        self.status_toast = "Cycled table cell shading".into();
                    }
                }
                return;
            }
            CommandId::TableToggleZebra => {
                let target_idx = w.active_table_cell.map(|(b, _, _)| b).unwrap_or(idx);
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(target_idx) {
                    let on = t.toggle_zebra_striping();
                    self.status_toast = if on {
                        "Enabled alternating zebra row striping".into()
                    } else {
                        "Cleared alternating zebra row striping".into()
                    };
                }
                return;
            }
            CommandId::TableCycleTheme => {
                let target_idx = w.active_table_cell.map(|(b, _, _)| b).unwrap_or(idx);
                if let Some(Block::Table(t)) = w.doc.blocks.get_mut(target_idx) {
                    let next_theme: TableTheme = t.cycle_theme();
                    self.status_toast = format!("Applied '{}' table theme", next_theme.label());
                }
                return;
            }
            CommandId::TableEqualizeCols => {
                let target_idx = w.active_table_cell.map(|(b, _, _)| b).unwrap_or(idx);
                if matches!(w.doc.blocks.get(target_idx), Some(Block::Table(_))) {
                    w.table_col_weights.retain(|(b, _)| *b != target_idx);
                    self.status_toast = "Equalized all column widths".into();
                }
                return;
            }
            CommandId::TableDelete => {
                let target_idx = w.active_table_cell.map(|(b, _, _)| b).unwrap_or(idx);
                if matches!(w.doc.blocks.get(target_idx), Some(Block::Table(_))) {
                    w.doc.blocks.remove(target_idx);
                    w.table_col_weights.retain(|(b, _)| *b != target_idx);
                    for (b_idx, _) in &mut w.table_col_weights {
                        if *b_idx > target_idx {
                            *b_idx -= 1;
                        }
                    }
                    w.doc.ensure_non_empty();
                    w.active_table_cell = None;
                    w.selected_image_block = None;
                    w.cursor_block = target_idx.min(w.doc.blocks.len().saturating_sub(1));
                    w.cursor_char = 0;
                    self.status_toast = "Removed table (Ctrl+Z to undo)".into();
                }
                return;
            }
            CommandId::ImageScale25
            | CommandId::ImageScale50
            | CommandId::ImageScale75
            | CommandId::ImageScale100 => {
                let target_idx = w.selected_image_block.unwrap_or(idx);
                let cw = w.content_width_pt();
                let frac = match cmd {
                    CommandId::ImageScale25 => 0.25,
                    CommandId::ImageScale50 => 0.50,
                    CommandId::ImageScale75 => 0.75,
                    _ => 1.0,
                };
                if let Some(Block::Image(img)) = w.doc.blocks.get_mut(target_idx) {
                    img.scale_to_fraction(frac, cw);
                    w.selected_image_block = Some(target_idx);
                    self.status_toast =
                        format!("Resized figure to {:.0}% of sheet width", frac * 100.0);
                }
                return;
            }
            CommandId::ImageDelete => {
                let target_idx = w.selected_image_block.unwrap_or(idx);
                if w.delete_image_at(target_idx) {
                    self.status_toast = "Removed image figure (Ctrl+Z to undo)".into();
                }
                return;
            }
            CommandId::AlignLeft | CommandId::AlignCenter | CommandId::AlignRight
                if w.selected_image_block.is_some()
                    || matches!(w.doc.blocks.get(idx), Some(Block::Image(_))) =>
            {
                let target_idx = w.selected_image_block.unwrap_or(idx);
                if let Some(Block::Image(img)) = w.doc.blocks.get_mut(target_idx) {
                    img.alignment = match cmd {
                        CommandId::AlignCenter => Alignment::Center,
                        CommandId::AlignRight => Alignment::Right,
                        _ => Alignment::Left,
                    };
                    self.status_toast = match img.alignment {
                        Alignment::Center => "Centered figure on sheet".into(),
                        Alignment::Right => "Right-aligned figure on sheet".into(),
                        _ => "Left-aligned figure on sheet".into(),
                    };
                }
                return;
            }
            CommandId::AlignLeft | CommandId::AlignCenter | CommandId::AlignRight
                if w.active_table_cell.is_some() =>
            {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            cell.alignment = match cmd {
                                CommandId::AlignCenter => Alignment::Center,
                                CommandId::AlignRight => Alignment::Right,
                                _ => Alignment::Left,
                            };
                        }
                    }
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
                w.active_table_cell = None;
                w.scroll_to_caret = true;
                self.status_toast = "Inserted physical Page Break".into();
                return;
            }
            _ => {}
        }

        // Substring inline formatting when a range selection is active!
        if let Some(((b0, c0), (b1, c1))) = w.ordered_selection() {
            if matches!(
                cmd,
                CommandId::ToggleBold
                    | CommandId::ToggleItalic
                    | CommandId::ToggleUnderline
                    | CommandId::ToggleStrike
                    | CommandId::CycleFontFamily
                    | CommandId::FontSizeUp
                    | CommandId::FontSizeDown
                    | CommandId::CycleTextColor
                    | CommandId::ToggleHighlight
            ) {
                let probe_style = match w.doc.blocks.get(b0) {
                    Some(Block::Paragraph(p)) => p.style_at_char(c0 + 1),
                    _ => SpanStyle::default(),
                };
                let target_bold = !probe_style.bold;
                let target_italic = !probe_style.italic;
                let target_under = !probe_style.underline;
                let target_strike = !probe_style.strikethrough;
                let next_font = match probe_style.font_family {
                    FontFamily::Sans => FontFamily::Serif,
                    FontFamily::Serif => FontFamily::Mono,
                    FontFamily::Mono => FontFamily::Sans,
                };
                let next_color = if probe_style.color == RgbColor::INK_SLATE {
                    RgbColor::COBALT
                } else if probe_style.color == RgbColor::COBALT {
                    RgbColor::CORAL
                } else if probe_style.color == RgbColor::CORAL {
                    RgbColor::EMERALD
                } else {
                    RgbColor::INK_SLATE
                };
                let next_hl = match probe_style.highlight {
                    None => Some(RgbColor::AMBER_HIGHLIGHT),
                    Some(c) if c == RgbColor::AMBER_HIGHLIGHT => Some(RgbColor::MINT_HIGHLIGHT),
                    Some(c) if c == RgbColor::MINT_HIGHLIGHT => Some(RgbColor::SKY_HIGHLIGHT),
                    _ => None,
                };

                for bi in b0..=b1.min(w.doc.blocks.len().saturating_sub(1)) {
                    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(bi) {
                        let len = p.plain_text().chars().count();
                        let s = if bi == b0 { c0.min(len) } else { 0 };
                        let e = if bi == b1 { c1.min(len) } else { len };
                        p.transform_range(s, e, |st| match cmd {
                            CommandId::ToggleBold => st.bold = target_bold,
                            CommandId::ToggleItalic => st.italic = target_italic,
                            CommandId::ToggleUnderline => st.underline = target_under,
                            CommandId::ToggleStrike => st.strikethrough = target_strike,
                            CommandId::CycleFontFamily => st.font_family = next_font,
                            CommandId::FontSizeUp => {
                                st.font_size_pt = (st.font_size_pt + 1.0).min(56.0)
                            }
                            CommandId::FontSizeDown => {
                                st.font_size_pt = (st.font_size_pt - 1.0).max(8.5)
                            }
                            CommandId::CycleTextColor => st.color = next_color,
                            CommandId::ToggleHighlight => st.highlight = next_hl,
                            _ => {}
                        });
                    }
                }
                return;
            }
        }

        if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
            if p.spans.is_empty() {
                p.spans.push(TextSpan::plain(""));
            }
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
                    let next = match p.style {
                        ParagraphStyle::Normal => ParagraphStyle::Heading1,
                        ParagraphStyle::Heading1 => ParagraphStyle::Heading2,
                        ParagraphStyle::Heading2 => ParagraphStyle::Heading3,
                        ParagraphStyle::Heading3 => ParagraphStyle::Title,
                        ParagraphStyle::Title => ParagraphStyle::Quote,
                        ParagraphStyle::Quote | ParagraphStyle::Subtitle => ParagraphStyle::Normal,
                    };
                    p.style = next;
                    let sz = next.default_font_size();
                    let b = next.is_bold_by_default();
                    for s in &mut p.spans {
                        s.style.font_size_pt = sz;
                        s.style.bold = b;
                        s.style.italic = next == ParagraphStyle::Quote;
                    }
                    self.status_toast = format!("Style: {}", next.label());
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
                        s.style.font_size_pt = (s.style.font_size_pt + 1.0).min(56.0);
                    }
                }
                CommandId::FontSizeDown => {
                    for s in &mut p.spans {
                        s.style.font_size_pt = (s.style.font_size_pt - 1.0).max(8.5);
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
                self.open_dropdown = None;
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                }
            }
            UiAction::CloseTab(idx) => {
                self.open_dropdown = None;
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
                self.open_dropdown = None;
                self.active_tab = 0;
            }
            UiAction::OpenCommandPalette => {
                self.open_dropdown = None;
                self.modal = Some(ActiveModal::CommandPalette {
                    query: String::new(),
                    selected: 0,
                });
            }
            UiAction::ExecCommand(cmd) => {
                self.execute_command(cmd);
            }
            UiAction::ToggleDropdown(dd) => {
                if self
                    .open_dropdown
                    .map(|cur| cur.same_kind(dd))
                    .unwrap_or(false)
                {
                    self.open_dropdown = None;
                } else {
                    self.open_dropdown = Some(dd);
                }
            }
            UiAction::CloseDropdown => {
                self.open_dropdown = None;
            }
            UiAction::SelectParagraphStyle(st) => {
                self.open_dropdown = None;
                self.apply_paragraph_style(st);
            }
            UiAction::SelectFontFamily(fam) => {
                self.open_dropdown = None;
                self.apply_font_family(fam);
            }
            UiAction::InsertTableCustom { rows, cols } => {
                self.open_dropdown = None;
                let r = rows.clamp(1, 24);
                let c = cols.clamp(1, 12);
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.history.record(&w.doc);
                    // Focus header cell `(0, 0)` so typing replaces `Column 1` -> `Tab` -> `Column 2` effortlessly!
                    w.insert_table_smart(TableBlock::new(r, c), None, true, 0);
                    self.status_toast = format!(
                        "Inserted {r}×{c} table — type header & press Tab across cells (Esc to exit)"
                    );
                }
            }
            UiAction::InsertTablePreset(preset) => {
                self.open_dropdown = None;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    w.history.record(&w.doc);
                    let (tbl, weights) = preset.build();
                    let focus_r = if tbl.rows.len() > 1 { 1 } else { 0 };
                    w.insert_table_smart(tbl, weights, true, focus_r);
                    self.status_toast = format!(
                        "Inserted '{}' table — Tab across cells, drag dividers to resize",
                        preset.label()
                    );
                }
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
                self.open_dropdown = None;
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.cursor_block = block_idx;
                    w.cursor_char = 0;
                    w.selection_anchor = None;
                    w.active_table_cell = None;
                    w.selected_image_block = None;
                    w.target_scroll_y = target_scroll_y.clamp(0.0, w.max_scroll_y);
                }
            }
            UiAction::SelectWordImage { tab_idx, block_idx } => {
                self.open_dropdown = None;
                self.ai_dock.focused_field = None;
                self.active_tab = tab_idx;
                let now = Instant::now();
                let is_double_click = self
                    .last_click_at
                    .map(|t0| now.duration_since(t0).as_millis() <= 380)
                    .unwrap_or(false)
                    && (self.mouse_x - self.last_click_pos.0).abs() <= 10.0
                    && (self.mouse_y - self.last_click_pos.1).abs() <= 10.0;
                self.last_click_at = Some(now);
                self.last_click_pos = (self.mouse_x, self.mouse_y);

                let mut open_caption: Option<(usize, String)> = None;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.active_table_cell = None;
                    w.selection_anchor = None;
                    w.cursor_block = block_idx;
                    w.cursor_char = 0;
                    w.selected_image_block = Some(block_idx);
                    if is_double_click {
                        if let Some(Block::Image(img)) = w.doc.blocks.get(block_idx) {
                            open_caption = Some((block_idx, img.caption.clone()));
                        }
                    } else {
                        self.status_toast =
                            "Selected figure — drag corner handle to resize, double-click for caption, or press Delete"
                                .into();
                    }
                }
                if let Some((b_idx, cap)) = open_caption {
                    self.modal = Some(ActiveModal::EditImageCaption {
                        block_idx: b_idx,
                        text: cap,
                    });
                }
            }
            UiAction::ClickWordParagraph {
                tab_idx,
                block_idx,
                char_idx,
            } => {
                self.open_dropdown = None;
                self.ai_dock.focused_field = None;
                self.active_tab = tab_idx;
                let now = Instant::now();
                let is_double_click = self
                    .last_click_at
                    .map(|t0| now.duration_since(t0).as_millis() <= 380)
                    .unwrap_or(false)
                    && (self.mouse_x - self.last_click_pos.0).abs() <= 8.0
                    && (self.mouse_y - self.last_click_pos.1).abs() <= 8.0;
                self.last_click_at = Some(now);
                self.last_click_pos = (self.mouse_x, self.mouse_y);

                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.active_table_cell = None;
                    w.selected_image_block = None;
                    if is_double_click {
                        let (ws, we) = match w.doc.blocks.get(block_idx) {
                            Some(Block::Paragraph(p)) => {
                                word_range_at_char(&p.plain_text(), char_idx)
                            }
                            _ => (char_idx, char_idx),
                        };
                        w.selection_anchor = Some((block_idx, ws));
                        w.cursor_block = block_idx;
                        w.cursor_char = we;
                        self.active_drag = DragInteraction::None;
                    } else if self.last_click_mods.shift {
                        if w.selection_anchor.is_none() {
                            w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                        }
                        w.cursor_block = block_idx;
                        w.cursor_char = char_idx;
                        self.active_drag = DragInteraction::WordTextSelection { tab_idx };
                    } else {
                        w.selection_anchor = Some((block_idx, char_idx));
                        w.cursor_block = block_idx;
                        w.cursor_char = char_idx;
                        self.active_drag = DragInteraction::WordTextSelection { tab_idx };
                    }
                }
            }
            UiAction::AcceptRevisionById { tab_idx, rev_id } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.history.record(&w.doc);
                    let n = w.doc.accept_revision(rev_id);
                    if n > 0 {
                        w.dirty = true;
                        w.selection_anchor = None;
                        if self.policy.audit_log_enabled {
                            self.audit_trail.record(
                                "REVISION_CLAUSE_ACCEPTED",
                                &format!("rev_id={rev_id}"),
                                &w.doc.author,
                            );
                        }
                        self.status_toast =
                            format!("Accepted revision clause #{rev_id} ({n} run(s) committed)");
                    }
                }
            }
            UiAction::RejectRevisionById { tab_idx, rev_id } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.history.record(&w.doc);
                    let n = w.doc.reject_revision(rev_id);
                    if n > 0 {
                        w.dirty = true;
                        w.selection_anchor = None;
                        if self.policy.audit_log_enabled {
                            self.audit_trail.record(
                                "REVISION_CLAUSE_REJECTED",
                                &format!("rev_id={rev_id}"),
                                &w.doc.author,
                            );
                        }
                        self.status_toast =
                            format!("Rejected revision clause #{rev_id} ({n} run(s) reverted)");
                    }
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
                char_idx,
            } => {
                self.open_dropdown = None;
                self.ai_dock.focused_field = None;
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    w.cursor_block = block_idx;
                    w.selected_image_block = None;
                    w.active_table_cell = Some((block_idx, row, col));
                    w.table_cursor_char = char_idx;
                }
            }
            UiAction::TableQuickAddRow { tab_idx, block_idx } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    if matches!(w.doc.blocks.get(block_idx), Some(Block::Table(_))) {
                        w.history.record(&w.doc);
                    }
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(block_idx) {
                        t.add_row();
                        let last_r = t.rows.len().saturating_sub(1);
                        w.cursor_block = block_idx;
                        w.active_table_cell = Some((block_idx, last_r, 0));
                        w.table_cursor_char = 0;
                        w.dirty = true;
                        self.status_toast =
                            format!("Added row {} to table — start typing", last_r + 1);
                    }
                }
            }
            UiAction::TableQuickAddCol { tab_idx, block_idx } => {
                self.active_tab = tab_idx;
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    if matches!(w.doc.blocks.get(block_idx), Some(Block::Table(_))) {
                        w.history.record(&w.doc);
                    }
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(block_idx) {
                        t.add_column();
                        w.table_col_weights.retain(|(b, _)| *b != block_idx);
                        let last_c = t.cols.saturating_sub(1);
                        w.cursor_block = block_idx;
                        w.active_table_cell = Some((block_idx, 0, last_c));
                        w.table_cursor_char = t.rows[0][last_c].text.chars().count();
                        w.dirty = true;
                        self.status_toast =
                            format!("Added column {} to table", last_c + 1);
                    }
                }
            }
            UiAction::BeginDrag(drag) => {
                self.open_dropdown = None;
                if let DragInteraction::ImageResize {
                    tab_idx, block_idx, ..
                } = drag
                {
                    self.active_tab = tab_idx;
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                        w.history.record(&w.doc);
                        w.selected_image_block = Some(block_idx);
                        w.active_table_cell = None;
                        w.selection_anchor = None;
                    }
                }
                self.active_drag = drag;
                if let DragInteraction::Scrollbar {
                    tab_idx,
                    track_y,
                    track_h,
                    max_scroll,
                } = drag
                {
                    let my = self.mouse_y;
                    self.apply_scrollbar_drag(tab_idx, track_y, track_h, max_scroll, my);
                }
            }
            UiAction::TogglePdfRightInspector => {
                self.pdf_right_open = !self.pdf_right_open;
                self.status_toast = if self.pdf_right_open {
                    "Showed PDF Inspector panel".into()
                } else {
                    "Collapsed PDF Inspector panel".into()
                };
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
                self.ai_dock.focused_field = None;
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
                self.ai_dock.focused_field = None;
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
                                    12.5,
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
            UiAction::AiSelectProvider(preset) => {
                self.ai_dock.config.apply_preset(preset);
                self.ai_dock.config.save_non_secret();
                if preset == AiProviderPreset::OllamaLocal {
                    self.ai_dock.discovered_models =
                        discover_local_ollama_models(&self.ai_dock.config.endpoint_url);
                    if let Some(first) = self.ai_dock.discovered_models.first() {
                        self.ai_dock.config.model = first.clone();
                    }
                } else {
                    self.ai_dock.discovered_models.clear();
                }
                self.ai_dock.is_error = false;
                self.ai_dock.status_line = format!("Switched to {}", preset.label());
            }
            UiAction::AiSelectTask(task) => {
                self.ai_dock.selected_task = task;
                if task == AiTask::AddSections || task == AiTask::CustomPrompt {
                    self.ai_dock.focused_field = Some(AiInputField::CustomPrompt);
                }
            }
            UiAction::AiFocusField(field) => {
                self.ai_dock.focused_field = field;
            }
            UiAction::AiDetectOllamaModels => {
                let found = discover_local_ollama_models(&self.ai_dock.config.endpoint_url);
                if found.is_empty() {
                    self.ai_dock.is_error = true;
                    self.ai_dock.status_line =
                        "No local Ollama models detected on :11434 (run `ollama serve`).".into();
                } else {
                    let count = found.len();
                    if let Some(first) = found.first() {
                        self.ai_dock.config.model = first.clone();
                    }
                    self.ai_dock.discovered_models = found;
                    self.ai_dock.is_error = false;
                    self.ai_dock.status_line =
                        format!("Detected {count} local model(s) on Ollama");
                }
            }
            UiAction::AiPickDiscoveredModel(m) => {
                self.ai_dock.config.model = m.clone();
                self.ai_dock.config.save_non_secret();
                self.ai_dock.status_line = format!("Selected local model '{m}'");
            }
            UiAction::AiRunCurrentTask => {
                let task = self.ai_dock.selected_task;
                self.start_ai_task(task);
            }
            UiAction::AiCancelTask => {
                self.ai_dock.worker_rx = None;
                self.ai_dock.started_at = None;
                self.ai_dock.is_error = false;
                self.ai_dock.status_line = "Cancelled in-flight AI request.".into();
                self.status_toast = "Cancelled AI Agent request".into();
            }
            UiAction::AiApplyOutput { replace_current } => {
                self.apply_ai_output_to_document(replace_current);
            }
            UiAction::AiCopyOutput => {
                if !self.ai_dock.output_text.is_empty() {
                    set_clipboard_text(&self.ai_dock.output_text);
                    self.status_toast = "Copied AI Agent output to clipboard".into();
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
                            w.active_table_cell = None;
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
                let maybe_kind = match &self.modal {
                    Some(ActiveModal::FilePrompt { kind, .. }) => Some(*kind),
                    _ => None,
                };
                if let Some(k) = maybe_kind {
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
            ActiveModal::AddComment { text } => {
                let body = text.trim();
                if !body.is_empty() {
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                        let author = w.doc.author.clone();
                        let blk = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                        let cid =
                            w.doc
                                .add_comment(blk, &author, "2026-10-27T12:00:00Z", body);
                        w.dirty = true;
                        self.word_sidebar_open = true;
                        if self.policy.audit_log_enabled {
                            self.audit_trail.record(
                                "COMMENT_ADDED",
                                &format!("{} #{cid}", w.doc.title),
                                &author,
                            );
                        }
                        self.status_toast =
                            format!("Added threaded review comment #{cid} to paragraph {}", blk + 1);
                    }
                }
            }
            ActiveModal::EditImageCaption { block_idx, text } => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                    if matches!(w.doc.blocks.get(block_idx), Some(Block::Image(_))) {
                        w.history.record(&w.doc);
                    }
                    if let Some(Block::Image(img)) = w.doc.blocks.get_mut(block_idx) {
                        let cleaned = text.trim().to_string();
                        img.caption = cleaned.clone();
                        w.dirty = true;
                        w.selected_image_block = Some(block_idx);
                        self.status_toast = if cleaned.is_empty() {
                            "Cleared figure caption".into()
                        } else {
                            format!("Updated figure caption: '{cleaned}'")
                        };
                    }
                }
            }
            ActiveModal::FindReplace { .. }
            | ActiveModal::SnapshotsDrawer
            | ActiveModal::Governance
            | ActiveModal::About => {}
        }
    }

    fn apply_scrollbar_drag(
        &mut self,
        tab_idx: usize,
        track_y: f32,
        track_h: f32,
        max_scroll: f32,
        y: f32,
    ) {
        let ratio = ((y - track_y) / track_h.max(24.0)).clamp(0.0, 1.0);
        let pos = (ratio * max_scroll).clamp(0.0, max_scroll.max(0.0));
        match self.tabs.get_mut(tab_idx) {
            Some(StudioTab::Word(w)) => {
                w.scroll_y = pos;
                w.target_scroll_y = pos;
            }
            Some(StudioTab::Pdf(p)) => {
                p.scroll_y = pos;
                p.target_scroll_y = pos;
            }
            _ => {}
        }
    }

    pub fn handle_mouse_down(&mut self, x: f32, y: f32, mods: Modifiers) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.last_click_mods = mods;

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

        if let Some(hit) = self
            .hit_regions
            .iter()
            .rev()
            .find(|h| h.rect.contains(x, y))
            .cloned()
        {
            self.dispatch_ui_action(hit.action);
        } else {
            self.open_dropdown = None;
            self.ai_dock.focused_field = None;
            if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                w.selection_anchor = None;
            }
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
                    last.push((x, y));
                    return true;
                }
            }
        }

        match self.active_drag {
            DragInteraction::WordOutlineSplitter { pane_left } => {
                self.word_sidebar_w = ((x - pane_left) / self.ui_scale).clamp(168.0, 440.0);
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::AiDockSplitter { window_right } => {
                self.ai_dock.width = ((window_right - x) / self.ui_scale).clamp(268.0, 500.0);
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::PdfLeftSplitter { pane_left } => {
                self.pdf_left_w = ((x - pane_left) / self.ui_scale).clamp(140.0, 340.0);
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::PdfRightSplitter { pane_right } => {
                self.pdf_right_w = ((pane_right - x) / self.ui_scale).clamp(176.0, 380.0);
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::WorkspaceSplit { work_x, work_w } => {
                self.split_ratio = ((x - work_x) / work_w.max(200.0)).clamp(0.24, 0.76);
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::TableColumn {
                tab_idx,
                block_idx,
                cols,
                col_idx,
                last_x,
                table_px_w,
            } => {
                let dx = x - last_x;
                if dx.abs() > 0.5 {
                    let delta_ratio = dx / table_px_w.max(80.0);
                    if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                        w.adjust_col_divider(block_idx, cols, col_idx, delta_ratio);
                        w.dirty = true;
                    }
                    self.active_drag = DragInteraction::TableColumn {
                        tab_idx,
                        block_idx,
                        cols,
                        col_idx,
                        last_x: x,
                        table_px_w,
                    };
                }
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::Scrollbar {
                tab_idx,
                track_y,
                track_h,
                max_scroll,
            } => {
                self.apply_scrollbar_drag(tab_idx, track_y, track_h, max_scroll, y);
                self.desired_cursor = CursorShape::Hand;
                return true;
            }
            DragInteraction::WordTextSelection { tab_idx } => {
                // Zero-lag 60-120Hz live text range drag using cached per-line glyph positions
                let mut best_line: Option<(usize, usize)> = None;
                let mut best_dy = f32::MAX;
                for lb in &self.word_line_boxes {
                    if lb.tab_idx != tab_idx {
                        continue;
                    }
                    let dy = if y < lb.rect.y {
                        lb.rect.y - y
                    } else if y > lb.rect.y + lb.rect.h {
                        y - (lb.rect.y + lb.rect.h)
                    } else {
                        0.0
                    };
                    if dy < best_dy {
                        best_dy = dy;
                        best_line = Some((lb.block_idx, lb.hit_test_char(x)));
                    }
                }
                let mut moved = false;
                if let (Some((blk, ch_idx)), Some(StudioTab::Word(w))) =
                    (best_line, self.tabs.get_mut(tab_idx))
                {
                    if w.cursor_block != blk || w.cursor_char != ch_idx {
                        w.cursor_block = blk;
                        w.cursor_char = ch_idx;
                        moved = true;
                    }
                }
                self.desired_cursor = CursorShape::IBeam;
                return moved;
            }
            DragInteraction::ImageResize {
                tab_idx,
                block_idx,
                start_x,
                start_width_pt,
                scale,
                anchor_factor,
                max_width_pt,
            } => {
                if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                    if let Some(Block::Image(img)) = w.doc.blocks.get_mut(block_idx) {
                        let delta_pt = (x - start_x) * anchor_factor / scale.max(0.4);
                        img.resize_preserving_aspect(start_width_pt + delta_pt, max_width_pt);
                        w.dirty = true;
                    }
                }
                self.desired_cursor = CursorShape::ResizeEW;
                return true;
            }
            DragInteraction::None => {}
        }

        let new_hit_idx = self
            .hit_regions
            .iter()
            .enumerate()
            .rev()
            .find(|(_, h)| h.rect.contains(x, y))
            .map(|(i, _)| i);

        let new_cursor = new_hit_idx
            .and_then(|i| self.hit_regions.get(i))
            .map(|h| h.cursor)
            .unwrap_or(CursorShape::Arrow);

        let changed = new_hit_idx != self.hovered_hit_idx || new_cursor != self.desired_cursor;
        self.hovered_hit_idx = new_hit_idx;
        self.desired_cursor = new_cursor;
        changed || self.open_dropdown.is_some()
    }

    pub fn handle_mouse_up(&mut self) {
        if let Some(ActiveModal::SignaturePad { is_drawing, .. }) = &mut self.modal {
            *is_drawing = false;
        }
        if let DragInteraction::WordTextSelection { tab_idx } = self.active_drag {
            if let Some(StudioTab::Word(w)) = self.tabs.get_mut(tab_idx) {
                if !w.has_selection() {
                    w.selection_anchor = None;
                }
            }
        }
        self.active_drag = DragInteraction::None;
    }

    pub fn handle_scroll(&mut self, delta_y: f32, ctrl: bool) {
        if ctrl {
            self.adjust_zoom(if delta_y < 0.0 { 0.08 } else { -0.08 });
            return;
        }
        if let Some(dock_r) = self.ai_dock_rect {
            if self.ai_dock.open && dock_r.contains(self.mouse_x, self.mouse_y) {
                self.ai_dock.output_scroll_y =
                    (self.ai_dock.output_scroll_y + delta_y).clamp(0.0, 2400.0);
                return;
            }
        }
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                w.target_scroll_y =
                    (w.target_scroll_y + delta_y).clamp(0.0, w.max_scroll_y.max(0.0));
                // Advance an immediate partial step on the input frame for instant tactile feedback
                w.scroll_y += (w.target_scroll_y - w.scroll_y) * 0.42;
            }
            Some(StudioTab::Pdf(p)) => {
                p.target_scroll_y =
                    (p.target_scroll_y + delta_y).clamp(0.0, p.max_scroll_y.max(0.0));
                p.scroll_y += (p.target_scroll_y - p.scroll_y) * 0.42;
            }
            _ => {}
        }
    }

    fn refresh_find_matches_and_jump(&mut self) {
        let needle = match &self.modal {
            Some(ActiveModal::FindReplace { find_text, .. }) => find_text.clone(),
            _ => return,
        };
        let mut count = 0usize;
        if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
            let hits = w.doc.find_occurrences(&needle);
            count = hits.len();
            if let Some(&(b_idx, byte_off)) = hits.first() {
                if let Some(Block::Paragraph(p)) = w.doc.blocks.get(b_idx) {
                    let hay_low = p.plain_text().to_lowercase();
                    let safe_b = byte_off.min(hay_low.len());
                    let m_start = hay_low[..safe_b].chars().count();
                    let m_end = m_start + needle.chars().count();
                    w.active_table_cell = None;
                    w.selection_anchor = Some((b_idx, m_start));
                    w.cursor_block = b_idx;
                    w.cursor_char = m_end;
                    w.scroll_to_caret = true;
                }
            } else {
                w.selection_anchor = None;
            }
        }
        if let Some(ActiveModal::FindReplace { matches_count, .. }) = &mut self.modal {
            *matches_count = count;
        }
    }

    pub fn handle_key_down(&mut self, key: Key, mods: Modifiers) {
        // Global Escape closes dropdowns, modals, AI input focus, active selection, or Focus Mode
        if key == Key::Escape {
            if self.open_dropdown.is_some() {
                self.open_dropdown = None;
                return;
            }
            if self.modal.is_some() {
                self.modal = None;
                return;
            }
            if self.ai_dock.focused_field.is_some() {
                self.ai_dock.focused_field = None;
                return;
            }
            if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                if w.has_selection() {
                    w.selection_anchor = None;
                    return;
                }
                if let Some(img_idx) = w.selected_image_block.take() {
                    let next_idx = img_idx + 1;
                    if next_idx >= w.doc.blocks.len() {
                        w.doc.blocks.push(Block::Paragraph(Paragraph::new(
                            ParagraphStyle::Normal,
                            "",
                        )));
                    }
                    w.cursor_block = next_idx.min(w.doc.blocks.len().saturating_sub(1));
                    w.cursor_char = match w.doc.blocks.get(w.cursor_block) {
                        Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
                        _ => 0,
                    };
                    w.scroll_to_caret = true;
                    self.status_toast = "Deselected figure — continue writing prose below".into();
                    return;
                }
                if let Some((tb_idx, _, _)) = w.active_table_cell.take() {
                    let next_idx = tb_idx + 1;
                    if next_idx >= w.doc.blocks.len() {
                        w.doc.blocks.push(Block::Paragraph(Paragraph::new(
                            ParagraphStyle::Normal,
                            "",
                        )));
                    }
                    w.cursor_block = next_idx.min(w.doc.blocks.len().saturating_sub(1));
                    w.cursor_char = match w.doc.blocks.get(w.cursor_block) {
                        Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
                        _ => 0,
                    };
                    w.scroll_to_caret = true;
                    self.status_toast = "Exited table — continue writing prose below".into();
                    return;
                }
            }
            if self.focus_mode {
                self.focus_mode = false;
                self.status_toast = "Exited Focus Mode".into();
                return;
            }
        }

        // Keyboard navigation inside the Interactive Table Creator Popover (Arrow keys + Enter)
        if let Some(ToolbarDropdown::TableCreator {
            anchor_x,
            anchor_y,
            rows,
            cols,
        }) = self.open_dropdown
        {
            match key {
                Key::Right => {
                    let nc = (cols + 1).min(8);
                    self.open_dropdown = Some(ToolbarDropdown::TableCreator {
                        anchor_x,
                        anchor_y,
                        rows,
                        cols: nc,
                    });
                    return;
                }
                Key::Left => {
                    let nc = cols.saturating_sub(1).max(1);
                    self.open_dropdown = Some(ToolbarDropdown::TableCreator {
                        anchor_x,
                        anchor_y,
                        rows,
                        cols: nc,
                    });
                    return;
                }
                Key::Down => {
                    let nr = (rows + 1).min(8);
                    self.open_dropdown = Some(ToolbarDropdown::TableCreator {
                        anchor_x,
                        anchor_y,
                        rows: nr,
                        cols,
                    });
                    return;
                }
                Key::Up => {
                    let nr = rows.saturating_sub(1).max(1);
                    self.open_dropdown = Some(ToolbarDropdown::TableCreator {
                        anchor_x,
                        anchor_y,
                        rows: nr,
                        cols,
                    });
                    return;
                }
                Key::Enter => {
                    self.dispatch_ui_action(UiAction::InsertTableCustom { rows, cols });
                    return;
                }
                _ => {}
            }
        }

        // Clipboard Paste (`Ctrl+V`) works for Screenshots/Images, Modals, AI Dock inputs, Table cells, and Prose!
        if mods.ctrl && key == Key::Char('v') {
            if self.modal.is_none()
                && self.ai_dock.focused_field.is_none()
                && matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_)))
            {
                if let Some(img_bytes) = get_clipboard_image_bytes() {
                    if self
                        .insert_image_from_bytes("Clipboard Screenshot", img_bytes, None)
                        .is_ok()
                    {
                        return;
                    }
                }
            }
            if let Some(clip) = get_clipboard_text() {
                if self.modal.is_none()
                    && self.ai_dock.focused_field.is_none()
                    && matches!(self.tabs.get(self.active_tab), Some(StudioTab::Word(_)))
                {
                    let trimmed_path = clip.trim().trim_matches('"');
                    let candidate = Path::new(trimmed_path);
                    let ext = candidate
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp") && candidate.is_file()
                    {
                        if self.insert_image_from_path(candidate, None).is_ok() {
                            return;
                        }
                    }
                }
                self.paste_text(&clip);
            }
            return;
        }

        // Global Ctrl shortcuts
        if mods.ctrl {
            match key {
                Key::Char('k') => {
                    self.dispatch_ui_action(UiAction::OpenCommandPalette);
                    return;
                }
                Key::Char('j') => {
                    self.execute_command(CommandId::ToggleAiDock);
                    return;
                }
                Key::Char('n') => {
                    self.execute_command(CommandId::NewWordBlank);
                    return;
                }
                Key::Char('o') => {
                    self.execute_command(CommandId::OpenFile);
                    return;
                }
                Key::Char('s') => {
                    self.execute_command(CommandId::SaveFile);
                    return;
                }
                Key::Char('p') => {
                    self.execute_command(CommandId::PrintDocument);
                    return;
                }
                Key::Char('e') if mods.shift => {
                    self.execute_command(CommandId::ToggleTrackChanges);
                    return;
                }
                Key::Char('m') if mods.shift => {
                    self.execute_command(CommandId::AddReviewComment);
                    return;
                }
                Key::Char('t') if mods.shift => {
                    self.execute_command(CommandId::OpenTableCreator);
                    return;
                }
                Key::Char('i') if mods.shift => {
                    self.execute_command(CommandId::InsertImage);
                    return;
                }
                Key::Char('e') => {
                    self.execute_command(CommandId::ExportWordToPdf);
                    return;
                }
                Key::Char('f') if mods.shift => {
                    self.execute_command(CommandId::ToggleFocusMode);
                    return;
                }
                Key::Char('f') => {
                    self.execute_command(CommandId::OpenFindReplace);
                    return;
                }
                Key::Char('a') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::SelectAll);
                    return;
                }
                Key::Char('m') => {
                    self.execute_command(CommandId::PdfMergeFile);
                    return;
                }
                Key::Char('\\') => {
                    self.execute_command(CommandId::ToggleSplitView);
                    return;
                }
                Key::Char('[') => {
                    self.execute_command(CommandId::ToggleLeftSidebar);
                    return;
                }
                Key::Char('=') if mods.shift => {
                    self.execute_command(CommandId::UiScaleUp);
                    return;
                }
                Key::Char('-') if mods.shift => {
                    self.execute_command(CommandId::UiScaleDown);
                    return;
                }
                Key::Char('=') => {
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
                Key::Char('c') => {
                    self.copy_selection_or_block(false);
                    return;
                }
                Key::Char('x') => {
                    self.copy_selection_or_block(true);
                    return;
                }
                Key::Char('b') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::ToggleBold);
                    return;
                }
                Key::Char('i') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::ToggleItalic);
                    return;
                }
                Key::Char('u') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::ToggleUnderline);
                    return;
                }
                Key::Char('z') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::Undo);
                    return;
                }
                Key::Char('y') if self.modal.is_none() && self.ai_dock.focused_field.is_none() => {
                    self.execute_command(CommandId::Redo);
                    return;
                }
                _ => {}
            }
        }

        // Modal keyboard navigation (extract follow-up action first so `&mut self.modal` drops cleanly)
        if self.modal.is_some() {
            enum ModalKeyFollowUp {
                None,
                Confirm,
                ReplaceAll,
            }
            let mut follow_up = ModalKeyFollowUp::None;
            if let Some(modal) = &mut self.modal {
                match modal {
                    ActiveModal::CommandPalette { query, selected } => match key {
                        Key::Backspace => {
                            query.pop();
                            *selected = 0;
                        }
                        Key::Down => {
                            let count = filter_commands(query).len().max(1);
                            *selected = (*selected + 1) % count;
                        }
                        Key::Up => {
                            let count = filter_commands(query).len().max(1);
                            *selected = if *selected == 0 {
                                count - 1
                            } else {
                                *selected - 1
                            };
                        }
                        Key::Enter => follow_up = ModalKeyFollowUp::Confirm,
                        _ => {}
                    },
                    ActiveModal::FilePrompt { input_path, .. } => match key {
                        Key::Backspace => {
                            input_path.pop();
                        }
                        Key::Enter => follow_up = ModalKeyFollowUp::Confirm,
                        _ => {}
                    },
                    ActiveModal::SplitRange { range_spec } => match key {
                        Key::Backspace => {
                            range_spec.pop();
                        }
                        Key::Enter => follow_up = ModalKeyFollowUp::Confirm,
                        _ => {}
                    },
                    ActiveModal::Watermark { text }
                    | ActiveModal::AddComment { text }
                    | ActiveModal::EditImageCaption { text, .. } => match key {
                        Key::Backspace => {
                            text.pop();
                        }
                        Key::Enter => follow_up = ModalKeyFollowUp::Confirm,
                        _ => {}
                    },
                    ActiveModal::FindReplace {
                        find_text,
                        replace_text,
                        editing_replace,
                        ..
                    } => match key {
                        Key::Tab => *editing_replace = !*editing_replace,
                        Key::Backspace => {
                            if *editing_replace {
                                replace_text.pop();
                            } else {
                                find_text.pop();
                            }
                        }
                        Key::Enter => follow_up = ModalKeyFollowUp::ReplaceAll,
                        _ => {}
                    },
                    ActiveModal::SignaturePad { .. } => {
                        if key == Key::Enter {
                            follow_up = ModalKeyFollowUp::Confirm;
                        }
                    }
                    ActiveModal::SnapshotsDrawer | ActiveModal::Governance | ActiveModal::About => {}
                }
            }
            match follow_up {
                ModalKeyFollowUp::None => {
                    if matches!(self.modal, Some(ActiveModal::FindReplace { .. }))
                        && key == Key::Backspace
                    {
                        self.refresh_find_matches_and_jump();
                    }
                }
                ModalKeyFollowUp::Confirm => self.confirm_active_modal(),
                ModalKeyFollowUp::ReplaceAll => self.dispatch_ui_action(UiAction::FindReplaceAll),
            }
            return;
        }

        // AI Dock text field editing
        if let Some(field) = self.ai_dock.focused_field {
            match key {
                Key::Backspace => {
                    match field {
                        AiInputField::EndpointUrl => {
                            self.ai_dock.config.endpoint_url.pop();
                        }
                        AiInputField::Model => {
                            self.ai_dock.config.model.pop();
                        }
                        AiInputField::ApiKey => {
                            self.ai_dock.config.api_key.pop();
                        }
                        AiInputField::CustomPrompt => {
                            self.ai_dock.custom_prompt.pop();
                        }
                    }
                    return;
                }
                Key::Tab => {
                    self.ai_dock.focused_field = Some(match field {
                        AiInputField::EndpointUrl => AiInputField::Model,
                        AiInputField::Model => AiInputField::ApiKey,
                        AiInputField::ApiKey => AiInputField::CustomPrompt,
                        AiInputField::CustomPrompt => AiInputField::EndpointUrl,
                    });
                    return;
                }
                Key::Enter => {
                    let task = self.ai_dock.selected_task;
                    self.start_ai_task(task);
                    return;
                }
                _ => return,
            }
        }

        // Selected figure or `/image` slash command handling before mutable borrow of `self.tabs`
        if matches!(key, Key::Backspace | Key::Delete | Key::Enter) {
            enum PreWordAction {
                None,
                DeletedSelectedImage,
                ExitedSelectedImage,
                SlashImageDialog,
                SlashImagePath(String),
            }
            let mut pre = PreWordAction::None;
            if let Some(StudioTab::Word(w)) = self.tabs.get_mut(self.active_tab) {
                if let Some(img_idx) = w.selected_image_block {
                    match key {
                        Key::Backspace | Key::Delete => {
                            w.history.record(&w.doc);
                            if w.delete_image_at(img_idx) {
                                pre = PreWordAction::DeletedSelectedImage;
                            }
                        }
                        Key::Enter => {
                            w.selected_image_block = None;
                            let next_idx = img_idx + 1;
                            if next_idx >= w.doc.blocks.len() {
                                w.doc.blocks.push(Block::Paragraph(Paragraph::new(
                                    ParagraphStyle::Normal,
                                    "",
                                )));
                            }
                            w.cursor_block = next_idx.min(w.doc.blocks.len().saturating_sub(1));
                            w.cursor_char = 0;
                            w.scroll_to_caret = true;
                            pre = PreWordAction::ExitedSelectedImage;
                        }
                        _ => {}
                    }
                } else if key == Key::Enter && w.active_table_cell.is_none() {
                    let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));
                    let slash_img = w.doc.blocks.get(idx).and_then(|b| match b {
                        Block::Paragraph(p) => ImageBlock::parse_slash_image_command(&p.plain_text()),
                        _ => None,
                    });
                    if let Some(opt_path) = slash_img {
                        w.history.record(&w.doc);
                        w.doc.blocks[idx] =
                            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, ""));
                        w.cursor_char = 0;
                        w.dirty = true;
                        pre = match opt_path {
                            Some(pth) => PreWordAction::SlashImagePath(pth),
                            None => PreWordAction::SlashImageDialog,
                        };
                    }
                }
            }
            match pre {
                PreWordAction::None => {}
                PreWordAction::DeletedSelectedImage => {
                    self.status_toast = "Removed image figure (Ctrl+Z to undo)".into();
                    return;
                }
                PreWordAction::ExitedSelectedImage => {
                    self.status_toast = "Moved below figure — continue typing prose".into();
                    return;
                }
                PreWordAction::SlashImageDialog => {
                    self.prompt_or_open_modal(FileModalKind::InsertImage, "");
                    return;
                }
                PreWordAction::SlashImagePath(pth) => {
                    if let Err(e) = self.insert_image_from_path(Path::new(&pth), None) {
                        self.status_toast = format!("Image slash command error: {e}");
                    }
                    return;
                }
            }
        }

        // Active Tab Editor Navigation & Typing (with Shift range selection & Ctrl+Arrow word jump!)
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => match key {
                Key::Backspace => word_backspace(w),
                Key::Delete => word_delete(w),
                Key::Enter => {
                    if let Some(msg) = word_enter(w) {
                        self.status_toast = msg;
                    }
                }
                Key::Left => {
                    if mods.shift {
                        if w.selection_anchor.is_none() {
                            w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                        }
                        if mods.ctrl {
                            word_move_word_left(w);
                        } else {
                            word_move_left(w);
                        }
                    } else if let Some(((b0, c0), _)) = w.ordered_selection() {
                        w.cursor_block = b0;
                        w.cursor_char = c0;
                        w.selection_anchor = None;
                    } else {
                        w.selection_anchor = None;
                        if mods.ctrl {
                            word_move_word_left(w);
                        } else {
                            word_move_left(w);
                        }
                    }
                }
                Key::Right => {
                    if mods.shift {
                        if w.selection_anchor.is_none() {
                            w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                        }
                        if mods.ctrl {
                            word_move_word_right(w);
                        } else {
                            word_move_right(w);
                        }
                    } else if let Some((_, (b1, c1))) = w.ordered_selection() {
                        w.cursor_block = b1;
                        w.cursor_char = c1;
                        w.selection_anchor = None;
                    } else {
                        w.selection_anchor = None;
                        if mods.ctrl {
                            word_move_word_right(w);
                        } else {
                            word_move_right(w);
                        }
                    }
                }
                Key::Up => {
                    if mods.shift {
                        if w.selection_anchor.is_none() {
                            w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                        }
                    } else {
                        w.selection_anchor = None;
                    }
                    word_move_up(w);
                }
                Key::Down => {
                    if mods.shift {
                        if w.selection_anchor.is_none() {
                            w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                        }
                    } else {
                        w.selection_anchor = None;
                    }
                    word_move_down(w);
                }
                Key::Home => {
                    if w.active_table_cell.is_some() {
                        w.table_cursor_char = 0;
                    } else {
                        if mods.shift {
                            if w.selection_anchor.is_none() {
                                w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                            }
                        } else {
                            w.selection_anchor = None;
                        }
                        if mods.ctrl {
                            w.cursor_block = 0;
                            w.scroll_to_caret = true;
                        }
                        w.cursor_char = 0;
                    }
                }
                Key::End => {
                    if let Some((tb_idx, r, c)) = w.active_table_cell {
                        if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
                            if let Some(cell) = t.rows.get(r).and_then(|row| row.get(c)) {
                                w.table_cursor_char = cell.text.chars().count();
                            }
                        }
                    } else {
                        if mods.shift {
                            if w.selection_anchor.is_none() {
                                w.selection_anchor = Some((w.cursor_block, w.cursor_char));
                            }
                        } else {
                            w.selection_anchor = None;
                        }
                        if mods.ctrl {
                            w.cursor_block = w.doc.blocks.len().saturating_sub(1);
                            w.scroll_to_caret = true;
                        }
                        if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
                            w.cursor_char = p.plain_text().chars().count();
                        }
                    }
                }
                Key::PageDown => {
                    w.target_scroll_y =
                        (w.target_scroll_y + 420.0).clamp(0.0, w.max_scroll_y.max(0.0));
                }
                Key::PageUp => {
                    w.target_scroll_y =
                        (w.target_scroll_y - 420.0).clamp(0.0, w.max_scroll_y.max(0.0));
                }
                Key::Tab => {
                    if let Some((tb_idx, r, c)) = w.active_table_cell {
                        if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                            if mods.shift {
                                let (nr, nc) = if c > 0 {
                                    (r, c - 1)
                                } else if r > 0 {
                                    (r - 1, t.cols.saturating_sub(1))
                                } else {
                                    (0, 0)
                                };
                                let len = t
                                    .rows
                                    .get(nr)
                                    .and_then(|row| row.get(nc))
                                    .map(|cell| cell.text.chars().count())
                                    .unwrap_or(0);
                                w.active_table_cell = Some((tb_idx, nr, nc));
                                w.table_cursor_char = len;
                            } else if c + 1 < t.cols {
                                let nc = c + 1;
                                let len = t
                                    .rows
                                    .get(r)
                                    .and_then(|row| row.get(nc))
                                    .map(|cell| cell.text.chars().count())
                                    .unwrap_or(0);
                                w.active_table_cell = Some((tb_idx, r, nc));
                                w.table_cursor_char = len;
                            } else if r + 1 < t.rows.len() {
                                let nr = r + 1;
                                let len = t
                                    .rows
                                    .get(nr)
                                    .and_then(|row| row.first())
                                    .map(|cell| cell.text.chars().count())
                                    .unwrap_or(0);
                                w.active_table_cell = Some((tb_idx, nr, 0));
                                w.table_cursor_char = len;
                            } else {
                                // Tab at the very last cell appends a new row automatically!
                                t.add_row();
                                let nr = t.rows.len().saturating_sub(1);
                                w.active_table_cell = Some((tb_idx, nr, 0));
                                w.table_cursor_char = 0;
                                w.dirty = true;
                            }
                            w.scroll_to_caret = true;
                        }
                    } else if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) {
                        if mods.shift {
                            p.indent_level = p.indent_level.saturating_sub(1);
                        } else {
                            p.indent_level = (p.indent_level + 1).min(5);
                        }
                        w.dirty = true;
                    }
                }
                _ => {}
            },
            Some(StudioTab::Pdf(p)) => match key {
                Key::Backspace => {
                    if let Some(sel_id) = p.selected_text_id {
                        if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                            if let Some(item) = page.text_items.iter_mut().find(|i| i.id == sel_id)
                            {
                                item.text.pop();
                                p.dirty = true;
                            }
                        }
                    }
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
            },
            _ => {}
        }
    }

    fn copy_selection_or_block(&mut self, cut: bool) {
        if self.ai_dock.open && !self.ai_dock.output_text.is_empty() && self.ai_dock.focused_field.is_none() {
            // If on HomeHub, copy AI output; in Word/PDF, copy active block unless empty
        }
        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                if let Some(sel) = w.selected_text() {
                    set_clipboard_text(&sel);
                    let chars = sel.chars().count();
                    if cut {
                        w.history.record(&w.doc);
                        w.delete_selection();
                        self.status_toast =
                            format!("Cut selection ({chars} chars) to clipboard");
                    } else {
                        self.status_toast =
                            format!("Copied selection ({chars} chars) to clipboard");
                    }
                    return;
                }
                if cut {
                    w.history.record(&w.doc);
                }
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            set_clipboard_text(&cell.text);
                            if cut {
                                cell.text.clear();
                                w.table_cursor_char = 0;
                                w.dirty = true;
                                self.status_toast = "Cut table cell to clipboard".into();
                            } else {
                                self.status_toast = "Copied table cell to clipboard".into();
                            }
                            return;
                        }
                    }
                }
                if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) {
                    let txt = p.plain_text();
                    set_clipboard_text(&txt);
                    if cut {
                        let st = p.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                        p.spans = vec![TextSpan::styled("", st)];
                        w.cursor_char = 0;
                        w.dirty = true;
                        self.status_toast = "Cut paragraph to clipboard".into();
                    } else {
                        self.status_toast = "Copied paragraph to clipboard".into();
                    }
                }
            }
            Some(StudioTab::Pdf(p)) => {
                if let Some(sel_id) = p.selected_text_id {
                    if let Some(page) = p.pdf.pages.get(p.current_page) {
                        if let Some(item) = page.text_items.iter().find(|i| i.id == sel_id) {
                            set_clipboard_text(&item.text);
                            self.status_toast = "Copied PDF text block to clipboard".into();
                            return;
                        }
                    }
                }
                if let Some(page) = p.pdf.pages.get(p.current_page) {
                    let joined: Vec<String> =
                        page.text_items.iter().map(|i| i.text.clone()).collect();
                    set_clipboard_text(&joined.join("\n"));
                    self.status_toast = "Copied PDF page text to clipboard".into();
                }
            }
            _ => {}
        }
    }

    pub fn paste_text(&mut self, raw: &str) {
        let cleaned = raw.replace("\r\n", "\n").replace('\r', "\n");
        if cleaned.is_empty() {
            return;
        }

        if let Some(modal) = &mut self.modal {
            let single = cleaned.lines().next().unwrap_or("").trim();
            match modal {
                ActiveModal::CommandPalette { query, selected } => {
                    query.push_str(single);
                    *selected = 0;
                }
                ActiveModal::FilePrompt { input_path, .. } => input_path.push_str(single),
                ActiveModal::SplitRange { range_spec } => range_spec.push_str(single),
                ActiveModal::Watermark { text }
                | ActiveModal::AddComment { text }
                | ActiveModal::EditImageCaption { text, .. } => {
                    text.push_str(single)
                }
                ActiveModal::FindReplace {
                    find_text,
                    replace_text,
                    editing_replace,
                    ..
                } => {
                    if *editing_replace {
                        replace_text.push_str(single);
                    } else {
                        find_text.push_str(single);
                    }
                }
                _ => {}
            }
            return;
        }

        if let Some(field) = self.ai_dock.focused_field {
            let single = cleaned.lines().next().unwrap_or("").trim();
            match field {
                AiInputField::EndpointUrl => self.ai_dock.config.endpoint_url.push_str(single),
                AiInputField::Model => self.ai_dock.config.model.push_str(single),
                AiInputField::ApiKey => self.ai_dock.config.api_key.push_str(single),
                AiInputField::CustomPrompt => {
                    self.ai_dock.custom_prompt.push_str(&cleaned.replace('\n', " "));
                }
            }
            return;
        }

        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                w.history.record(&w.doc);
                w.dirty = true;
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some((grid, _)) = TableBlock::parse_delimited_grid(&cleaned) {
                        if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                            let prev_cols = t.cols;
                            t.paste_grid_at(r, c, &grid);
                            if t.cols > prev_cols {
                                w.table_col_weights.retain(|(b, _)| *b != tb_idx);
                            }
                            let gr = grid.len();
                            let gc = grid.first().map(|rw| rw.len()).unwrap_or(1);
                            self.status_toast = format!(
                                "Pasted {gr}×{gc} spreadsheet cells into table (Ctrl+Z to undo)"
                            );
                            return;
                        }
                    }
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            let flat = cleaned.replace('\n', " ");
                            if r == 0 && cell.text == format!("Column {}", c + 1) {
                                cell.text = flat.clone();
                                w.table_cursor_char = flat.chars().count();
                            } else {
                                insert_str_at_char(&mut cell.text, w.table_cursor_char, &flat);
                                w.table_cursor_char += flat.chars().count();
                            }
                            self.status_toast = "Pasted into table cell".into();
                            return;
                        }
                    }
                }

                if w.has_selection() {
                    w.delete_selection();
                }

                // Smart Tabular Paste: TSV (Excel/Sheets) or Markdown pipe tables create a structured TableBlock!
                if let Some(tbl) = TableBlock::from_delimited_text(&cleaned) {
                    let tr = tbl.rows.len();
                    let tc = tbl.cols;
                    let focus_r = if tr > 1 { 1 } else { 0 };
                    w.insert_table_smart(tbl, None, true, focus_r);
                    self.status_toast = format!(
                        "Created {tr}×{tc} table from clipboard data (Ctrl+Z to undo)"
                    );
                    return;
                }

                let lines: Vec<&str> = cleaned.split('\n').collect();
                for (li, line) in lines.iter().enumerate() {
                    if li > 0 {
                        let _ = word_enter(w);
                    }
                    for ch in line.chars() {
                        word_insert_char_no_record(w, ch);
                    }
                }
                w.scroll_to_caret = true;
                self.status_toast = "Pasted from clipboard".into();
            }
            Some(StudioTab::Pdf(p)) => {
                if let Some(sel_id) = p.selected_text_id {
                    if let Some(page) = p.pdf.pages.get_mut(p.current_page) {
                        if let Some(item) = page.text_items.iter_mut().find(|i| i.id == sel_id) {
                            item.text.push_str(&cleaned.replace('\n', " "));
                            p.dirty = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_text_input(&mut self, ch: char) {
        if let Some(modal) = &mut self.modal {
            match modal {
                ActiveModal::CommandPalette { query, selected } => {
                    query.push(ch);
                    *selected = 0;
                }
                ActiveModal::FilePrompt { input_path, .. } => input_path.push(ch),
                ActiveModal::SplitRange { range_spec } => range_spec.push(ch),
                ActiveModal::Watermark { text }
                | ActiveModal::AddComment { text }
                | ActiveModal::EditImageCaption { text, .. } => text.push(ch),
                ActiveModal::FindReplace {
                    find_text,
                    replace_text,
                    editing_replace,
                    ..
                } => {
                    if *editing_replace {
                        replace_text.push(ch);
                    } else {
                        find_text.push(ch);
                    }
                }
                ActiveModal::SignaturePad { .. }
                | ActiveModal::SnapshotsDrawer
                | ActiveModal::Governance
                | ActiveModal::About => {}
            }
            if matches!(self.modal, Some(ActiveModal::FindReplace { .. })) {
                self.refresh_find_matches_and_jump();
            }
            return;
        }

        if let Some(field) = self.ai_dock.focused_field {
            match field {
                AiInputField::EndpointUrl => self.ai_dock.config.endpoint_url.push(ch),
                AiInputField::Model => self.ai_dock.config.model.push(ch),
                AiInputField::ApiKey => self.ai_dock.config.api_key.push(ch),
                AiInputField::CustomPrompt => self.ai_dock.custom_prompt.push(ch),
            }
            return;
        }

        match self.tabs.get_mut(self.active_tab) {
            Some(StudioTab::Word(w)) => {
                if let Some((tb_idx, r, c)) = w.active_table_cell {
                    if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                        if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                            // Auto-replace untouched default `Column N` header placeholder on first typed character!
                            if r == 0 && cell.text == format!("Column {}", c + 1) {
                                cell.text.clear();
                                cell.text.push(ch);
                                w.table_cursor_char = 1;
                            } else {
                                let pos = w.table_cursor_char.min(cell.text.chars().count());
                                let byte_idx = char_to_byte(&cell.text, pos);
                                cell.text.insert(byte_idx, ch);
                                w.table_cursor_char = pos + 1;
                            }
                            w.dirty = true;
                            return;
                        }
                    }
                }
                word_insert_char(w, ch);
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
}

fn insert_str_at_char(s: &mut String, char_idx: usize, ins: &str) {
    let pos = char_idx.min(s.chars().count());
    let byte_idx = char_to_byte(s, pos);
    s.insert_str(byte_idx, ins);
}

fn word_insert_char(w: &mut WordTabState, ch: char) {
    w.doc.ensure_non_empty();
    if w.has_selection() {
        w.history.record(&w.doc);
        w.delete_selection();
    }
    let idx = w.cursor_block.min(w.doc.blocks.len() - 1);
    w.cursor_block = idx;
    if !matches!(w.doc.blocks.get(idx), Some(Block::Paragraph(_))) {
        w.doc.blocks.insert(
            idx + 1,
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")),
        );
        w.cursor_block = idx + 1;
        w.cursor_char = 0;
    }
    word_insert_char_no_record(w, ch);
    w.dirty = true;
    w.scroll_to_caret = true;
}

fn word_insert_char_no_record(w: &mut WordTabState, ch: char) {
    let track = w.doc.track_changes;
    let rev_author = w.doc.author.clone();
    let next_rev_id = if track { w.doc.next_revision_id() } else { 1 };
    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) {
        if p.spans.is_empty() {
            if track {
                p.spans.push(TextSpan::tracked(
                    ch.to_string(),
                    SpanStyle::default(),
                    RevisionMeta {
                        id: next_rev_id,
                        kind: RevisionKind::Insertion,
                        author: rev_author,
                        date_iso: "2026-10-27T12:00:00Z".into(),
                    },
                ));
                w.cursor_char = 1;
                return;
            }
            p.spans.push(TextSpan::plain(""));
        }
        let mut rem = w.cursor_char;
        let span_count = p.spans.len();
        for si in 0..span_count {
            let slen = p.spans[si].text.chars().count();
            if rem <= slen || si + 1 == span_count {
                let target_char = rem.min(slen);
                let byte_idx = char_to_byte(&p.spans[si].text, target_char);
                let is_ins = p.spans[si]
                    .revision
                    .as_ref()
                    .map(|r| r.kind == RevisionKind::Insertion)
                    .unwrap_or(false);
                if !track || is_ins {
                    p.spans[si].text.insert(byte_idx, ch);
                } else {
                    let orig = p.spans[si].clone();
                    let left = orig.text[..byte_idx].to_string();
                    let right = orig.text[byte_idx..].to_string();
                    let ins_span = TextSpan::tracked(
                        ch.to_string(),
                        orig.style.clone(),
                        RevisionMeta {
                            id: next_rev_id,
                            kind: RevisionKind::Insertion,
                            author: rev_author,
                            date_iso: "2026-10-27T12:00:00Z".into(),
                        },
                    );
                    let mut replacement = Vec::with_capacity(3);
                    if !left.is_empty() {
                        let mut l_sp = orig.clone();
                        l_sp.text = left;
                        replacement.push(l_sp);
                    }
                    replacement.push(ins_span);
                    if !right.is_empty() {
                        let mut r_sp = orig;
                        r_sp.text = right;
                        replacement.push(r_sp);
                    }
                    p.spans.splice(si..=si, replacement);
                    p.normalize_spans();
                }
                w.cursor_char += 1;
                return;
            }
            rem -= slen;
        }
    }
}

fn word_backspace(w: &mut WordTabState) {
    if w.has_selection() {
        w.history.record(&w.doc);
        w.delete_selection();
        w.scroll_to_caret = true;
        return;
    }
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
            if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                if r == 0 && cell.text == format!("Column {}", c + 1) {
                    cell.text.clear();
                    w.table_cursor_char = 0;
                    w.dirty = true;
                    return;
                }
                let len = cell.text.chars().count();
                let cur = w.table_cursor_char.min(len);
                if cur > 0 {
                    let b0 = char_to_byte(&cell.text, cur - 1);
                    let b1 = char_to_byte(&cell.text, cur);
                    cell.text.replace_range(b0..b1, "");
                    w.table_cursor_char = cur - 1;
                    w.dirty = true;
                }
                return;
            }
        }
    }

    let track = w.doc.track_changes;
    let rev_author = w.doc.author.clone();
    let next_rev_id = if track { w.doc.next_revision_id() } else { 1 };
    let mut should_merge_prev = false;
    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) {
        if w.cursor_char > 0 {
            let target = w.cursor_char - 1;
            let mut rem = target;
            for si in 0..p.spans.len() {
                let slen = p.spans[si].text.chars().count();
                if rem < slen {
                    let b0 = char_to_byte(&p.spans[si].text, rem);
                    let b1 = char_to_byte(&p.spans[si].text, rem + 1);
                    let rev_kind = p.spans[si].revision.as_ref().map(|r| r.kind);
                    if !track || rev_kind == Some(RevisionKind::Insertion) {
                        p.spans[si].text.replace_range(b0..b1, "");
                        p.normalize_spans();
                    } else if rev_kind.is_none() {
                        let orig = p.spans[si].clone();
                        let left = orig.text[..b0].to_string();
                        let del_txt = orig.text[b0..b1].to_string();
                        let right = orig.text[b1..].to_string();
                        let mut replacement = Vec::with_capacity(3);
                        if !left.is_empty() {
                            let mut l_sp = orig.clone();
                            l_sp.text = left;
                            replacement.push(l_sp);
                        }
                        replacement.push(TextSpan::tracked(
                            del_txt,
                            orig.style.clone(),
                            RevisionMeta {
                                id: next_rev_id,
                                kind: RevisionKind::Deletion,
                                author: rev_author,
                                date_iso: "2026-10-27T12:00:00Z".into(),
                            },
                        ));
                        if !right.is_empty() {
                            let mut r_sp = orig;
                            r_sp.text = right;
                            replacement.push(r_sp);
                        }
                        p.spans.splice(si..=si, replacement);
                        p.normalize_spans();
                    }
                    w.cursor_char -= 1;
                    w.dirty = true;
                    w.scroll_to_caret = true;
                    return;
                }
                rem -= slen;
            }
        } else if p.list_kind != ListKind::None {
            p.list_kind = ListKind::None;
            w.dirty = true;
            return;
        } else if w.cursor_block > 0 {
            should_merge_prev = true;
        }
    }

    if should_merge_prev {
        // Merge paragraph cleanly into previous block (even if previous paragraph had 0 spans!)
        let cur_block = w.doc.blocks.remove(w.cursor_block);
        w.cursor_block -= 1;
        if let Block::Paragraph(cur_p) = cur_block {
            let cur_text = cur_p.plain_text();
            let prev_is_para = matches!(
                w.doc.blocks.get(w.cursor_block),
                Some(Block::Paragraph(_))
            );
            let prev_is_break_or_rule = matches!(
                w.doc.blocks.get(w.cursor_block),
                Some(Block::PageBreak | Block::HorizontalRule)
            );

            if prev_is_para {
                if let Some(Block::Paragraph(prev_p)) = w.doc.blocks.get_mut(w.cursor_block) {
                    let prev_len = prev_p.plain_text().chars().count();
                    if prev_p.spans.is_empty() {
                        if cur_p.spans.is_empty() {
                            prev_p.spans.push(TextSpan::plain(cur_text));
                        } else {
                            prev_p.spans = cur_p.spans;
                        }
                    } else if !cur_text.is_empty() {
                        if let Some(last) = prev_p.spans.last_mut() {
                            last.text.push_str(&cur_text);
                        }
                    }
                    w.cursor_char = prev_len;
                }
            } else if prev_is_break_or_rule {
                w.doc.blocks[w.cursor_block] = Block::Paragraph(cur_p);
                w.cursor_char = 0;
            } else {
                w.doc
                    .blocks
                    .insert(w.cursor_block + 1, Block::Paragraph(cur_p));
            }
        }
        w.dirty = true;
        w.scroll_to_caret = true;
    }
}

fn word_delete(w: &mut WordTabState) {
    if w.has_selection() {
        w.history.record(&w.doc);
        w.delete_selection();
        w.scroll_to_caret = true;
        return;
    }
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
            if let Some(cell) = t.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                let len = cell.text.chars().count();
                let cur = w.table_cursor_char.min(len);
                if cur < len {
                    let b0 = char_to_byte(&cell.text, cur);
                    let b1 = char_to_byte(&cell.text, cur + 1);
                    cell.text.replace_range(b0..b1, "");
                    w.dirty = true;
                }
                return;
            }
        }
    }

    let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(w.cursor_block) else {
        return;
    };
    let len = p.plain_text().chars().count();
    if w.cursor_char < len {
        let mut rem = w.cursor_char;
        for span in &mut p.spans {
            let slen = span.text.chars().count();
            if rem < slen {
                let b0 = char_to_byte(&span.text, rem);
                let b1 = char_to_byte(&span.text, rem + 1);
                span.text.replace_range(b0..b1, "");
                w.dirty = true;
                return;
            }
            rem -= slen;
        }
    } else if w.cursor_block + 1 < w.doc.blocks.len() {
        if matches!(w.doc.blocks.get(w.cursor_block + 1), Some(Block::Paragraph(_))) {
            if let Block::Paragraph(next_p) = w.doc.blocks.remove(w.cursor_block + 1) {
                let next_text = next_p.plain_text();
                if let Some(Block::Paragraph(cur_p)) = w.doc.blocks.get_mut(w.cursor_block) {
                    if cur_p.spans.is_empty() {
                        cur_p.spans.push(TextSpan::plain(next_text));
                    } else if let Some(last) = cur_p.spans.last_mut() {
                        last.text.push_str(&next_text);
                    }
                }
                w.dirty = true;
            }
        }
    }
}

fn word_enter(w: &mut WordTabState) -> Option<String> {
    if w.has_selection() {
        w.history.record(&w.doc);
        w.delete_selection();
    }
    if w.active_table_cell.is_some() {
        // Inside a table cell, Enter moves cleanly to the cell directly below (or adds a row!)
        if let Some((tb_idx, r, c)) = w.active_table_cell {
            if let Some(Block::Table(t)) = w.doc.blocks.get_mut(tb_idx) {
                if r + 1 < t.rows.len() {
                    let nr = r + 1;
                    let len = t.rows[nr][c].text.chars().count();
                    w.active_table_cell = Some((tb_idx, nr, c));
                    w.table_cursor_char = len;
                } else {
                    t.add_row();
                    let nr = t.rows.len().saturating_sub(1);
                    w.active_table_cell = Some((tb_idx, nr, c));
                    w.table_cursor_char = 0;
                    w.dirty = true;
                }
                w.scroll_to_caret = true;
                return None;
            }
        }
    }

    w.history.record(&w.doc);
    let idx = w.cursor_block.min(w.doc.blocks.len().saturating_sub(1));

    // Zero-mouse table auto-creation on Enter:
    // 1. `/table`, `/table 4x5`, `/4x3`, `/invoice`, `/roadmap`, `/compare`, `/pros`, `/schedule`
    // 2. `| Header A | Header B | Header C |` Markdown pipe header line
    let line_text_opt = w.doc.blocks.get(idx).and_then(|b| match b {
        Block::Paragraph(p) => Some(p.plain_text()),
        _ => None,
    });
    if let Some(line_text) = line_text_opt {
        if let Some((tbl, weights, msg)) = TableBlock::from_slash_command(&line_text) {
            let focus_r = if weights.is_some() && tbl.rows.len() > 1 {
                1
            } else {
                0
            };
            w.doc.blocks[idx] = Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, ""));
            w.insert_table_smart(tbl, weights, true, focus_r);
            return Some(msg);
        }
        if let Some(tbl) = TableBlock::from_pipe_header_line(&line_text) {
            let cols = tbl.cols;
            w.doc.blocks[idx] = Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, ""));
            w.insert_table_smart(tbl, None, true, 1);
            return Some(format!(
                "Created {cols}-column table from pipe headers — type Row 2 & Tab across cells"
            ));
        }
    }

    // Pressing Enter on an empty bullet/checklist item exits the list cleanly
    if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
        if p.list_kind != ListKind::None && p.plain_text().trim().is_empty() {
            p.list_kind = ListKind::None;
            w.dirty = true;
            return None;
        }
    }

    let new_para = if let Some(Block::Paragraph(p)) = w.doc.blocks.get_mut(idx) {
        let full = p.plain_text();
        let split_char = w.cursor_char.min(full.chars().count());
        let b = char_to_byte(&full, split_char);
        let left = full[..b].to_string();
        let right = full[b..].to_string();
        let inherited_font = p
            .spans
            .first()
            .map(|s| s.style.font_family)
            .unwrap_or(FontFamily::Sans);
        let st = p
            .spans
            .first()
            .map(|s| s.style.clone())
            .unwrap_or_default();
        p.spans = vec![TextSpan::styled(left, st)];

        let next_list = match p.list_kind {
            ListKind::Bullet => ListKind::Bullet,
            ListKind::Checklist(_) => ListKind::Checklist(false),
            ListKind::Numbered(n) => ListKind::Numbered(n + 1),
            ListKind::None => ListKind::None,
        };
        let mut np = Paragraph::new(ParagraphStyle::Normal, right);
        np.list_kind = next_list;
        np.indent_level = p.indent_level;
        for s in &mut np.spans {
            s.style.font_family = inherited_font;
        }
        np
    } else {
        Paragraph::new(ParagraphStyle::Normal, "")
    };

    let insert_at = (idx + 1).min(w.doc.blocks.len());
    w.doc.blocks.insert(insert_at, Block::Paragraph(new_para));
    w.cursor_block = insert_at;
    w.cursor_char = 0;
    w.dirty = true;
    w.scroll_to_caret = true;
    None
}

fn word_move_left(w: &mut WordTabState) {
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if w.table_cursor_char > 0 {
            w.table_cursor_char -= 1;
        } else if c > 0 {
            if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
                let len = t.rows[r][c - 1].text.chars().count();
                w.active_table_cell = Some((tb_idx, r, c - 1));
                w.table_cursor_char = len;
            }
        }
        return;
    }
    if w.cursor_char > 0 {
        w.cursor_char -= 1;
    } else if w.cursor_block > 0 {
        w.cursor_block -= 1;
        w.cursor_char = match w.doc.blocks.get(w.cursor_block) {
            Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
            _ => 0,
        };
        w.scroll_to_caret = true;
    }
}

fn word_move_word_left(w: &mut WordTabState) {
    if w.cursor_char > 0 {
        if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
            w.cursor_char = prev_word_char_idx(&p.plain_text(), w.cursor_char);
            return;
        }
    }
    word_move_left(w);
}

fn word_move_word_right(w: &mut WordTabState) {
    if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
        let plain = p.plain_text();
        let len = plain.chars().count();
        if w.cursor_char < len {
            w.cursor_char = next_word_char_idx(&plain, w.cursor_char);
            return;
        }
    }
    word_move_right(w);
}

fn word_move_right(w: &mut WordTabState) {
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
            let len = t.rows[r][c].text.chars().count();
            if w.table_cursor_char < len {
                w.table_cursor_char += 1;
            } else if c + 1 < t.cols {
                w.active_table_cell = Some((tb_idx, r, c + 1));
                w.table_cursor_char = 0;
            }
        }
        return;
    }
    let len = match w.doc.blocks.get(w.cursor_block) {
        Some(Block::Paragraph(p)) => p.plain_text().chars().count(),
        _ => 0,
    };
    if w.cursor_char < len {
        w.cursor_char += 1;
    } else if w.cursor_block + 1 < w.doc.blocks.len() {
        w.cursor_block += 1;
        w.cursor_char = 0;
        w.scroll_to_caret = true;
    }
}

fn word_move_up(w: &mut WordTabState) {
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if r > 0 {
            if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
                let len = t.rows[r - 1][c].text.chars().count();
                w.active_table_cell = Some((tb_idx, r - 1, c));
                w.table_cursor_char = w.table_cursor_char.min(len);
                w.scroll_to_caret = true;
            }
        } else {
            w.active_table_cell = None;
            if tb_idx > 0 {
                w.cursor_block = tb_idx - 1;
            }
        }
        return;
    }
    if w.cursor_block > 0 {
        w.cursor_block -= 1;
        if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
            w.cursor_char = w.cursor_char.min(p.plain_text().chars().count());
        } else {
            w.cursor_char = 0;
        }
        w.scroll_to_caret = true;
    }
}

fn word_move_down(w: &mut WordTabState) {
    if let Some((tb_idx, r, c)) = w.active_table_cell {
        if let Some(Block::Table(t)) = w.doc.blocks.get(tb_idx) {
            if r + 1 < t.rows.len() {
                let len = t.rows[r + 1][c].text.chars().count();
                w.active_table_cell = Some((tb_idx, r + 1, c));
                w.table_cursor_char = w.table_cursor_char.min(len);
                w.scroll_to_caret = true;
            } else {
                w.active_table_cell = None;
                if tb_idx + 1 < w.doc.blocks.len() {
                    w.cursor_block = tb_idx + 1;
                    w.cursor_char = 0;
                }
            }
        }
        return;
    }
    if w.cursor_block + 1 < w.doc.blocks.len() {
        w.cursor_block += 1;
        if let Some(Block::Paragraph(p)) = w.doc.blocks.get(w.cursor_block) {
            w.cursor_char = w.cursor_char.min(p.plain_text().chars().count());
        } else {
            w.cursor_char = 0;
        }
        w.scroll_to_caret = true;
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn sanitize_filename(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "Document".into()
    } else {
        trimmed.into()
    }
}

fn discover_workspace_documents() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [".", "examples", "samples"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for ent in entries.flatten() {
                let p = ent.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "docx" | "pdf" | "md"
                    ) && p.file_name().and_then(|n| n.to_str()) != Some("README.md")
                        && p.file_name().and_then(|n| n.to_str()) != Some("CHANGELOG.md")
                    {
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

fn discover_workspace_images() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [".", "assets", "assets/icons", "public", "examples", "samples"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for ent in entries.flatten() {
                let p = ent.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "bmp"
                    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backspace_into_empty_paragraph_preserves_text() {
        let mut doc = Document::blank();
        doc.blocks = vec![
            Block::Paragraph(Paragraph {
                style: ParagraphStyle::Normal,
                alignment: Alignment::Left,
                list_kind: ListKind::None,
                indent_level: 0,
                line_spacing: 1.4,
                spans: Vec::new(), // Empty spans edge case (Issue #5)
            }),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Preserved prose")),
        ];
        let mut w = WordTabState::from_doc(doc, None);
        w.cursor_block = 1;
        w.cursor_char = 0;
        word_backspace(&mut w);
        assert_eq!(w.doc.blocks.len(), 1);
        assert_eq!(w.doc.to_plain_text().trim(), "Preserved prose");
    }

    #[test]
    fn test_table_tab_appends_row_and_column_resize() {
        let mut app = RustPadApp::new();
        app.execute_command(CommandId::InsertTable);
        if let Some(StudioTab::Word(w)) = app.tabs.get_mut(app.active_tab) {
            let tb_idx = w.cursor_block;
            w.active_table_cell = Some((tb_idx, 2, 2)); // Last cell of 3x3
            w.adjust_col_divider(tb_idx, 3, 0, 0.12);
            let weights = w.normalized_col_weights(tb_idx, 3);
            assert!(weights[0] > 0.40);
        }
        app.handle_key_down(Key::Tab, Modifiers::default());
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            if let Some(Block::Table(t)) = w.doc.blocks.get(w.cursor_block) {
                assert_eq!(t.rows.len(), 4);
            }
        }
    }

    #[test]
    fn test_enterprise_track_changes_dlp_and_audit_chain() {
        let mut app = RustPadApp::new();
        app.execute_command(CommandId::NewWordBlank);
        for ch in "Net 30".chars() {
            app.handle_text_input(ch);
        }
        // Enable Track Changes and replace "30" with "45"
        app.execute_command(CommandId::ToggleTrackChanges);
        app.handle_key_down(Key::Backspace, Modifiers::default());
        app.handle_key_down(Key::Backspace, Modifiers::default());
        if let Some(StudioTab::Word(w)) = app.tabs.get_mut(app.active_tab) {
            w.cursor_char = w.doc.blocks[0].as_paragraph_len();
        }
        app.handle_text_input('4');
        app.handle_text_input('5');

        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            let (ins, del) = w.doc.revision_counts();
            assert!(ins >= 1 && del >= 1);
        }
        app.execute_command(CommandId::AcceptAllRevisions);
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            assert_eq!(w.doc.revision_counts(), (0, 0));
            assert_eq!(w.doc.to_plain_text().trim(), "Net 45");
        }

        // Verify Strict Air-Gapped DLP blocks external cloud endpoint & logs to SHA-256 Audit Chain
        app.execute_command(CommandId::ToggleAirGappedDlp);
        app.ai_dock.config.endpoint_url = "https://api.openai.com/v1/chat/completions".into();
        app.start_ai_task(AiTask::SummarizeDocument);
        assert!(app.ai_dock.is_error);
        assert!(app.ai_dock.status_line.contains("DLP POLICY BLOCKED"));
        assert!(app.audit_trail.verify_chain().is_ok());
    }

    #[test]
    fn test_v21_range_selection_substring_formatting_and_spring_scroll() {
        let mut app = RustPadApp::new();
        app.execute_command(CommandId::NewWordBlank);
        for ch in "Smooth responsive craft".chars() {
            app.handle_text_input(ch);
        }

        // Ctrl+Shift+Left selects "craft"
        app.handle_key_down(
            Key::Left,
            Modifiers {
                ctrl: true,
                shift: true,
                alt: false,
            },
        );
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            assert_eq!(w.selected_text().as_deref(), Some("craft"));
        }

        // Ctrl+B bolds ONLY "craft", leaving "Smooth responsive " unbolded!
        app.execute_command(CommandId::ToggleBold);
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            if let Block::Paragraph(p) = &w.doc.blocks[0] {
                assert_eq!(p.spans.len(), 2);
                assert!(!p.spans[0].style.bold);
                assert!(p.spans[1].style.bold);
                assert_eq!(p.spans[1].text, "craft");
            } else {
                panic!("expected paragraph");
            }
        }

        // Typing over active selection replaces "craft" cleanly -> "Smooth responsive UX"
        app.handle_text_input('U');
        app.handle_text_input('X');
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            assert_eq!(w.doc.to_plain_text().trim(), "Smooth responsive UX");
            assert!(!w.has_selection());
        }

        // Verify 60Hz spring scroll step converges smoothly
        if let Some(StudioTab::Word(w)) = app.tabs.get_mut(app.active_tab) {
            w.max_scroll_y = 600.0;
        }
        app.handle_scroll(180.0, false);
        assert!(app.poll_background_tasks());
    }

    #[test]
    fn test_v22_smooth_table_creator_presets_slash_pipe_paste_and_esc_exit() {
        let mut app = RustPadApp::new();
        app.execute_command(CommandId::NewWordBlank);

        // 1. Ctrl+Shift+T opens TableCreator -> Arrow keys size to 4x5 -> Enter inserts 4x5 table!
        app.execute_command(CommandId::OpenTableCreator);
        app.handle_key_down(Key::Down, Modifiers::default()); // rows: 3 -> 4
        app.handle_key_down(Key::Right, Modifiers::default()); // cols: 3 -> 4
        app.handle_key_down(Key::Right, Modifiers::default()); // cols: 4 -> 5
        app.handle_key_down(Key::Enter, Modifiers::default());
        assert!(app.open_dropdown.is_none());

        // Typing into header cell (0, 0) auto-replaces untouched `Column 1` -> `Task`!
        for ch in "Task".chars() {
            app.handle_text_input(ch);
        }
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            if let Block::Table(t) = &w.doc.blocks[0] {
                assert_eq!(t.rows.len(), 4);
                assert_eq!(t.cols, 5);
                assert_eq!(t.rows[0][0].text, "Task");
            } else {
                panic!("expected 4x5 table at block 0");
            }
        }

        // Esc cleanly exits the table to the prose paragraph below!
        app.handle_key_down(Key::Escape, Modifiers::default());
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            assert!(w.active_table_cell.is_none());
            assert_eq!(w.cursor_block, 1);
        }

        // 2. Typing `| Name | Role | ETA |` + Enter auto-creates a 3-column table!
        for ch in "| Name | Role | ETA |".chars() {
            app.handle_text_input(ch);
        }
        app.handle_key_down(Key::Enter, Modifiers::default());
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            if let Block::Table(t) = &w.doc.blocks[1] {
                assert_eq!(t.cols, 3);
                assert_eq!(t.rows[0][1].text, "Role");
            } else {
                panic!("expected pipe-header table at block 1");
            }
            assert_eq!(w.active_table_cell, Some((1, 1, 0)));
        }

        // 3. Exit table and paste TSV spreadsheet data -> auto-builds structured TableBlock!
        app.handle_key_down(Key::Escape, Modifiers::default());
        app.paste_text("Item\tQty\tTotal\nAudit\t2\t$900");
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            if let Block::Table(t) = &w.doc.blocks[2] {
                assert_eq!(t.cols, 3);
                assert_eq!(t.rows.len(), 2);
                assert_eq!(t.rows[1][2].text, "$900");
            } else {
                panic!("expected TSV table at block 2");
            }
        }
    }

    #[test]
    fn test_v22_image_insertion_png_dib_resize_caption_align_and_docx_roundtrip() {
        let mut app = RustPadApp::new();
        app.execute_command(CommandId::NewWordBlank);

        // 1. Insert a 120x80 RGBA image encoded via pure-Rust PNG codec
        let png_bytes = encode_rgba_png(120, 80, &vec![40u8; 120 * 80 * 4]);
        app.insert_image_from_bytes("system-architecture.png", png_bytes, None)
            .expect("insert PNG figure");

        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            assert_eq!(w.selected_image_block, Some(0));
            assert!(matches!(w.doc.blocks.get(0), Some(Block::Image(_))));
            assert!(matches!(w.doc.blocks.get(1), Some(Block::Paragraph(_))));
        }

        // 2. Resize to 50% sheet width + right-align + add caption via modal!
        app.execute_command(CommandId::ImageScale50);
        app.execute_command(CommandId::AlignRight);
        app.execute_command(CommandId::ImageEditCaption);
        for ch in "Figure 1: Pure-Rust Compositor".chars() {
            app.handle_text_input(ch);
        }
        app.handle_key_down(Key::Enter, Modifiers::default());

        // 3. Verify full `.docx` round-trip preserves image, caption, and right alignment!
        if let Some(StudioTab::Word(w)) = app.tabs.get(app.active_tab) {
            let docx = docx_io::save_docx_bytes(&w.doc);
            let reloaded = docx_io::load_docx_bytes(&docx).expect("reload image docx");
            if let Block::Image(re_img) = &reloaded.blocks[0] {
                assert_eq!(re_img.caption, "Figure 1: Pure-Rust Compositor");
                assert_eq!(re_img.alignment, Alignment::Right);
                let (dw, dh, _) =
                    decode_any_image_rgba(&re_img.png_bytes).expect("decode reloaded PNG");
                assert_eq!((dw, dh), (120, 80));
            } else {
                panic!("expected Block::Image at block 0");
            }
        }
    }

    trait BlockParaLen {
        fn as_paragraph_len(&self) -> usize;
    }
    impl BlockParaLen for Block {
        fn as_paragraph_len(&self) -> usize {
            match self {
                Block::Paragraph(p) => p.plain_text().chars().count(),
                _ => 0,
            }
        }
    }
}
