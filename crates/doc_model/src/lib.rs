//! Enterprise Structured Rich Document Model (`Document`, `Block`, `Paragraph`, `TableBlock`,
//! `SpanStyle`, `RevisionMeta` Track Changes, `DocComment`, `SecurityClassification`,
//! Lossless OOXML `passthrough_parts`, Word-Level Legal Redline Diffing, and Undo/Redo).

pub mod templates;

use std::collections::BTreeMap;
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
    pub const TABLE_BORDER: Self = Self::new(210, 205, 196);
    pub const TABLE_HEADER_BG: Self = Self::new(244, 241, 235);
    pub const TABLE_STRIPE_LINEN: Self = Self::new(250, 248, 244);
    pub const TABLE_TINT_CREAM: Self = Self::new(252, 246, 229);
    pub const TABLE_TINT_SAGE: Self = Self::new(234, 243, 238);
    pub const TABLE_TINT_BLUE: Self = Self::new(233, 239, 246);
    pub const TABLE_HEADER_COBALT: Self = Self::new(226, 236, 250);
    pub const TABLE_BORDER_COBALT: Self = Self::new(169, 193, 226);
    pub const TABLE_STRIPE_COBALT: Self = Self::new(244, 248, 253);
    pub const TABLE_HEADER_SAGE: Self = Self::new(225, 240, 232);
    pub const TABLE_BORDER_SAGE: Self = Self::new(172, 204, 187);
    pub const TABLE_STRIPE_SAGE: Self = Self::new(244, 250, 247);
    pub const TABLE_HEADER_TERRACOTTA: Self = Self::new(250, 236, 226);
    pub const TABLE_BORDER_TERRACOTTA: Self = Self::new(219, 186, 165);
    pub const TABLE_STRIPE_TERRACOTTA: Self = Self::new(253, 248, 244);

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

/// Enterprise data-governance classification banner & metadata level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityClassification {
    Unclassified,
    Internal,
    Confidential,
    Restricted,
}

impl Default for SecurityClassification {
    fn default() -> Self {
        Self::Unclassified
    }
}

impl SecurityClassification {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unclassified => "UNCLASSIFIED",
            Self::Internal => "INTERNAL USE",
            Self::Confidential => "CONFIDENTIAL",
            Self::Restricted => "RESTRICTED // AIR-GAPPED",
        }
    }

    pub fn short_badge(self) -> &'static str {
        match self {
            Self::Unclassified => "PUBLIC",
            Self::Internal => "INTERNAL",
            Self::Confidential => "CONFIDENTIAL",
            Self::Restricted => "RESTRICTED",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Unclassified => "unclassified",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
        }
    }

    pub fn from_slug(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "internal" | "internal use" => Self::Internal,
            "confidential" => Self::Confidential,
            "restricted" | "secret" | "air-gapped" => Self::Restricted,
            _ => Self::Unclassified,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Unclassified => Self::Internal,
            Self::Internal => Self::Confidential,
            Self::Confidential => Self::Restricted,
            Self::Restricted => Self::Unclassified,
        }
    }

    pub fn cycle_next(self) -> Self {
        self.next()
    }

    pub fn badge_color(self) -> RgbColor {
        match self {
            Self::Unclassified => RgbColor::MUTED_GRAY,
            Self::Internal => RgbColor::COBALT,
            Self::Confidential => RgbColor::new(180, 83, 9),
            Self::Restricted => RgbColor::CORAL,
        }
    }
}

/// ECMA-376 OpenXML Track Changes revision kind (`<w:ins>` vs `<w:del>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionKind {
    Insertion,
    Deletion,
}

/// Attribution metadata attached to a tracked insertion or deletion span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionMeta {
    pub id: u32,
    pub kind: RevisionKind,
    pub author: String,
    pub date_iso: String,
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
            font_size_pt: 12.5,
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
    pub revision: Option<RevisionMeta>,
}

impl TextSpan {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: SpanStyle::default(),
            revision: None,
        }
    }

    pub fn styled(text: impl Into<String>, style: SpanStyle) -> Self {
        Self {
            text: text.into(),
            style,
            revision: None,
        }
    }

    pub fn tracked(text: impl Into<String>, style: SpanStyle, revision: RevisionMeta) -> Self {
        Self {
            text: text.into(),
            style,
            revision: Some(revision),
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
            Self::Title => 28.0,
            Self::Subtitle => 14.0,
            Self::Heading1 => 22.0,
            Self::Heading2 => 17.5,
            Self::Heading3 => 14.5,
            Self::Normal | Self::Quote => 12.5,
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

/// Finds the character index of the start of the previous word (for `Ctrl+Left` navigation).
pub fn prev_word_char_idx(text: &str, cur_char: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = cur_char.min(chars.len());
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    if i > 0 && !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_' {
        while i > 0
            && !chars[i - 1].is_whitespace()
            && !chars[i - 1].is_alphanumeric()
            && chars[i - 1] != '_'
        {
            i -= 1;
        }
    } else {
        while i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
            i -= 1;
        }
    }
    i
}

/// Finds the character index of the start of the next word (for `Ctrl+Right` navigation).
pub fn next_word_char_idx(text: &str, cur_char: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = cur_char.min(len);
    if i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
        while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
    } else if i < len && !chars[i].is_whitespace() {
        while i < len
            && !chars[i].is_whitespace()
            && !chars[i].is_alphanumeric()
            && chars[i] != '_'
        {
            i += 1;
        }
    }
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Returns `(start_char, end_char)` of the word at `cur_char` (for double-click word selection).
pub fn word_range_at_char(text: &str, cur_char: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let probe = cur_char.min(chars.len().saturating_sub(1));
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '\'';
    if !is_word(chars[probe]) {
        return (probe, (probe + 1).min(chars.len()));
    }
    let mut s = probe;
    while s > 0 && is_word(chars[s - 1]) {
        s -= 1;
    }
    let mut e = probe + 1;
    while e < chars.len() && is_word(chars[e]) {
        e += 1;
    }
    (s, e)
}

