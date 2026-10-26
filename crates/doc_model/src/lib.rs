//! Structured Rich Document Model (`Document`, `Block`, `Paragraph`, `TableBlock`, `SpanStyle`),
//! Page Setup (A4/Letter/Legal), Undo/Redo history, Find/Replace, Outline extraction, and Starter Templates.

pub mod templates;

pub use templates::TemplateKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const INK_SLATE: Self = Self::new(15, 23, 42);
    pub const COBALT: Self = Self::new(37, 99, 235);
    pub const CORAL: Self = Self::new(225, 29, 72);
    pub const EMERALD: Self = Self::new(5, 150, 105);
    pub const AMBER_HIGHLIGHT: Self = Self::new(254, 240, 138);
    pub const MINT_HIGHLIGHT: Self = Self::new(187, 247, 208);
    pub const SKY_HIGHLIGHT: Self = Self::new(191, 219, 254);
    pub const ROSE_HIGHLIGHT: Self = Self::new(254, 205, 211);
    pub const MUTED_GRAY: Self = Self::new(100, 116, 139);
    pub const TABLE_BORDER: Self = Self::new(203, 213, 225);
    pub const TABLE_HEADER_BG: Self = Self::new(241, 245, 249);

    pub fn to_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let s = hex.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    Sans,
    Serif,
    Mono,
}