/// Detects whether `text` contains Right-to-Left (Arabic / Hebrew) script code points (`UAX #9`).
pub fn contains_rtl_script(text: &str) -> bool {
    text.chars().any(|ch| {
        let u = ch as u32;
        (0x0590..=0x08FF).contains(&u)
            || (0xFB1D..=0xFDFF).contains(&u)
            || (0xFE70..=0xFEFF).contains(&u)
    })
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
        let s: String = text.into();
        let alignment = if contains_rtl_script(&s) {
            Alignment::Right
        } else {
            Alignment::Left
        };
        Self {
            style,
            alignment,
            list_kind: ListKind::None,
            line_spacing: 1.35,
            indent_level: 0,
            spans: vec![TextSpan::styled(s, span_style)],
        }
    }

    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for s in &self.spans {
            out.push_str(&s.text);
        }
        out
    }

    /// Returns paragraph text with tracked deletions omitted (the accepted view).
    pub fn accepted_plain_text(&self) -> String {
        let mut out = String::new();
        for s in &self.spans {
            if s.revision.as_ref().map(|r| r.kind) != Some(RevisionKind::Deletion) {
                out.push_str(&s.text);
            }
        }
        out
    }

    pub fn has_revisions(&self) -> bool {
        self.spans.iter().any(|s| s.revision.is_some())
    }

    pub fn is_rtl(&self) -> bool {
        contains_rtl_script(&self.plain_text())
    }

    /// Returns the `SpanStyle` active at `char_idx` (biasing to the preceding character when inside text).
    pub fn style_at_char(&self, char_idx: usize) -> SpanStyle {
        let probe = char_idx.saturating_sub(1);
        let mut cur = 0usize;
        for sp in &self.spans {
            let len = sp.text.chars().count();
            if len == 0 {
                continue;
            }
            if probe < cur + len {
                return sp.style.clone();
            }
            cur += len;
        }
        self.spans
            .first()
            .map(|s| s.style.clone())
            .unwrap_or_else(|| SpanStyle {
                font_size_pt: self.style.default_font_size(),
                bold: self.style.is_bold_by_default(),
                ..SpanStyle::default()
            })
    }

    /// Returns the `RevisionMeta` at `char_idx` (or the first revision in the paragraph as fallback).
    pub fn revision_at_char(&self, char_idx: usize) -> Option<RevisionMeta> {
        let mut cur = 0usize;
        for sp in &self.spans {
            let len = sp.text.chars().count();
            if char_idx >= cur && char_idx <= cur + len {
                if let Some(rev) = &sp.revision {
                    return Some(rev.clone());
                }
            }
            cur += len;
        }
        self.spans.iter().find_map(|s| s.revision.clone())
    }

    /// Splits spans at `[start_char..end_char]` and applies `f` strictly to that character range.
    pub fn transform_range(
        &mut self,
        start_char: usize,
        end_char: usize,
        mut f: impl FnMut(&mut SpanStyle),
    ) {
        if start_char >= end_char {
            return;
        }
        let mut next_spans: Vec<TextSpan> = Vec::with_capacity(self.spans.len() + 2);
        let mut cursor = 0usize;

        for sp in self.spans.drain(..) {
            let len = sp.text.chars().count();
            let sp_start = cursor;
            let sp_end = cursor + len;
            cursor = sp_end;

            if len == 0 || sp_end <= start_char || sp_start >= end_char {
                next_spans.push(sp);
                continue;
            }

            let rel_s = start_char.saturating_sub(sp_start).min(len);
            let rel_e = end_char.saturating_sub(sp_start).min(len);

            let prefix: String = sp.text.chars().take(rel_s).collect();
            let mid: String = sp
                .text
                .chars()
                .skip(rel_s)
                .take(rel_e.saturating_sub(rel_s))
                .collect();
            let suffix: String = sp.text.chars().skip(rel_e).collect();

            if !prefix.is_empty() {
                next_spans.push(TextSpan {
                    text: prefix,
                    style: sp.style.clone(),
                    revision: sp.revision.clone(),
                });
            }
            if !mid.is_empty() {
                let mut mid_style = sp.style.clone();
                f(&mut mid_style);
                next_spans.push(TextSpan {
                    text: mid,
                    style: mid_style,
                    revision: sp.revision.clone(),
                });
            }
            if !suffix.is_empty() {
                next_spans.push(TextSpan {
                    text: suffix,
                    style: sp.style,
                    revision: sp.revision,
                });
            }
        }

        self.spans = next_spans;
        self.normalize_spans();
    }

    /// Deletes (or marks as `RevisionKind::Deletion` when `track_changes` is active) `[start_char..end_char]`.
    pub fn delete_char_range(
        &mut self,
        start_char: usize,
        end_char: usize,
        track_changes: bool,
        rev_id: u32,
        author: &str,
    ) {
        if start_char >= end_char {
            return;
        }
        let mut next_spans: Vec<TextSpan> = Vec::with_capacity(self.spans.len() + 2);
        let mut cursor = 0usize;

        for sp in self.spans.drain(..) {
            let len = sp.text.chars().count();
            let sp_start = cursor;
            let sp_end = cursor + len;
            cursor = sp_end;

            if len == 0 || sp_end <= start_char || sp_start >= end_char {
                next_spans.push(sp);
                continue;
            }

            let rel_s = start_char.saturating_sub(sp_start).min(len);
            let rel_e = end_char.saturating_sub(sp_start).min(len);

            let prefix: String = sp.text.chars().take(rel_s).collect();
            let mid: String = sp
                .text
                .chars()
                .skip(rel_s)
                .take(rel_e.saturating_sub(rel_s))
                .collect();
            let suffix: String = sp.text.chars().skip(rel_e).collect();

            if !prefix.is_empty() {
                next_spans.push(TextSpan {
                    text: prefix,
                    style: sp.style.clone(),
                    revision: sp.revision.clone(),
                });
            }
            if !mid.is_empty() && track_changes {
                match sp.revision.as_ref().map(|r| r.kind) {
                    Some(RevisionKind::Insertion) => {
                        // Deleting an uncommitted insertion simply removes it cleanly
                    }
                    Some(RevisionKind::Deletion) => {
                        next_spans.push(TextSpan {
                            text: mid,
                            style: sp.style.clone(),
                            revision: sp.revision.clone(),
                        });
                    }
                    None => {
                        next_spans.push(TextSpan::tracked(
                            mid,
                            sp.style.clone(),
                            RevisionMeta {
                                id: rev_id,
                                kind: RevisionKind::Deletion,
                                author: author.to_string(),
                                date_iso: "2026-10-27T12:00:00Z".into(),
                            },
                        ));
                    }
                }
            }
            if !suffix.is_empty() {
                next_spans.push(TextSpan {
                    text: suffix,
                    style: sp.style,
                    revision: sp.revision,
                });
            }
        }

        self.spans = next_spans;
        self.normalize_spans();
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
                let same_rev = match (&last.revision, &span.revision) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.kind == b.kind && a.author == b.author,
                    _ => false,
                };
                if last.style == span.style && same_rev {
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

    pub fn insert_row_at(&mut self, index: usize) {
        let idx = index.min(self.rows.len());
        let row = (0..self.cols).map(|_| TableCell::plain("")).collect();
        self.rows.insert(idx, row);
    }

    pub fn delete_row(&mut self, index: usize) -> bool {
        if self.rows.len() <= 1 || index >= self.rows.len() {
            return false;
        }
        self.rows.remove(index);
        true
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

    pub fn insert_column_at(&mut self, col_idx: usize) {
        let idx = col_idx.min(self.cols);
        self.cols += 1;
        for (r_idx, row) in self.rows.iter_mut().enumerate() {
            let cell = if r_idx == 0 {
                TableCell::header(format!("Column {}", idx + 1))
            } else {
                TableCell::plain("")
            };
            let insert_pos = idx.min(row.len());
            row.insert(insert_pos, cell);
        }
    }

    pub fn delete_column(&mut self, col_idx: usize) -> bool {
        if self.cols <= 1 || col_idx >= self.cols {
            return false;
        }
        self.cols -= 1;
        for row in &mut self.rows {
            if col_idx < row.len() {
                row.remove(col_idx);
            }
        }
        true
    }

    pub fn toggle_cell_bold(&mut self, row: usize, col: usize) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
            cell.bold = !cell.bold;
        }
    }

    pub fn cycle_cell_fill(&mut self, row: usize, col: usize) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
            cell.bg_color = match cell.bg_color {
                None => Some(RgbColor::TABLE_HEADER_BG),
                Some(c) if c == RgbColor::TABLE_HEADER_BG => Some(RgbColor::TABLE_TINT_CREAM),
                Some(c) if c == RgbColor::TABLE_TINT_CREAM => Some(RgbColor::TABLE_TINT_SAGE),
                Some(c) if c == RgbColor::TABLE_TINT_SAGE => Some(RgbColor::TABLE_TINT_BLUE),
                _ => None,
            };
        }
    }

    /// Detect the closest active `TableTheme` from `border_color` and header fill.
    pub fn detected_theme(&self) -> TableTheme {
        if self.border_color == RgbColor::TABLE_BORDER_COBALT {
            TableTheme::CobaltLedger
        } else if self.border_color == RgbColor::TABLE_BORDER_SAGE {
            TableTheme::SageBotanical
        } else if self.border_color == RgbColor::TABLE_BORDER_TERRACOTTA {
            TableTheme::TerracottaStudio
        } else if self
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|c| c.bg_color)
            .is_none()
        {
            TableTheme::MinimalHairline
        } else {
            TableTheme::WarmLinen
        }
    }

    /// Apply a coordinated palette theme across header, border, and optional zebra rows.
    pub fn apply_theme(&mut self, theme: TableTheme) {
        let striped = self.is_zebra_striped();
        let (hdr_bg, border, stripe_bg) = theme.palette();
        self.border_color = border;
        for (r_idx, row) in self.rows.iter_mut().enumerate() {
            for cell in row.iter_mut() {
                if r_idx == 0 {
                    cell.bold = true;
                    cell.bg_color = hdr_bg;
                } else if striped && r_idx % 2 == 0 {
                    cell.bg_color = stripe_bg;
                } else if is_stripe_fill(cell.bg_color) || r_idx > 0 && is_header_fill(cell.bg_color) {
                    cell.bg_color = None;
                }
            }
        }
    }

    pub fn cycle_theme(&mut self) -> TableTheme {
        let next = self.detected_theme().next();
        self.apply_theme(next);
        next
    }

    pub fn is_zebra_striped(&self) -> bool {
        self.rows
            .get(2)
            .and_then(|r| r.first())
            .and_then(|c| c.bg_color)
            .map(|bg| is_stripe_fill(Some(bg)))
            .unwrap_or(false)
    }

    /// Toggle delicate alternating row striping while preserving custom semantic cell tints.
    pub fn toggle_zebra_striping(&mut self) -> bool {
        let currently_striped = self.is_zebra_striped();
        let (_, _, stripe_opt) = self.detected_theme().palette();
        let stripe_color = stripe_opt.unwrap_or(RgbColor::TABLE_STRIPE_LINEN);

        if !currently_striped && self.rows.len() < 3 {
            // Ensure zebra striping is immediately visible even on a 2-row table
            for (r_idx, row) in self.rows.iter_mut().enumerate().skip(1) {
                if r_idx % 2 == 1 {
                    for cell in row.iter_mut() {
                        if cell.bg_color.is_none() {
                            cell.bg_color = Some(stripe_color);
                        }
                    }
                }
            }
            return true;
        }

        let enable = !currently_striped;
        for (r_idx, row) in self.rows.iter_mut().enumerate().skip(1) {
            for cell in row.iter_mut() {
                if enable {
                    if r_idx % 2 == 0 && cell.bg_color.is_none() {
                        cell.bg_color = Some(stripe_color);
                    }
                } else if is_stripe_fill(cell.bg_color) {
                    cell.bg_color = None;
                }
            }
        }
        enable
    }

    /// Parse a single Markdown pipe header line such as `| Feature | Owner | Target |`
    /// into a ready-to-type 2-row `TableBlock` (headers in Row 0, empty data cells in Row 1).
    pub fn from_pipe_header_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            return None;
        }
        let inner = trimmed.trim_matches('|');
        if !inner.contains('|') {
            return None;
        }
        let headers: Vec<String> = inner
            .split('|')
            .map(|part| part.trim().to_string())
            .collect();
        if headers.len() < 2 || headers.len() > 12 {
            return None;
        }
        if headers.iter().all(|h| is_markdown_separator_cell(h)) {
            return None;
        }
        let cols = headers.len();
        let header_row = headers
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                if h.is_empty() {
                    TableCell::header(format!("Column {}", i + 1))
                } else {
                    TableCell::header(h)
                }
            })
            .collect();
        let first_data_row = (0..cols).map(|_| TableCell::plain("")).collect();
        Some(Self {
            cols,
            rows: vec![header_row, first_data_row],
            border_color: RgbColor::TABLE_BORDER,
        })
    }

    /// Parse keyboard slash table shorthands on `Enter`:
    /// `/table`, `/table 4x5`, `/4x3`, `/table invoice`, `/invoice`, `/roadmap`, `/compare`, `/pros`, `/schedule`.
    pub fn from_slash_command(line: &str) -> Option<(Self, Option<Vec<f32>>, String)> {
        let trimmed = line.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let body = trimmed[1..].trim();
        if body.is_empty() {
            return None;
        }
        let lower = body.to_ascii_lowercase();

        // Direct preset shorthands: `/invoice`, `/pricing`, `/tracker`, `/roadmap`, `/compare`, `/pros`, `/schedule`
        let preset_match = match lower.as_str() {
            "invoice" | "pricing" | "table invoice" | "table pricing" => {
                Some(TablePreset::PricingInvoice)
            }
            "tracker" | "roadmap" | "table tracker" | "table roadmap" => {
                Some(TablePreset::ProjectTracker)
            }
            "compare" | "matrix" | "table compare" | "table matrix" => {
                Some(TablePreset::ComparisonMatrix)
            }
            "pros" | "proscons" | "table pros" => Some(TablePreset::ProsAndCons),
            "schedule" | "agenda" | "table schedule" | "table agenda" => {
                Some(TablePreset::WeeklySchedule)
            }
            _ => None,
        };
        if let Some(preset) = preset_match {
            let (tbl, weights) = preset.build();
            return Some((
                tbl,
                weights,
                format!("Inserted {} table — Tab to move across cells", preset.label()),
            ));
        }

        if lower == "table" || lower == "tbl" {
            return Some((
                Self::new(3, 3),
                None,
                "Inserted 3×3 table — Tab to advance, Esc to exit".into(),
            ));
        }

        let dim_part = if let Some(rest) = lower.strip_prefix("table ") {
            rest.trim()
        } else if let Some(rest) = lower.strip_prefix("tbl ") {
            rest.trim()
        } else {
            lower.as_str()
        };

        if let Some((r, c)) = parse_grid_dimensions(dim_part) {
            let rows = r.clamp(1, 24);
            let cols = c.clamp(1, 12);
            return Some((
                Self::new(rows, cols),
                None,
                format!(
                    "Inserted {}×{} table — Tab to advance, drag borders to resize",
                    rows, cols
                ),
            ));
        }

        None
    }

    /// Parse TSV (Excel / Google Sheets / Calc) or multi-line Markdown pipe text into a raw cell grid
    /// plus optional column alignments inferred from Markdown `:---` / `:---:` / `---:` rows.
    pub fn parse_delimited_grid(raw: &str) -> Option<(Vec<Vec<String>>, Vec<Alignment>)> {
        let non_empty_lines: Vec<&str> = raw
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.trim().is_empty())
            .collect();
        if non_empty_lines.is_empty() {
            return None;
        }

        // 1. Check TSV (tab-separated spreadsheet paste)
        let tab_lines = non_empty_lines.iter().filter(|l| l.contains('\t')).count();
        if (non_empty_lines.len() >= 2 && tab_lines * 2 >= non_empty_lines.len())
            || (non_empty_lines.len() == 1 && non_empty_lines[0].matches('\t').count() >= 2)
        {
            let mut max_cols = 0usize;
            let mut grid: Vec<Vec<String>> = non_empty_lines
                .iter()
                .map(|line| {
                    let row: Vec<String> =
                        line.split('\t').map(|s| s.trim().to_string()).collect();
                    max_cols = max_cols.max(row.len());
                    row
                })
                .collect();
            if max_cols < 2 || max_cols > 16 {
                return None;
            }
            for row in &mut grid {
                while row.len() < max_cols {
                    row.push(String::new());
                }
            }
            return Some((grid, vec![Alignment::Left; max_cols]));
        }

        // 2. Check multi-line Markdown pipe table (`| A | B |\n|---|---|\n| 1 | 2 |`)
        if non_empty_lines.len() >= 2
            && non_empty_lines
                .iter()
                .all(|l| l.trim().contains('|'))
        {
            let mut grid = Vec::new();
            let mut alignments: Vec<Alignment> = Vec::new();
            let mut max_cols = 0usize;
            for line in non_empty_lines {
                let inner = line.trim().trim_matches('|');
                let parts: Vec<&str> = inner.split('|').map(|p| p.trim()).collect();
                if parts.len() < 2 {
                    return None;
                }
                if parts.iter().all(|p| is_markdown_separator_cell(p)) {
                    alignments = parts.iter().map(|p| parse_md_alignment(p)).collect();
                    continue;
                }
                max_cols = max_cols.max(parts.len());
                grid.push(parts.into_iter().map(|s| s.to_string()).collect::<Vec<_>>());
            }
            if grid.is_empty() || max_cols < 2 || max_cols > 16 {
                return None;
            }
            for row in &mut grid {
                while row.len() < max_cols {
                    row.push(String::new());
                }
            }
            while alignments.len() < max_cols {
                alignments.push(Alignment::Left);
            }
            return Some((grid, alignments));
        }

        None
    }

    /// Build a styled `TableBlock` directly from TSV or Markdown pipe clipboard text.
    pub fn from_delimited_text(raw: &str) -> Option<Self> {
        let (grid, alignments) = Self::parse_delimited_grid(raw)?;
        let cols = grid.first()?.len().max(1);
        let mut rows = Vec::with_capacity(grid.len().max(2));
        for (r_idx, r_vals) in grid.into_iter().enumerate() {
            let mut row = Vec::with_capacity(cols);
            for (c_idx, text) in r_vals.into_iter().enumerate() {
                let align = alignments.get(c_idx).copied().unwrap_or(Alignment::Left);
                let mut cell = if r_idx == 0 {
                    TableCell::header(text)
                } else {
                    TableCell::plain(text)
                };
                cell.alignment = align;
                row.push(cell);
            }
            rows.push(row);
        }
        if rows.len() == 1 {
            rows.push((0..cols).map(|_| TableCell::plain("")).collect());
        }
        Some(Self {
            cols,
            rows,
            border_color: RgbColor::TABLE_BORDER,
        })
    }

    /// Paste a 2D cell grid into an existing table starting at `(start_row, start_col)`,
    /// automatically expanding rows and columns so spreadsheet pastes never clip.
    pub fn paste_grid_at(&mut self, start_row: usize, start_col: usize, grid: &[Vec<String>]) {
        if grid.is_empty() {
            return;
        }
        let needed_cols = grid
            .iter()
            .map(|r| start_col + r.len())
            .max()
            .unwrap_or(self.cols)
            .min(16);
        while self.cols < needed_cols {
            self.add_column();
        }
        let needed_rows = (start_row + grid.len()).min(128);
        while self.rows.len() < needed_rows {
            self.add_row();
        }
        for (dr, r_vals) in grid.iter().enumerate() {
            let tr = start_row + dr;
            if tr >= self.rows.len() {
                break;
            }
            for (dc, val) in r_vals.iter().enumerate() {
                let tc = start_col + dc;
                if tc >= self.cols {
                    break;
                }
                if let Some(cell) = self.rows.get_mut(tr).and_then(|r| r.get_mut(tc)) {
                    cell.text = val.clone();
                }
            }
        }
    }
}

fn is_stripe_fill(bg: Option<RgbColor>) -> bool {
    matches!(
        bg,
        Some(c)
            if c == RgbColor::TABLE_STRIPE_LINEN
                || c == RgbColor::TABLE_STRIPE_COBALT
                || c == RgbColor::TABLE_STRIPE_SAGE
                || c == RgbColor::TABLE_STRIPE_TERRACOTTA
    )
}

fn is_header_fill(bg: Option<RgbColor>) -> bool {
    matches!(
        bg,
        Some(c)
            if c == RgbColor::TABLE_HEADER_BG
                || c == RgbColor::TABLE_HEADER_COBALT
                || c == RgbColor::TABLE_HEADER_SAGE
                || c == RgbColor::TABLE_HEADER_TERRACOTTA
    )
}

fn is_markdown_separator_cell(cell: &str) -> bool {
    let t = cell.trim();
    if t.is_empty() {
        return false;
    }
    let core = t.trim_matches(':');
    core.len() >= 2 && core.chars().all(|ch| ch == '-')
}

fn parse_md_alignment(cell: &str) -> Alignment {
    let t = cell.trim();
    match (t.starts_with(':'), t.ends_with(':')) {
        (true, true) => Alignment::Center,
        (false, true) => Alignment::Right,
        _ => Alignment::Left,
    }
}