impl FontFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sans => "Inter / Sans",
            Self::Serif => "Serif / Georgia",
            Self::Mono => "Mono / Code",
        }
    }

    pub fn docx_name(self) -> &'static str {
        match self {
            Self::Sans => "Calibri",
            Self::Serif => "Georgia",
            Self::Mono => "Consolas",
        }
    }

    pub fn from_docx_name(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if lower.contains("mono") || lower.contains("consol") || lower.contains("courier") {
            Self::Mono
        } else if lower.contains("serif")
            || lower.contains("times")
            || lower.contains("georgia")
            || lower.contains("cambria")
            || lower.contains("garamond")
        {
            Self::Serif
        } else {
            Self::Sans
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpanStyle {
    pub font_family: FontFamily,
    pub font_size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: RgbColor,
    pub highlight: Option<RgbColor>,
}

impl Default for SpanStyle {
    fn default() -> Self {
        Self {
            font_family: FontFamily::Sans,
            font_size_pt: 11.5,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: RgbColor::INK_SLATE,
            highlight: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub style: SpanStyle,
}

impl TextSpan {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: SpanStyle::default(),
        }
    }

    pub fn styled(text: impl Into<String>, style: SpanStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphStyle {
    Normal,
    Title,
    Subtitle,
    Heading1,
    Heading2,
    Heading3,
    Quote,
}

impl ParagraphStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Title => "Title",
            Self::Subtitle => "Subtitle",
            Self::Heading1 => "Heading 1",
            Self::Heading2 => "Heading 2",
            Self::Heading3 => "Heading 3",
            Self::Quote => "Quote",
        }
    }

    pub fn default_font_size(self) -> f32 {
        match self {
            Self::Title => 26.0,
            Self::Subtitle => 15.0,
            Self::Heading1 => 20.0,
            Self::Heading2 => 16.0,
            Self::Heading3 => 13.5,
            Self::Normal | Self::Quote => 11.5,
        }
    }

    pub fn is_bold_by_default(self) -> bool {
        matches!(
            self,
            Self::Title | Self::Heading1 | Self::Heading2 | Self::Heading3
        )
    }

    pub fn heading_level(self) -> Option<u8> {
        match self {
            Self::Title => Some(0),
            Self::Heading1 => Some(1),
            Self::Heading2 => Some(2),
            Self::Heading3 => Some(3),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    None,
    Bullet,
    Numbered(u32),
    Checklist(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub style: ParagraphStyle,
    pub alignment: Alignment,
    pub list_kind: ListKind,
    pub line_spacing: f32,
    pub indent_level: u8,
    pub spans: Vec<TextSpan>,
}

impl Paragraph {
    pub fn new(style: ParagraphStyle, text: impl Into<String>) -> Self {
        let mut span_style = SpanStyle {
            font_size_pt: style.default_font_size(),
            bold: style.is_bold_by_default(),
            ..SpanStyle::default()
        };
        if style == ParagraphStyle::Subtitle || style == ParagraphStyle::Quote {
            span_style.color = RgbColor::MUTED_GRAY;
            span_style.italic = style == ParagraphStyle::Quote;
        }
        Self {
            style,
            alignment: Alignment::Left,
            list_kind: ListKind::None,
            line_spacing: 1.35,
            indent_level: 0,
            spans: vec![TextSpan::styled(text, span_style)],
        }
    }

    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for s in &self.spans {
            out.push_str(&s.text);
        }
        out
    }

    pub fn normalize_spans(&mut self) {
        if self.spans.is_empty() {
            self.spans.push(TextSpan::styled(
                String::new(),
                SpanStyle {
                    font_size_pt: self.style.default_font_size(),
                    bold: self.style.is_bold_by_default(),
                    ..SpanStyle::default()
                },
            ));
            return;
        }
        let mut merged: Vec<TextSpan> = Vec::with_capacity(self.spans.len());
        for span in self.spans.drain(..) {
            if span.text.is_empty() && !merged.is_empty() {
                continue;
            }
            if let Some(last) = merged.last_mut() {
                if last.style == span.style {
                    last.text.push_str(&span.text);
                    continue;
                }
            }
            merged.push(span);
        }
        merged.retain(|s| !s.text.is_empty());
        if merged.is_empty() {
            merged.push(TextSpan::styled(
                String::new(),
                SpanStyle {
                    font_size_pt: self.style.default_font_size(),
                    bold: self.style.is_bold_by_default(),
                    ..SpanStyle::default()
                },
            ));
        }
        self.spans = merged;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub bold: bool,
    pub bg_color: Option<RgbColor>,
    pub alignment: Alignment,
}

impl TableCell {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            bg_color: None,
            alignment: Alignment::Left,
        }
    }

    pub fn header(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: true,
            bg_color: Some(RgbColor::TABLE_HEADER_BG),
            alignment: Alignment::Left,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableBlock {
    pub cols: usize,
    pub rows: Vec<Vec<TableCell>>,
    pub border_color: RgbColor,
}

impl TableBlock {
    pub fn new(rows_count: usize, cols_count: usize) -> Self {
        let cols = cols_count.max(1);
        let mut rows = Vec::with_capacity(rows_count.max(1));
        for r in 0..rows_count.max(1) {
            let mut row = Vec::with_capacity(cols);
            for c in 0..cols {
                if r == 0 {
                    row.push(TableCell::header(format!("Column {}", c + 1)));
                } else {
                    row.push(TableCell::plain(""));
                }
            }
            rows.push(row);
        }
        Self {
            cols,
            rows,
            border_color: RgbColor::TABLE_BORDER,
        }
    }

    pub fn add_row(&mut self) {
        let row = (0..self.cols).map(|_| TableCell::plain("")).collect();
        self.rows.push(row);
    }

    pub fn add_column(&mut self) {
        self.cols += 1;
        for (r_idx, row) in self.rows.iter_mut().enumerate() {
            if r_idx == 0 {
                row.push(TableCell::header(format!("Column {}", self.cols)));
            } else {
                row.push(TableCell::plain(""));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageBlock {
    pub name: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub png_bytes: Vec<u8>,
    pub rgba_preview: Option<(u32, u32, Vec<u8>)>,
    pub caption: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Paragraph),
    Table(TableBlock),
    Image(ImageBlock),
    HorizontalRule,
    PageBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    A4,
    Letter,
    Legal,
}

impl PaperSize {
    /// Dimensions in PDF/typographic points (1 pt = 1/72 inch).
    pub fn dimensions_pt(self, landscape: bool) -> (f32, f32) {
        let (w, h) = match self {
            Self::A4 => (595.28, 841.89),
            Self::Letter => (612.0, 792.0),
            Self::Legal => (612.0, 1008.0),
        };
        if landscape {
            (h, w)
        } else {
            (w, h)
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::A4 => "A4 (210 × 297 mm)",
            Self::Letter => "US Letter",
            Self::Legal => "US Legal",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageSetup {
    pub paper_size: PaperSize,
    pub landscape: bool,
    pub margin_top_pt: f32,
    pub margin_bottom_pt: f32,
    pub margin_left_pt: f32,
    pub margin_right_pt: f32,
    pub header_text: String,
    pub footer_text: String,
    pub show_page_numbers: bool,
    pub watermark: Option<String>,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            paper_size: PaperSize::A4,
            landscape: false,
            margin_top_pt: 64.0,
            margin_bottom_pt: 64.0,
            margin_left_pt: 64.0,
            margin_right_pt: 64.0,
            header_text: String::new(),
            footer_text: String::new(),
            show_page_numbers: true,
            watermark: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocComment {
    pub id: u32,
    pub block_index: usize,
    pub author: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub words: usize,
    pub chars: usize,
    pub chars_no_spaces: usize,
    pub paragraphs: usize,
    pub tables: usize,
    pub reading_time_minutes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineItem {
    pub block_index: usize,
    pub level: u8,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub title: String,
    pub author: String,
    pub page_setup: PageSetup,
    pub blocks: Vec<Block>,
    pub comments: Vec<DocComment>,
}

impl Default for Document {
    fn default() -> Self {
        Self::blank()
    }
}

impl Document {
    pub fn blank() -> Self {
        Self {
            title: "Untitled Document".into(),
            author: "Nour Yahyaoui".into(),
            page_setup: PageSetup::default(),
            blocks: vec![Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, ""))],
            comments: Vec::new(),
        }
    }

    pub fn ensure_non_empty(&mut self) {
        if self.blocks.is_empty() {
            self.blocks
                .push(Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")));
        }
    }

    pub fn stats(&self) -> DocumentStats {
        let mut stats = DocumentStats::default();
        for block in &self.blocks {
            match block {
                Block::Paragraph(p) => {
                    stats.paragraphs += 1;
                    let text = p.plain_text();
                    stats.words += text.split_whitespace().count();
                    stats.chars += text.chars().count();
                    stats.chars_no_spaces += text.chars().filter(|c| !c.is_whitespace()).count();
                }
                Block::Table(t) => {
                    stats.tables += 1;
                    for row in &t.rows {
                        for cell in row {
                            stats.words += cell.text.split_whitespace().count();
                            stats.chars += cell.text.chars().count();
                            stats.chars_no_spaces +=
                                cell.text.chars().filter(|c| !c.is_whitespace()).count();
                        }
                    }
                }
                _ => {}
            }
        }
        stats.reading_time_minutes = if stats.words == 0 {
            0
        } else {
            stats.words.div_ceil(220)
        };
        stats
    }

    pub fn outline(&self) -> Vec<OutlineItem> {
        let mut items = Vec::new();
        for (idx, block) in self.blocks.iter().enumerate() {
            if let Block::Paragraph(p) = block {
                if let Some(level) = p.style.heading_level() {
                    let title = p.plain_text().trim().to_string();
                    if !title.is_empty() {
                        items.push(OutlineItem {
                            block_index: idx,
                            level,
                            title,
                        });
                    }
                }
            }
        }
        items
    }

    pub fn find_occurrences(&self, needle: &str) -> Vec<(usize, usize)> {
        if needle.is_empty() {
            return Vec::new();
        }
        let needle_lower = needle.to_lowercase();
        let mut hits = Vec::new();
        for (b_idx, block) in self.blocks.iter().enumerate() {
            if let Block::Paragraph(p) = block {
                let hay = p.plain_text().to_lowercase();
                let mut start = 0usize;
                while let Some(rel) = hay[start..].find(&needle_lower) {
                    let abs = start + rel;
                    hits.push((b_idx, abs));
                    start = abs + needle_lower.len().max(1);
                }
            }
        }
        hits
    }

    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        let mut count = 0usize;
        for block in &mut self.blocks {
            match block {
                Block::Paragraph(p) => {
                    for span in &mut p.spans {
                        let matches = span.text.matches(needle).count();
                        if matches > 0 {
                            span.text = span.text.replace(needle, replacement);
                            count += matches;
                        }
                    }
                }
                Block::Table(t) => {
                    for row in &mut t.rows {
                        for cell in row {
                            let matches = cell.text.matches(needle).count();
                            if matches > 0 {
                                cell.text = cell.text.replace(needle, replacement);
                                count += matches;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        count
    }

    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            match block {
                Block::Paragraph(p) => {
                    match p.list_kind {
                        ListKind::Bullet => out.push_str("• "),
                        ListKind::Numbered(n) => out.push_str(&format!("{n}. ")),
                        ListKind::Checklist(done) => {
                            out.push_str(if done { "[x] " } else { "[ ] " })
                        }
                        ListKind::None => {}
                    }
                    out.push_str(&p.plain_text());
                    out.push('\n');
                }
                Block::Table(t) => {
                    for row in &t.rows {
                        let line: Vec<&str> = row.iter().map(|c| c.text.as_str()).collect();
                        out.push_str(&line.join(" | "));
                        out.push('\n');
                    }
                }
                Block::HorizontalRule => out.push_str("---\n"),
                Block::PageBreak => out.push_str("\n=== PAGE BREAK ===\n"),
                Block::Image(img) => out.push_str(&format!("[Image: {}]\n", img.name)),
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct UndoHistory {
    undo_stack: Vec<Document>,
    redo_stack: Vec<Document>,
    max_depth: usize,
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self::new(80)
    }
}

impl UndoHistory {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn record(&mut self, current: &Document) {
        if self.undo_stack.last() == Some(current) {
            return;
        }
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(current.clone());
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut Document) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current.clone());
            *current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut Document) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current.clone());
            *current = next;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_and_stats() {
        let doc = TemplateKind::WelcomeGuide.build();
        let stats = doc.stats();
        assert!(stats.words > 40);
        assert!(stats.tables >= 1);
        assert!(!doc.outline().is_empty());
    }

    #[test]
    fn test_undo_redo_and_replace() {
        let mut doc = TemplateKind::ModernResume.build();
        let mut history = UndoHistory::default();
        history.record(&doc);
        let replaced = doc.replace_all("Software", "Systems");
        assert!(replaced >= 1);
        assert!(history.undo(&mut doc));
        assert!(doc.to_plain_text().contains("Software"));
        assert!(history.redo(&mut doc));
        assert!(doc.to_plain_text().contains("Systems"));
    }
}