fn parse_grid_dimensions(spec: &str) -> Option<(usize, usize)> {
    let normalized = spec.replace('×', "x").replace('*', "x");
    let parts: Vec<&str> = if normalized.contains('x') {
        normalized.split('x').collect()
    } else {
        normalized.split_whitespace().collect()
    };
    if parts.len() != 2 {
        return None;
    }
    let r = parts[0].trim().parse::<usize>().ok()?;
    let c = parts[1].trim().parse::<usize>().ok()?;
    if r == 0 || c == 0 {
        return None;
    }
    Some((r, c))
}

/// Coordinated 1-click visual themes for `TableBlock` that round-trip natively to `.docx` and `.pdf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableTheme {
    WarmLinen,
    CobaltLedger,
    SageBotanical,
    TerracottaStudio,
    MinimalHairline,
}

impl TableTheme {
    pub const ALL: [TableTheme; 5] = [
        Self::WarmLinen,
        Self::CobaltLedger,
        Self::SageBotanical,
        Self::TerracottaStudio,
        Self::MinimalHairline,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::WarmLinen => "Warm Linen",
            Self::CobaltLedger => "Cobalt Ledger",
            Self::SageBotanical => "Sage Botanical",
            Self::TerracottaStudio => "Terracotta Studio",
            Self::MinimalHairline => "Minimal Hairline",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::WarmLinen => Self::CobaltLedger,
            Self::CobaltLedger => Self::SageBotanical,
            Self::SageBotanical => Self::TerracottaStudio,
            Self::TerracottaStudio => Self::MinimalHairline,
            Self::MinimalHairline => Self::WarmLinen,
        }
    }

    pub fn palette(self) -> (Option<RgbColor>, RgbColor, Option<RgbColor>) {
        match self {
            Self::WarmLinen => (
                Some(RgbColor::TABLE_HEADER_BG),
                RgbColor::TABLE_BORDER,
                Some(RgbColor::TABLE_STRIPE_LINEN),
            ),
            Self::CobaltLedger => (
                Some(RgbColor::TABLE_HEADER_COBALT),
                RgbColor::TABLE_BORDER_COBALT,
                Some(RgbColor::TABLE_STRIPE_COBALT),
            ),
            Self::SageBotanical => (
                Some(RgbColor::TABLE_HEADER_SAGE),
                RgbColor::TABLE_BORDER_SAGE,
                Some(RgbColor::TABLE_STRIPE_SAGE),
            ),
            Self::TerracottaStudio => (
                Some(RgbColor::TABLE_HEADER_TERRACOTTA),
                RgbColor::TABLE_BORDER_TERRACOTTA,
                Some(RgbColor::TABLE_STRIPE_TERRACOTTA),
            ),
            Self::MinimalHairline => (
                None,
                RgbColor::TABLE_BORDER,
                Some(RgbColor::TABLE_STRIPE_LINEN),
            ),
        }
    }
}

/// Curated 1-click ready-to-edit table templates inside the Visual Table Creator Popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablePreset {
    PricingInvoice,
    ProjectTracker,
    ComparisonMatrix,
    ProsAndCons,
    WeeklySchedule,
}

impl TablePreset {
    pub const ALL: [TablePreset; 5] = [
        Self::PricingInvoice,
        Self::ProjectTracker,
        Self::ComparisonMatrix,
        Self::ProsAndCons,
        Self::WeeklySchedule,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PricingInvoice => "Pricing / Invoice",
            Self::ProjectTracker => "Project Tracker",
            Self::ComparisonMatrix => "Comparison Matrix",
            Self::ProsAndCons => "Pros & Cons",
            Self::WeeklySchedule => "Schedule / Agenda",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::PricingInvoice => "4×4 · $",
            Self::ProjectTracker => "4×4 · Status",
            Self::ComparisonMatrix => "4×3 · vs",
            Self::ProsAndCons => "4×2 · Split",
            Self::WeeklySchedule => "5×3 · Zebra",
        }
    }

    /// Build the styled `TableBlock` plus optional natural proportional column weights.
    pub fn build(self) -> (TableBlock, Option<Vec<f32>>) {
        match self {
            Self::PricingInvoice => {
                let mut t = TableBlock::new(4, 4);
                t.border_color = RgbColor::TABLE_BORDER_COBALT;
                let mut h_qty = TableCell::header("Qty");
                h_qty.alignment = Alignment::Center;
                h_qty.bg_color = Some(RgbColor::TABLE_HEADER_COBALT);
                let mut h_unit = TableCell::header("Unit Price");
                h_unit.alignment = Alignment::Right;
                h_unit.bg_color = Some(RgbColor::TABLE_HEADER_COBALT);
                let mut h_total = TableCell::header("Line Total");
                h_total.alignment = Alignment::Right;
                h_total.bg_color = Some(RgbColor::TABLE_HEADER_COBALT);
                let mut h_desc = TableCell::header("Deliverable / Item");
                h_desc.bg_color = Some(RgbColor::TABLE_HEADER_COBALT);
                t.rows[0] = vec![h_desc, h_qty, h_unit, h_total];

                let items = [
                    ("Core Architecture & Engineering", "1", "$1,200.00", "$1,200.00"),
                    ("QA Verification & Release Packaging", "1", "$650.00", "$650.00"),
                ];
                for (idx, (desc, qty, unit, tot)) in items.iter().enumerate() {
                    let mut c_qty = TableCell::plain(*qty);
                    c_qty.alignment = Alignment::Center;
                    let mut c_unit = TableCell::plain(*unit);
                    c_unit.alignment = Alignment::Right;
                    let mut c_tot = TableCell::plain(*tot);
                    c_tot.alignment = Alignment::Right;
                    t.rows[idx + 1] = vec![TableCell::plain(*desc), c_qty, c_unit, c_tot];
                }
                let mut sum_lbl = TableCell::plain("Total Due");
                sum_lbl.bold = true;
                sum_lbl.bg_color = Some(RgbColor::TABLE_TINT_CREAM);
                let mut sum_blank1 = TableCell::plain("");
                sum_blank1.bg_color = Some(RgbColor::TABLE_TINT_CREAM);
                let mut sum_blank2 = TableCell::plain("USD");
                sum_blank2.alignment = Alignment::Right;
                sum_blank2.bg_color = Some(RgbColor::TABLE_TINT_CREAM);
                let mut sum_val = TableCell::plain("$1,850.00");
                sum_val.bold = true;
                sum_val.alignment = Alignment::Right;
                sum_val.bg_color = Some(RgbColor::TABLE_TINT_CREAM);
                t.rows[3] = vec![sum_lbl, sum_blank1, sum_blank2, sum_val];

                (t, Some(vec![0.46, 0.12, 0.21, 0.21]))
            }
            Self::ProjectTracker => {
                let mut t = TableBlock::new(4, 4);
                t.rows[0] = vec![
                    TableCell::header("Milestone / Task"),
                    TableCell::header("Owner"),
                    TableCell::header("Status"),
                    TableCell::header("Target"),
                ];
                let mut s_done = TableCell::plain("Complete");
                s_done.bold = true;
                s_done.bg_color = Some(RgbColor::TABLE_TINT_SAGE);
                let mut s_prog = TableCell::plain("In Progress");
                s_prog.bold = true;
                s_prog.bg_color = Some(RgbColor::TABLE_TINT_BLUE);
                let mut s_plan = TableCell::plain("Planned");
                s_plan.bg_color = Some(RgbColor::TABLE_TINT_CREAM);

                t.rows[1] = vec![
                    TableCell::plain("Design System & Spec"),
                    TableCell::plain("Design Lead"),
                    s_done,
                    TableCell::plain("Week 1"),
                ];
                t.rows[2] = vec![
                    TableCell::plain("Core Engine Implementation"),
                    TableCell::plain("Systems Eng"),
                    s_prog,
                    TableCell::plain("Week 2"),
                ];
                t.rows[3] = vec![
                    TableCell::plain("Security Audit & Sign-Off"),
                    TableCell::plain("QA / SecOps"),
                    s_plan,
                    TableCell::plain("Week 3"),
                ];
                (t, Some(vec![0.42, 0.20, 0.20, 0.18]))
            }
            Self::ComparisonMatrix => {
                let mut t = TableBlock::new(4, 3);
                let mut h_prop = TableCell::header("Proposed Option");
                h_prop.bg_color = Some(RgbColor::TABLE_HEADER_COBALT);
                t.rows[0] = vec![
                    TableCell::header("Evaluation Criterion"),
                    TableCell::header("Current Baseline"),
                    h_prop,
                ];
                let rows_data = [
                    ("Latency / Speed", "240 ms average", "< 16 ms native VSync"),
                    ("Offline Operation", "Requires cloud login", "100% offline-first"),
                    ("Annual Cost", "$24 / seat / mo", "Open-source MIT"),
                ];
                for (i, (crit, base, prop)) in rows_data.iter().enumerate() {
                    let mut c_crit = TableCell::plain(*crit);
                    c_crit.bold = true;
                    let mut c_prop = TableCell::plain(*prop);
                    c_prop.bg_color = Some(RgbColor::TABLE_STRIPE_COBALT);
                    t.rows[i + 1] = vec![c_crit, TableCell::plain(*base), c_prop];
                }
                (t, Some(vec![0.32, 0.34, 0.34]))
            }
            Self::ProsAndCons => {
                let mut t = TableBlock::new(4, 2);
                let mut h_pros = TableCell::header("Advantages / Pros");
                h_pros.bg_color = Some(RgbColor::TABLE_HEADER_SAGE);
                let mut h_cons = TableCell::header("Trade-Offs / Considerations");
                h_cons.bg_color = Some(RgbColor::TABLE_HEADER_TERRACOTTA);
                t.rows[0] = vec![h_pros, h_cons];
                for r in 1..4 {
                    let mut p = TableCell::plain(if r == 1 {
                        "Fast, predictable native execution"
                    } else {
                        ""
                    });
                    p.bg_color = Some(RgbColor::TABLE_STRIPE_SAGE);
                    let mut c = TableCell::plain(if r == 1 {
                        "Initial migration & onboarding effort"
                    } else {
                        ""
                    });
                    c.bg_color = Some(RgbColor::TABLE_STRIPE_TERRACOTTA);
                    t.rows[r] = vec![p, c];
                }
                (t, Some(vec![0.50, 0.50]))
            }
            Self::WeeklySchedule => {
                let mut t = TableBlock::new(5, 3);
                t.rows[0] = vec![
                    TableCell::header("Time"),
                    TableCell::header("Session / Deliverable"),
                    TableCell::header("Facilitator"),
                ];
                let slots = [
                    ("09:00", "Kickoff & Architecture Alignment", "Engineering"),
                    ("10:30", "Interactive UX Critique & Polish", "Product"),
                    ("13:30", "Security & Compliance Review", "SecOps"),
                    ("15:00", "Release Verification & Demo", "All Hands"),
                ];
                for (i, (tm, topic, lead)) in slots.iter().enumerate() {
                    let mut c_tm = TableCell::plain(*tm);
                    c_tm.bold = true;
                    t.rows[i + 1] = vec![c_tm, TableCell::plain(*topic), TableCell::plain(*lead)];
                }
                t.toggle_zebra_striping();
                (t, Some(vec![0.18, 0.56, 0.26]))
            }
        }
    }
}

/// Reference-counted immutable byte buffer (`O(1)` clone & pointer-equality fast path)
/// so `Document` clones in `UndoHistory` and `SnapshotVault` never duplicate multi-MB media.
#[derive(Clone, Default)]
pub struct SharedBytes(std::sync::Arc<[u8]>);

impl SharedBytes {
    pub fn new() -> Self {
        Self(std::sync::Arc::from(&[][..]))
    }

    pub fn from_vec(v: Vec<u8>) -> Self {
        Self(std::sync::Arc::from(v))
    }

    pub fn from_slice(s: &[u8]) -> Self {
        Self(std::sync::Arc::from(s))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl std::ops::Deref for SharedBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for SharedBytes {
    fn from(v: Vec<u8>) -> Self {
        Self::from_vec(v)
    }
}

impl From<&[u8]> for SharedBytes {
    fn from(s: &[u8]) -> Self {
        Self::from_slice(s)
    }
}

impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0) || self.0[..] == other.0[..]
    }
}

impl Eq for SharedBytes {}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedBytes({} B)", self.0.len())
    }
}

/// Zero-allocation header dimension probe for PNG, JPEG (`SOF0`..`SOF2`), BMP, DIB, and GIF.
/// Allows 100+ screenshot `.docx` layouts to compute exact aspect ratios in microseconds
/// without decoding gigabytes of pixel data upfront.
pub fn probe_media_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // 1. PNG IHDR (bytes 16..24)
    if bytes.len() >= 24
        && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
        && &bytes[12..16] == b"IHDR"
    {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        if w > 0 && h > 0 {
            return Some((w, h));
        }
    }
    // 2. JPEG SOF0 / SOF1 / SOF2 marker scan (zero entropy decoding)
    if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        let mut pos = 2usize;
        while pos + 1 < bytes.len() {
            if bytes[pos] != 0xFF {
                pos += 1;
                continue;
            }
            while pos < bytes.len() && bytes[pos] == 0xFF {
                pos += 1;
            }
            if pos >= bytes.len() {
                break;
            }
            let marker = bytes[pos];
            pos += 1;
            if marker == 0xD9 || marker == 0xDA {
                break;
            }
            if marker == 0x00 || marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01
            {
                continue;
            }
            if pos + 2 > bytes.len() {
                break;
            }
            let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
            if seg_len < 2 || pos + seg_len > bytes.len() {
                break;
            }
            if matches!(marker, 0xC0 | 0xC1 | 0xC2) && seg_len >= 7 {
                let h = u16::from_be_bytes([bytes[pos + 3], bytes[pos + 4]]) as u32;
                let w = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
                if w > 0 && h > 0 {
                    return Some((w, h));
                }
            }
            pos += seg_len;
        }
    }
    // 3. BMP ("BM" + BITMAPINFOHEADER)
    if bytes.len() >= 26 && &bytes[0..2] == b"BM" {
        let w = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]).unsigned_abs();
        let h = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).unsigned_abs();
        if w > 0 && h > 0 {
            return Some((w, h));
        }
    }
    // 4. GIF87a / GIF89a Logical Screen Descriptor
    if bytes.len() >= 10 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        let w = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
        let h = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
        if w > 0 && h > 0 {
            return Some((w, h));
        }
    }
    // 5. Raw Win32 DIB header
    if bytes.len() >= 40 {
        let hdr = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if matches!(hdr, 40 | 52 | 56 | 108 | 124) {
            let w = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]).unsigned_abs();
            let h = i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]).unsigned_abs();
            if w > 0 && h > 0 && w <= 32_768 && h <= 32_768 {
                return Some((w, h));
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageBlock {
    pub name: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub png_bytes: SharedBytes,
    pub rgba_preview: Option<(u32, u32, SharedBytes)>,
    pub intrinsic_px: Option<(u32, u32)>,
    pub caption: String,
    pub alignment: Alignment,
}

impl ImageBlock {
    pub fn new(
        name: impl Into<String>,
        width_pt: f32,
        height_pt: f32,
        raw_bytes: impl Into<SharedBytes>,
    ) -> Self {
        let png_bytes = raw_bytes.into();
        let intrinsic_px = probe_media_dimensions(&png_bytes);
        Self {
            name: name.into(),
            width_pt: width_pt.max(24.0),
            height_pt: height_pt.max(24.0),
            png_bytes,
            rgba_preview: None,
            intrinsic_px,
            caption: String::new(),
            alignment: Alignment::Center,
        }
    }

    /// Construct a crisp, sheet-fitted `ImageBlock` from decoded RGBA pixels + original payload bytes.
    pub fn from_decoded(
        name: impl Into<String>,
        pixel_w: u32,
        pixel_h: u32,
        rgba: Vec<u8>,
        raw_bytes: impl Into<SharedBytes>,
        max_content_width_pt: f32,
    ) -> Self {
        let pw = pixel_w.max(1) as f32;
        let ph = pixel_h.max(1) as f32;
        let aspect = (pw / ph).clamp(0.08, 12.0);
        // Convert 96-DPI pixels to 72-DPI typographic points (`0.75`), smoothly fitted to page margins
        let max_w = max_content_width_pt.max(140.0);
        let natural_w = (pw * 0.75).max(48.0);
        let width_pt = natural_w.min(max_w);
        let height_pt = (width_pt / aspect).clamp(24.0, 620.0);
        let fitted_w = (height_pt * aspect).min(max_w);

        // Keep inline `rgba_preview` only for compact buffers (<= 4 MiB); larger screenshots
        // stream through the bounded viewport thumbnail LRU cache on demand.
        let inline_preview = if rgba.len() <= 4 * 1024 * 1024 {
            Some((pixel_w, pixel_h, SharedBytes::from_vec(rgba)))
        } else {
            None
        };

        Self {
            name: name.into(),
            width_pt: fitted_w,
            height_pt,
            png_bytes: raw_bytes.into(),
            rgba_preview: inline_preview,
            intrinsic_px: Some((pixel_w, pixel_h)),
            caption: String::new(),
            alignment: Alignment::Center,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if let Some((iw, ih)) = self.intrinsic_px {
            if iw > 0 && ih > 0 {
                return (iw as f32 / ih as f32).clamp(0.08, 12.0);
            }
        }
        if let Some((iw, ih, _)) = &self.rgba_preview {
            if *iw > 0 && *ih > 0 {
                return (*iw as f32 / *ih as f32).clamp(0.08, 12.0);
            }
        }
        if let Some((iw, ih)) = probe_media_dimensions(&self.png_bytes) {
            if iw > 0 && ih > 0 {
                return (iw as f32 / ih as f32).clamp(0.08, 12.0);
            }
        }
        (self.width_pt / self.height_pt.max(1.0)).clamp(0.08, 12.0)
    }

    pub fn resize_preserving_aspect(&mut self, target_width_pt: f32, max_width_pt: f32) {
        let aspect = self.aspect_ratio();
        let clamped_w = target_width_pt.clamp(48.0, max_width_pt.max(96.0));
        self.width_pt = clamped_w;
        self.height_pt = (clamped_w / aspect).clamp(24.0, 680.0);
    }

    pub fn scale_to_fraction(&mut self, fraction: f32, content_width_pt: f32) {
        let target = content_width_pt.max(120.0) * fraction.clamp(0.15, 1.0);
        self.resize_preserving_aspect(target, content_width_pt);
    }

    /// Detect canonical OOXML media extension (`"png"`, `"jpeg"`, `"bmp"`, `"gif"`) from payload bytes.
    pub fn media_extension(&self) -> &'static str {
        let b = &self.png_bytes;
        if b.len() >= 8 && b[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
            "png"
        } else if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF {
            "jpeg"
        } else if b.len() >= 2 && &b[0..2] == b"BM" {
            "bmp"
        } else if b.len() >= 6 && (&b[0..6] == b"GIF87a" || &b[0..6] == b"GIF89a") {
            "gif"
        } else {
            "png"
        }
    }

    /// Parse `/image`, `/img`, `/figure`, or `/photo [optional/path.png]`.
    /// Returns `Some(None)` to open the native image picker or `Some(Some(path))` to load directly.
    pub fn parse_slash_image_command(line: &str) -> Option<Option<String>> {
        let trimmed = line.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let body = &trimmed[1..];
        let (cmd, rest) = match body.find(char::is_whitespace) {
            Some(pos) => (&body[..pos], body[pos..].trim()),
            None => (body, ""),
        };
        if matches!(
            cmd.to_ascii_lowercase().as_str(),
            "image" | "img" | "figure" | "photo" | "picture"
        ) {
            if rest.is_empty() {
                Some(None)
            } else {
                let unquoted = rest.trim_matches('"').trim_matches('\'').trim();
                if unquoted.is_empty() {
                    Some(None)
                } else {
                    Some(Some(unquoted.to_string()))
                }
            }
        } else {
            None
        }
    }
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
    pub date_iso: String,
    pub text: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub words: usize,
    pub chars: usize,
    pub chars_no_spaces: usize,
    pub paragraphs: usize,
    pub tables: usize,
    pub reading_time_minutes: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub comments: usize,
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
    pub classification: SecurityClassification,
    pub track_changes: bool,
    pub page_setup: PageSetup,
    pub blocks: Vec<Block>,
    pub comments: Vec<DocComment>,
    /// Unmodeled OOXML ZIP entries preserved verbatim for lossless enterprise round-trip saving.
    pub passthrough_parts: BTreeMap<String, Vec<u8>>,
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
            classification: SecurityClassification::Unclassified,
            track_changes: false,
            page_setup: PageSetup::default(),
            blocks: vec![Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, ""))],
            comments: Vec::new(),
            passthrough_parts: BTreeMap::new(),
        }
    }

    pub fn ensure_non_empty(&mut self) {
        if self.blocks.is_empty() {
            self.blocks
                .push(Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "")));
        }
    }

    /// Total binary media + passthrough payload size in bytes (used to skip redundant
    /// synchronous LZ77 compression on 30–100+ MB screenshot-heavy `.docx` documents).
    pub fn estimated_media_bytes(&self) -> usize {
        let mut sum = 0usize;
        for b in &self.blocks {
            if let Block::Image(img) = b {
                sum = sum.saturating_add(img.png_bytes.len());
            }
        }
        for v in self.passthrough_parts.values() {
            sum = sum.saturating_add(v.len());
        }
        sum
    }

    /// Lightweight `O(1)` media-refcounted clone that omits archive `passthrough_parts`
    /// and decoded `rgba_preview` buffers for fast Undo/Redo and in-memory Time-Machine snapshots.
    pub fn clone_without_passthrough(&self) -> Self {
        let mut blocks = Vec::with_capacity(self.blocks.len());
        for b in &self.blocks {
            match b {
                Block::Image(img) => {
                    let mut slim = img.clone();
                    slim.rgba_preview = None;
                    blocks.push(Block::Image(slim));
                }
                other => blocks.push(other.clone()),
            }
        }
        Self {
            title: self.title.clone(),
            author: self.author.clone(),
            classification: self.classification,
            track_changes: self.track_changes,
            page_setup: self.page_setup.clone(),
            blocks,
            comments: self.comments.clone(),
            passthrough_parts: BTreeMap::new(),
        }
    }

    pub fn next_revision_id(&self) -> u32 {
        let mut max_id = 0u32;
        for block in &self.blocks {
            if let Block::Paragraph(p) = block {
                for s in &p.spans {
                    if let Some(r) = &s.revision {
                        max_id = max_id.max(r.id);
                    }
                }
            }
        }
        max_id.saturating_add(1)
    }

    pub fn revision_counts(&self) -> (usize, usize) {
        let mut ins = 0usize;
        let mut del = 0usize;
        for block in &self.blocks {
            if let Block::Paragraph(p) = block {
                for s in &p.spans {
                    if let Some(r) = &s.revision {
                        match r.kind {
                            RevisionKind::Insertion => ins += 1,
                            RevisionKind::Deletion => del += 1,
                        }
                    }
                }
            }
        }
        (ins, del)
    }

    /// Accepts all tracked changes: keeps inserted runs as clean text and permanently purges deleted runs.
    pub fn accept_all_revisions(&mut self) -> usize {
        let mut count = 0usize;
        for block in &mut self.blocks {
            if let Block::Paragraph(p) = block {
                let mut next_spans = Vec::with_capacity(p.spans.len());
                for mut s in p.spans.drain(..) {
                    match s.revision.take().map(|r| r.kind) {
                        Some(RevisionKind::Insertion) => {
                            count += 1;
                            next_spans.push(s);
                        }
                        Some(RevisionKind::Deletion) => {
                            count += 1;
                        }
                        None => next_spans.push(s),
                    }
                }
                p.spans = next_spans;
                p.normalize_spans();
            }
        }
        count
    }

    /// Rejects all tracked changes: discards inserted runs and restores deleted runs to clean text.
    pub fn reject_all_revisions(&mut self) -> usize {
        let mut count = 0usize;
        for block in &mut self.blocks {
            if let Block::Paragraph(p) = block {
                let mut next_spans = Vec::with_capacity(p.spans.len());
                for mut s in p.spans.drain(..) {
                    match s.revision.take().map(|r| r.kind) {
                        Some(RevisionKind::Insertion) => {
                            count += 1;
                        }
                        Some(RevisionKind::Deletion) => {
                            count += 1;
                            next_spans.push(s);
                        }
                        None => next_spans.push(s),
                    }
                }
                p.spans = next_spans;
                p.normalize_spans();
            }
        }
        count
    }

    /// Accepts a single tracked revision clause (`revision.id == target_id`), leaving other changes untouched.
    pub fn accept_revision(&mut self, target_id: u32) -> usize {
        let mut count = 0usize;
        for block in &mut self.blocks {
            if let Block::Paragraph(p) = block {
                let mut next_spans = Vec::with_capacity(p.spans.len());
                for mut s in p.spans.drain(..) {
                    match &s.revision {
                        Some(r) if r.id == target_id && r.kind == RevisionKind::Insertion => {
                            s.revision = None;
                            count += 1;
                            next_spans.push(s);
                        }
                        Some(r) if r.id == target_id && r.kind == RevisionKind::Deletion => {
                            count += 1;
                        }
                        _ => next_spans.push(s),
                    }
                }
                p.spans = next_spans;
                p.normalize_spans();
            }
        }
        count
    }

    /// Rejects a single tracked revision clause (`revision.id == target_id`), leaving other changes untouched.
    pub fn reject_revision(&mut self, target_id: u32) -> usize {
        let mut count = 0usize;
        for block in &mut self.blocks {
            if let Block::Paragraph(p) = block {
                let mut next_spans = Vec::with_capacity(p.spans.len());
                for mut s in p.spans.drain(..) {
                    match &s.revision {
                        Some(r) if r.id == target_id && r.kind == RevisionKind::Insertion => {
                            count += 1;
                        }
                        Some(r) if r.id == target_id && r.kind == RevisionKind::Deletion => {
                            s.revision = None;
                            count += 1;
                            next_spans.push(s);
                        }
                        _ => next_spans.push(s),
                    }
                }
                p.spans = next_spans;
                p.normalize_spans();
            }
        }
        count
    }

    pub fn add_comment(
        &mut self,
        block_index: usize,
        author: impl Into<String>,
        date_iso: impl Into<String>,
        text: impl Into<String>,
    ) -> u32 {
        let id = self
            .comments
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.comments.push(DocComment {
            id,
            block_index: block_index.min(self.blocks.len().saturating_sub(1)),
            author: author.into(),
            date_iso: date_iso.into(),
            text: text.into(),
            resolved: false,
        });
        id
    }

    pub fn toggle_comment_resolved(&mut self, comment_id: u32) -> bool {
        if let Some(c) = self.comments.iter_mut().find(|c| c.id == comment_id) {
            c.resolved = !c.resolved;
            true
        } else {
            false
        }
    }

    /// Generates an ECMA-376 legal redline document comparing `base` against `revised` word-by-word.
    pub fn diff_redline(
        base: &Document,
        revised: &Document,
        reviewer: &str,
        date_iso: &str,
    ) -> Document {
        let mut out = revised.clone();
        out.title = format!("{} (Redline)", revised.title);
        out.track_changes = true;
        out.blocks.clear();

        let max_len = base.blocks.len().max(revised.blocks.len());
        let mut rev_id = 1u32;

        for idx in 0..max_len {
            match (base.blocks.get(idx), revised.blocks.get(idx)) {
                (Some(Block::Paragraph(bp)), Some(Block::Paragraph(rp))) => {
                    let b_text = bp.accepted_plain_text();
                    let r_text = rp.accepted_plain_text();
                    if b_text == r_text {
                        out.blocks.push(Block::Paragraph(rp.clone()));
                    } else {
                        let base_style = rp
                            .spans
                            .first()
                            .map(|s| s.style.clone())
                            .unwrap_or_else(|| SpanStyle {
                                font_size_pt: rp.style.default_font_size(),
                                bold: rp.style.is_bold_by_default(),
                                ..SpanStyle::default()
                            });
                        let spans = diff_words_to_spans(
                            &b_text,
                            &r_text,
                            &base_style,
                            reviewer,
                            date_iso,
                            &mut rev_id,
                        );
                        let mut p = rp.clone();
                        p.spans = spans;
                        p.normalize_spans();
                        out.blocks.push(Block::Paragraph(p));
                    }
                }
                (None, Some(Block::Paragraph(rp))) => {
                    let mut p = rp.clone();
                    let r_text = rp.accepted_plain_text();
                    if !r_text.is_empty() {
                        let base_style = rp
                            .spans
                            .first()
                            .map(|s| s.style.clone())
                            .unwrap_or_default();
                        p.spans = vec![TextSpan::tracked(
                            r_text,
                            base_style,
                            RevisionMeta {
                                id: rev_id,
                                kind: RevisionKind::Insertion,
                                author: reviewer.to_string(),
                                date_iso: date_iso.to_string(),
                            },
                        )];
                        rev_id += 1;
                    }
                    out.blocks.push(Block::Paragraph(p));
                }
                (Some(Block::Paragraph(bp)), None) => {
                    let b_text = bp.accepted_plain_text();
                    if !b_text.is_empty() {
                        let mut p = bp.clone();
                        let base_style = bp
                            .spans
                            .first()
                            .map(|s| s.style.clone())
                            .unwrap_or_default();
                        p.spans = vec![TextSpan::tracked(
                            b_text,
                            base_style,
                            RevisionMeta {
                                id: rev_id,
                                kind: RevisionKind::Deletion,
                                author: reviewer.to_string(),
                                date_iso: date_iso.to_string(),
                            },
                        )];
                        rev_id += 1;
                        out.blocks.push(Block::Paragraph(p));
                    }
                }
                (_, Some(other)) => out.blocks.push(other.clone()),
                (Some(other), None) => out.blocks.push(other.clone()),
                (None, None) => {}
            }
        }

        out.ensure_non_empty();
        out
    }

    pub fn stats(&self) -> DocumentStats {
        let mut stats = DocumentStats::default();
        for block in &self.blocks {
            match block {
                Block::Paragraph(p) => {
                    stats.paragraphs += 1;
                    let text = p.accepted_plain_text();
                    stats.words += text.split_whitespace().count();
                    stats.chars += text.chars().count();
                    stats.chars_no_spaces += text.chars().filter(|c| !c.is_whitespace()).count();
                    for s in &p.spans {
                        if let Some(r) = &s.revision {
                            match r.kind {
                                RevisionKind::Insertion => stats.insertions += 1,
                                RevisionKind::Deletion => stats.deletions += 1,
                            }
                        }
                    }
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
        stats.comments = self.comments.len();
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
                    let title = p.accepted_plain_text().trim().to_string();
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
                    out.push_str(&p.accepted_plain_text());
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

fn tokenize_preserving_ws(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0usize;
    let mut in_ws: Option<bool> = None;
    for (idx, ch) in s.char_indices() {
        let ws = ch.is_whitespace();
        if let Some(prev_ws) = in_ws {
            if prev_ws != ws {
                tokens.push(&s[start..idx]);
                start = idx;
                in_ws = Some(ws);
            }
        } else {
            in_ws = Some(ws);
        }
    }
    if start < s.len() {
        tokens.push(&s[start..]);
    }
    tokens
}

fn diff_words_to_spans(
    old_text: &str,
    new_text: &str,
    style: &SpanStyle,
    reviewer: &str,
    date_iso: &str,
    rev_id: &mut u32,
) -> Vec<TextSpan> {
    let a = tokenize_preserving_ws(old_text);
    let b = tokenize_preserving_ws(new_text);

    // Guard quadratic DP on unusually huge paragraphs (> 600 tokens)
    if a.len() > 600 || b.len() > 600 {
        let mut spans = Vec::new();
        if !old_text.is_empty() {
            spans.push(TextSpan::tracked(
                old_text,
                style.clone(),
                RevisionMeta {
                    id: *rev_id,
                    kind: RevisionKind::Deletion,
                    author: reviewer.to_string(),
                    date_iso: date_iso.to_string(),
                },
            ));
            *rev_id += 1;
        }
        if !new_text.is_empty() {
            spans.push(TextSpan::tracked(
                new_text,
                style.clone(),
                RevisionMeta {
                    id: *rev_id,
                    kind: RevisionKind::Insertion,
                    author: reviewer.to_string(),
                    date_iso: date_iso.to_string(),
                },
            ));
            *rev_id += 1;
        }
        return spans;
    }

    let n = a.len();
    let m = b.len();
    let stride = m + 1;
    let mut dp = vec![0u16; (n + 1) * stride];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * stride + j] = if a[i] == b[j] {
                dp[(i + 1) * stride + (j + 1)] + 1
            } else {
                dp[(i + 1) * stride + j].max(dp[i * stride + (j + 1)])
            };
        }
    }

    let mut spans: Vec<TextSpan> = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < n && j < m {
        if a[i] == b[j] {
            spans.push(TextSpan::styled(a[i], style.clone()));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * stride + j] >= dp[i * stride + (j + 1)] {
            spans.push(TextSpan::tracked(
                a[i],
                style.clone(),
                RevisionMeta {
                    id: *rev_id,
                    kind: RevisionKind::Deletion,
                    author: reviewer.to_string(),
                    date_iso: date_iso.to_string(),
                },
            ));
            *rev_id += 1;
            i += 1;
        } else {
            spans.push(TextSpan::tracked(
                b[j],
                style.clone(),
                RevisionMeta {
                    id: *rev_id,
                    kind: RevisionKind::Insertion,
                    author: reviewer.to_string(),
                    date_iso: date_iso.to_string(),
                },
            ));
            *rev_id += 1;
            j += 1;
        }
    }
    while i < n {
        spans.push(TextSpan::tracked(
            a[i],
            style.clone(),
            RevisionMeta {
                id: *rev_id,
                kind: RevisionKind::Deletion,
                author: reviewer.to_string(),
                date_iso: date_iso.to_string(),
            },
        ));
        *rev_id += 1;
        i += 1;
    }
    while j < m {
        spans.push(TextSpan::tracked(
            b[j],
            style.clone(),
            RevisionMeta {
                id: *rev_id,
                kind: RevisionKind::Insertion,
                author: reviewer.to_string(),
                date_iso: date_iso.to_string(),
            },
        ));
        *rev_id += 1;
        j += 1;
    }
    spans
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
        let slim = current.clone_without_passthrough();
        if self.undo_stack.last() == Some(&slim) {
            return;
        }
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(slim);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut Document) -> bool {
        if let Some(mut prev) = self.undo_stack.pop() {
            let preserved_passthrough = std::mem::take(&mut current.passthrough_parts);
            self.redo_stack.push(current.clone_without_passthrough());
            prev.passthrough_parts = preserved_passthrough;
            *current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut Document) -> bool {
        if let Some(mut next) = self.redo_stack.pop() {
            let preserved_passthrough = std::mem::take(&mut current.passthrough_parts);
            self.undo_stack.push(current.clone_without_passthrough());
            next.passthrough_parts = preserved_passthrough;
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

    #[test]
    fn test_table_insert_delete_row_col_and_shading() {
        let mut tbl = TableBlock::new(2, 2);
        tbl.insert_row_at(1);
        assert_eq!(tbl.rows.len(), 3);
        tbl.insert_column_at(1);
        assert_eq!(tbl.cols, 3);
        assert_eq!(tbl.rows[0].len(), 3);

        tbl.toggle_cell_bold(1, 1);
        assert!(tbl.rows[1][1].bold);
        tbl.cycle_cell_fill(1, 1);
        assert!(tbl.rows[1][1].bg_color.is_some());

        assert!(tbl.delete_row(1));
        assert_eq!(tbl.rows.len(), 2);
        assert!(tbl.delete_column(1));
        assert_eq!(tbl.cols, 2);
    }

    #[test]
    fn test_track_changes_diff_accept_and_reject() {
        let mut base = Document::blank();
        base.blocks = vec![Block::Paragraph(Paragraph::new(
            ParagraphStyle::Normal,
            "Payment due within 30 calendar days of invoice receipt.",
        ))];
        let mut revised = Document::blank();
        revised.blocks = vec![Block::Paragraph(Paragraph::new(
            ParagraphStyle::Normal,
            "Payment due within 14 business days of verified invoice receipt.",
        ))];

        let redline = Document::diff_redline(&base, &revised, "Legal Counsel", "2026-10-27T10:00:00Z");
        let (ins, del) = redline.revision_counts();
        assert!(ins >= 1 && del >= 1);

        let mut accepted = redline.clone();
        assert!(accepted.accept_all_revisions() >= 2);
        assert_eq!(
            accepted.to_plain_text().trim(),
            "Payment due within 14 business days of verified invoice receipt."
        );

        let mut rejected = redline;
        assert!(rejected.reject_all_revisions() >= 2);
        assert_eq!(
            rejected.to_plain_text().trim(),
            "Payment due within 30 calendar days of invoice receipt."
        );
    }

    #[test]
    fn test_substring_transform_per_clause_revision_and_word_jumps() {
        let mut p = Paragraph::new(ParagraphStyle::Normal, "The quick brown fox jumps");
        p.transform_range(4, 15, |st| {
            st.bold = true;
            st.highlight = Some(RgbColor::AMBER_HIGHLIGHT);
        });
        assert_eq!(p.spans.len(), 3);
        assert!(!p.spans[0].style.bold);
        assert!(p.spans[1].style.bold);
        assert_eq!(p.spans[1].text, "quick brown");
        assert!(!p.spans[2].style.bold);

        assert_eq!(prev_word_char_idx("Hello brave world", 12), 6);
        assert_eq!(next_word_char_idx("Hello brave world", 0), 6);
        assert_eq!(word_range_at_char("Hello brave world", 8), (6, 11));

        let mut contract = TemplateKind::EnterpriseContract.build();
        let before = contract.revision_counts();
        assert!(before.0 + before.1 >= 2);
        assert!(contract.accept_revision(1) >= 1);
        let after = contract.revision_counts();
        assert!(after.0 + after.1 < before.0 + before.1);
    }

    #[test]
    fn test_smart_table_creation_presets_slash_pipe_and_tsv() {
        // 1. Slash command dimensions & presets
        let (tbl_dim, _, _) = TableBlock::from_slash_command("/table 4x5").expect("slash 4x5");
        assert_eq!(tbl_dim.rows.len(), 4);
        assert_eq!(tbl_dim.cols, 5);

        let (tbl_inv, weights_opt, _) =
            TableBlock::from_slash_command("/invoice").expect("slash invoice");
        assert_eq!(tbl_inv.cols, 4);
        assert_eq!(tbl_inv.rows[0][3].alignment, Alignment::Right);
        assert!(weights_opt.is_some());

        // 2. Single-line Markdown pipe header -> ready-to-type 2-row table
        let pipe_tbl = TableBlock::from_pipe_header_line("| Feature | Owner | ETA |")
            .expect("pipe header line");
        assert_eq!(pipe_tbl.cols, 3);
        assert_eq!(pipe_tbl.rows.len(), 2);
        assert_eq!(pipe_tbl.rows[0][0].text, "Feature");
        assert!(pipe_tbl.rows[0][0].bold);
        assert_eq!(pipe_tbl.rows[1][0].text, "");

        // 3. Multi-line Markdown pipe paste with column alignment inference
        let md = "| Item | Qty | Amount |\n| :--- | :---: | ---: |\n| License | 3 | $450 |";
        let md_tbl = TableBlock::from_delimited_text(md).expect("markdown table paste");
        assert_eq!(md_tbl.rows.len(), 2);
        assert_eq!(md_tbl.cols, 3);
        assert_eq!(md_tbl.rows[1][1].alignment, Alignment::Center);
        assert_eq!(md_tbl.rows[1][2].alignment, Alignment::Right);

        // 4. TSV paste + grid expansion inside an existing table + theme/zebra cycling
        let tsv = "Q1\t$12,000\t+14%\nQ2\t$15,400\t+28%";
        let mut tsv_tbl = TableBlock::from_delimited_text(tsv).expect("tsv paste");
        assert_eq!(tsv_tbl.rows.len(), 2);
        assert_eq!(tsv_tbl.cols, 3);

        tsv_tbl.paste_grid_at(
            1,
            2,
            &[
                vec!["A".to_string(), "B".to_string()],
                vec!["C".to_string(), "D".to_string()],
            ],
        );
        assert_eq!(tsv_tbl.cols, 4);
        assert_eq!(tsv_tbl.rows.len(), 3);
        assert_eq!(tsv_tbl.rows[2][3].text, "D");

        assert!(tsv_tbl.toggle_zebra_striping());
        assert!(tsv_tbl.is_zebra_striped());
        let next_theme = tsv_tbl.cycle_theme();
        assert_eq!(next_theme, TableTheme::CobaltLedger);
    }

    #[test]
    fn test_image_block_fitting_aspect_resize_and_slash_command() {
        let mut img = ImageBlock::from_decoded(
            "architecture-diagram",
            1600,
            800,
            vec![255; 16],
            vec![0xFF, 0xD8, 0xFF, 0xE0],
            460.0,
        );
        assert_eq!(img.media_extension(), "jpeg");
        assert!((img.width_pt - 460.0).abs() < 0.5);
        assert!((img.height_pt - 230.0).abs() < 0.5);
        assert_eq!(img.alignment, Alignment::Center);

        img.scale_to_fraction(0.5, 460.0);
        assert!((img.width_pt - 230.0).abs() < 0.5);
        assert!((img.height_pt - 115.0).abs() < 0.5);

        assert_eq!(ImageBlock::parse_slash_image_command("/image"), Some(None));
        assert_eq!(
            ImageBlock::parse_slash_image_command("/figure \"chart.png\""),
            Some(Some("chart.png".to_string()))
        );
    }
}
