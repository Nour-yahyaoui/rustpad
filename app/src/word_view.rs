//! Paginated Word (`.docx`) Studio View: Formatting Ribbon, Outline Rail,
//! Multi-Page A4/Letter/Legal Paper Canvas, Tables, Checklists, Cursor & Live Spellcheck.

use crate::commands::CommandId;
use crate::state::{HitRegion, UiAction, WordTabState};
use doc_model::{Alignment, Block, ListKind, Paragraph, ParagraphStyle};
use power_tools::SpellChecker;
use rustpad_render::{Color, DaylightTheme, FontBook, FontSlot, Rect, Surface};

pub fn paint_word_tab(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    is_focused_pane: bool,
    cursor_blink_on: bool,
    mouse_x: f32,
    mouse_y: f32,
    area: Rect,
    w: &WordTabState,
) {
    // 1. Word Formatting Toolbar Strip (42px)
    let toolbar_h = 42.0;
    let toolbar_rect = Rect::new(area.x, area.y, area.w, toolbar_h);
    surface.fill_rect(toolbar_rect, DaylightTheme::CHROME);
    surface.fill_rect(
        Rect::new(area.x, area.y + toolbar_h - 1.0, area.w, 1.0),
        DaylightTheme::BORDER,
    );

    if is_focused_pane {
        paint_word_toolbar(surface, fonts, hit_regions, mouse_x, mouse_y, toolbar_rect, w);
    } else {
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            12.5,
            toolbar_rect.x + 16.0,
            toolbar_rect.y + 26.0,
            DaylightTheme::WORD_ACCENT,
            &format!("WORD COMPANION VIEW — {}", w.doc.title),
        );
    }

    let body_rect = Rect::new(
        area.x,
        area.y + toolbar_h,
        area.w,
        (area.h - toolbar_h).max(1.0),
    );
    surface.fill_rect(body_rect, DaylightTheme::DESK_BG);

    // 2. Left Outline & Stats Sidebar when width >= 820px
    let show_sidebar = body_rect.w >= 820.0;
    let sidebar_w = if show_sidebar { 228.0 } else { 0.0 };
    if show_sidebar {
        let sb_rect = Rect::new(body_rect.x, body_rect.y, sidebar_w, body_rect.h);
        paint_word_sidebar(
            surface,
            fonts,
            hit_regions,
            tab_idx,
            mouse_x,
            mouse_y,
            sb_rect,
            w,
        );
    }

    // 3. Scrollable Paginated Paper Canvas
    let canvas_rect = Rect::new(
        body_rect.x + sidebar_w,
        body_rect.y,
        (body_rect.w - sidebar_w).max(1.0),
        body_rect.h,
    );
    surface.set_clip(canvas_rect);

    let scale = w.zoom * 1.02;
    let (pw_pt, ph_pt) = w
        .doc
        .page_setup
        .paper_size
        .dimensions_pt(w.doc.page_setup.landscape);
    let page_w = (pw_pt * scale).min((canvas_rect.w - 36.0).max(240.0));
    let eff_scale = page_w / pw_pt;
    let page_h = ph_pt * eff_scale;
    let page_gap = 24.0;

    let margin_x = w.doc.page_setup.margin_left_pt * eff_scale;
    let margin_top = w.doc.page_setup.margin_top_pt * eff_scale;
    let margin_bottom = w.doc.page_setup.margin_bottom_pt * eff_scale;
    let content_w = (page_w - margin_x * 2.0).max(120.0);

    // Pre-layout blocks into pages
    let layout_pages = layout_document_pages(fonts, w, eff_scale, content_w, page_h, margin_top, margin_bottom);

    let page_x = canvas_rect.x + ((canvas_rect.w - page_w) * 0.5).max(18.0);
    let start_y = canvas_rect.y + 22.0 - w.scroll_y;

    for (page_num, page_items) in layout_pages.iter().enumerate() {
        let py = start_y + (page_num as f32) * (page_h + page_gap);
        let sheet = Rect::new(page_x, py, page_w, page_h);

        if sheet.y + sheet.h < canvas_rect.y - 20.0 || sheet.y > canvas_rect.y + canvas_rect.h + 20.0 {
            continue;
        }

        // Crisp paper sheet + subtle elevation shadow
        surface.draw_paper_shadow(sheet, 3.0, 12.0, 26);
        surface.fill_rounded_rect(sheet, 3.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(sheet, 3.0, 1.0, DaylightTheme::BORDER);

        // Header & Footer bands
        if !w.doc.page_setup.header_text.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                10.5 * eff_scale,
                sheet.x + margin_x,
                sheet.y + (margin_top * 0.55).max(18.0),
                content_w,
                DaylightTheme::INK_MUTED,
                &w.doc.page_setup.header_text,
            );
            surface.fill_rect(
                Rect::new(
                    sheet.x + margin_x,
                    sheet.y + (margin_top * 0.68).max(24.0),
                    content_w,
                    1.0,
                ),
                DaylightTheme::BORDER,
            );
        }

        let footer_y = sheet.y + page_h - (margin_bottom * 0.45).max(16.0);
        if !w.doc.page_setup.footer_text.is_empty() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                10.0 * eff_scale,
                sheet.x + margin_x,
                footer_y,
                content_w * 0.78,
                DaylightTheme::INK_MUTED,
                &w.doc.page_setup.footer_text,
            );
        }
        if w.doc.page_setup.show_page_numbers {
            let pg_lbl = format!("Page {} of {}", page_num + 1, layout_pages.len());
            let lw = fonts.measure_text(FontSlot::SansRegular, &pg_lbl, 10.0 * eff_scale);
            surface.draw_text(
                fonts,
                FontSlot::SansRegular,
                10.0 * eff_scale,
                sheet.x + page_w - margin_x - lw,
                footer_y,
                DaylightTheme::INK_MUTED,
                &pg_lbl,
            );
        }

        // Optional watermark
        if let Some(wm) = &w.doc.page_setup.watermark {
            let wm_px = (34.0 * eff_scale).clamp(16.0, 54.0);
            let wm_w = fonts.measure_text(FontSlot::SansBold, wm, wm_px);
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                wm_px,
                sheet.x + (page_w - wm_w) * 0.5,
                sheet.y + page_h * 0.52,
                Color::rgba(148, 163, 184, 58),
                wm,
            );
        }

        // Paint each laid-out block on this page
        for item in page_items {
            paint_placed_block(
                surface,
                fonts,
                spellchecker,
                hit_regions,
                tab_idx,
                is_focused_pane,
                cursor_blink_on,
                w,
                sheet.x + margin_x,
                sheet.y + item.rel_y,
                content_w,
                eff_scale,
                item,
            );
        }
    }

    surface.reset_clip();
}

struct PlacedBlock {
    block_idx: usize,
    rel_y: f32,
    height: f32,
    wrapped_lines: Vec<(String, usize)>, // (line_text, char_offset_start)
}

fn layout_document_pages(
    fonts: &mut FontBook,
    w: &WordTabState,
    scale: f32,
    content_w: f32,
    page_h: f32,
    margin_top: f32,
    margin_bottom: f32,
) -> Vec<Vec<PlacedBlock>> {
    let mut pages: Vec<Vec<PlacedBlock>> = vec![Vec::new()];
    let mut cur_y = margin_top;
    let max_y = (page_h - margin_bottom).max(margin_top + 80.0);

    for (block_idx, block) in w.doc.blocks.iter().enumerate() {
        match block {
            Block::PageBreak => {
                pages.push(Vec::new());
                cur_y = margin_top;
            }
            Block::HorizontalRule => {
                let h = 18.0 * scale;
                if cur_y + h > max_y && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = margin_top;
                }
                pages.last_mut().unwrap().push(PlacedBlock {
                    block_idx,
                    rel_y: cur_y,
                    height: h,
                    wrapped_lines: Vec::new(),
                });
                cur_y += h;
            }
            Block::Image(img) => {
                let h = (img.height_pt * scale).clamp(48.0, 220.0) + 22.0 * scale;
                if cur_y + h > max_y && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = margin_top;
                }
                pages.last_mut().unwrap().push(PlacedBlock {
                    block_idx,
                    rel_y: cur_y,
                    height: h,
                    wrapped_lines: Vec::new(),
                });
                cur_y += h;
            }
            Block::Table(t) => {
                let row_h = 26.0 * scale;
                let h = (t.rows.len().max(1) as f32) * row_h + 14.0 * scale;
                if cur_y + h > max_y && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = margin_top;
                }
                pages.last_mut().unwrap().push(PlacedBlock {
                    block_idx,
                    rel_y: cur_y,
                    height: h,
                    wrapped_lines: Vec::new(),
                });
                cur_y += h;
            }
            Block::Paragraph(p) => {
                let base_style = p.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                let slot = FontSlot::select(base_style.font_family, base_style.bold);
                let px_size = (base_style.font_size_pt * scale * 1.18).clamp(10.0, 44.0);
                let indent = (p.indent_level as f32) * 20.0 * scale
                    + if matches!(p.list_kind, ListKind::None) {
                        0.0
                    } else {
                        22.0 * scale
                    };
                let avail_w = (content_w - indent).max(60.0);
                let lines = wrap_paragraph_lines(fonts, slot, px_size, &p.plain_text(), avail_w);
                let line_h = px_size * p.line_spacing.max(1.2);
                let space_after = match p.style {
                    ParagraphStyle::Title => 12.0 * scale,
                    ParagraphStyle::Heading1 | ParagraphStyle::Heading2 => 9.0 * scale,
                    _ => 6.0 * scale,
                };
                let h = (lines.len().max(1) as f32) * line_h + space_after;
                if cur_y + h > max_y && !pages.last().unwrap().is_empty() {
                    pages.push(Vec::new());
                    cur_y = margin_top;
                }
                pages.last_mut().unwrap().push(PlacedBlock {
                    block_idx,
                    rel_y: cur_y,
                    height: h,
                    wrapped_lines: lines,
                });
                cur_y += h;
            }
        }
    }
    pages
}

fn wrap_paragraph_lines(
    fonts: &mut FontBook,
    slot: FontSlot,
    px_size: f32,
    text: &str,
    max_w: f32,
) -> Vec<(String, usize)> {
    if text.is_empty() {
        return vec![(String::new(), 0)];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_start = 0usize;
    let mut char_cursor = 0usize;

    for word in text.split_inclusive(' ') {
        let w_chars = word.chars().count();
        let candidate = format!("{cur}{word}");
        if !cur.is_empty() && fonts.measure_text(slot, &candidate, px_size) > max_w {
            lines.push((cur.trim_end_matches(' ').to_string(), cur_start));
            cur = word.to_string();
            cur_start = char_cursor;
        } else {
            cur.push_str(word);
        }
        char_cursor += w_chars;
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push((cur, cur_start));
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn paint_placed_block(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    is_focused_pane: bool,
    cursor_blink_on: bool,
    w: &WordTabState,
    base_x: f32,
    base_y: f32,
    content_w: f32,
    scale: f32,
    item: &PlacedBlock,
) {
    let Some(block) = w.doc.blocks.get(item.block_idx) else {
        return;
    };
    match block {
        Block::HorizontalRule => {
            surface.fill_rect(
                Rect::new(base_x, base_y + 8.0 * scale, content_w, 1.5),
                DaylightTheme::BORDER_STRONG,
            );
        }
        Block::PageBreak => {}
        Block::Image(img) => {
            let box_h = (item.height - 16.0 * scale).max(36.0);
            let box_w = (img.width_pt * scale).min(content_w);
            let r = Rect::new(base_x, base_y + 4.0, box_w, box_h);
            surface.fill_rounded_rect(r, 6.0, DaylightTheme::SIDEBAR);
            surface.stroke_rounded_rect(r, 6.0, 1.0, DaylightTheme::BORDER_STRONG);
            if let Some((iw, ih, rgba)) = &img.rgba_preview {
                surface.blit_rgba(r.x as i32 + 8, r.y as i32 + 8, *iw, *ih, rgba);
            } else {
                surface.draw_logo_36(r.x as i32 + 12, r.y as i32 + 10);
                surface.draw_text_clipped(
                    fonts,
                    FontSlot::SansBold,
                    12.0 * scale,
                    r.x + 58.0,
                    r.y + 30.0,
                    r.w - 68.0,
                    DaylightTheme::INK,
                    &format!("Embedded Figure: {}", img.name),
                );
                if !img.caption.is_empty() {
                    surface.draw_text_clipped(
                        fonts,
                        FontSlot::SansRegular,
                        11.0 * scale,
                        r.x + 58.0,
                        r.y + 48.0,
                        r.w - 68.0,
                        DaylightTheme::INK_SUB,
                        &img.caption,
                    );
                }
            }
        }
        Block::Table(t) => {
            let cols = t.cols.max(1);
            let col_w = content_w / (cols as f32);
            let row_h = 26.0 * scale;
            for (r_idx, row) in t.rows.iter().enumerate() {
                for (c_idx, cell) in row.iter().enumerate() {
                    let cell_rect = Rect::new(
                        base_x + (c_idx as f32) * col_w,
                        base_y + (r_idx as f32) * row_h,
                        col_w,
                        row_h,
                    );
                    if let Some(bg) = cell.bg_color {
                        surface.fill_rect(cell_rect, Color::from_doc(bg));
                    }
                    let is_active_cell = is_focused_pane
                        && w.active_table_cell == Some((item.block_idx, r_idx, c_idx));
                    surface.stroke_rounded_rect(
                        cell_rect,
                        0.0,
                        if is_active_cell { 2.0 } else { 1.0 },
                        if is_active_cell {
                            DaylightTheme::WORD_ACCENT
                        } else {
                            Color::from_doc(t.border_color)
                        },
                    );
                    let slot = if cell.bold {
                        FontSlot::SansBold
                    } else {
                        FontSlot::SansRegular
                    };
                    let txt_px = (11.5 * scale).clamp(9.5, 18.0);
                    surface.draw_text_clipped(
                        fonts,
                        slot,
                        txt_px,
                        cell_rect.x + 7.0 * scale,
                        cell_rect.y + row_h * 0.68,
                        col_w - 14.0 * scale,
                        DaylightTheme::INK,
                        &cell.text,
                    );
                    hit_regions.push(HitRegion {
                        rect: cell_rect,
                        action: UiAction::ClickTableCell {
                            tab_idx,
                            block_idx: item.block_idx,
                            row: r_idx,
                            col: c_idx,
                        },
                    });
                }
            }
        }
        Block::Paragraph(p) => {
            paint_paragraph(
                surface,
                fonts,
                spellchecker,
                hit_regions,
                tab_idx,
                is_focused_pane,
                cursor_blink_on,
                w,
                p,
                base_x,
                base_y,
                content_w,
                scale,
                item,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_paragraph(
    surface: &mut Surface,
    fonts: &mut FontBook,
    spellchecker: &SpellChecker,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    is_focused_pane: bool,
    cursor_blink_on: bool,
    w: &WordTabState,
    p: &Paragraph,
    base_x: f32,
    base_y: f32,
    content_w: f32,
    scale: f32,
    item: &PlacedBlock,
) {
    let base_style = p.spans.first().map(|s| s.style.clone()).unwrap_or_default();
    let slot = FontSlot::select(base_style.font_family, base_style.bold);
    let px_size = (base_style.font_size_pt * scale * 1.18).clamp(10.0, 44.0);
    let line_h = px_size * p.line_spacing.max(1.2);
    let indent_px = (p.indent_level as f32) * 20.0 * scale;

    // Quote accent bar
    if p.style == ParagraphStyle::Quote {
        surface.fill_rounded_rect(
            Rect::new(base_x + indent_px, base_y + 2.0, 3.5, item.height - 6.0),
            1.5,
            DaylightTheme::WORD_ACCENT,
        );
    }

    let list_offset = match p.list_kind {
        ListKind::None => {
            if p.style == ParagraphStyle::Quote {
                14.0 * scale
            } else {
                0.0
            }
        }
        ListKind::Bullet => {
            surface.fill_rounded_rect(
                Rect::new(
                    base_x + indent_px + 5.0 * scale,
                    base_y + px_size * 0.52,
                    5.0 * scale,
                    5.0 * scale,
                ),
                2.5 * scale,
                DaylightTheme::WORD_ACCENT,
            );
            22.0 * scale
        }
        ListKind::Numbered(n) => {
            surface.draw_text(
                fonts,
                FontSlot::SansBold,
                px_size * 0.92,
                base_x + indent_px,
                base_y + px_size * 0.92,
                DaylightTheme::WORD_ACCENT,
                &format!("{n}."),
            );
            24.0 * scale
        }
        ListKind::Checklist(done) => {
            let cb = Rect::new(
                base_x + indent_px + 2.0,
                base_y + px_size * 0.18,
                14.0 * scale,
                14.0 * scale,
            );
            if done {
                surface.fill_rounded_rect(cb, 3.5, DaylightTheme::SUCCESS);
                surface.draw_line(
                    cb.x + 3.0 * scale,
                    cb.y + 7.5 * scale,
                    cb.x + 6.0 * scale,
                    cb.y + 10.5 * scale,
                    1.8,
                    DaylightTheme::PAPER,
                );
                surface.draw_line(
                    cb.x + 6.0 * scale,
                    cb.y + 10.5 * scale,
                    cb.x + 11.2 * scale,
                    cb.y + 4.0 * scale,
                    1.8,
                    DaylightTheme::PAPER,
                );
            } else {
                surface.fill_rounded_rect(cb, 3.5, DaylightTheme::PAPER);
                surface.stroke_rounded_rect(cb, 3.5, 1.4, DaylightTheme::BORDER_STRONG);
            }
            hit_regions.push(HitRegion {
                rect: cb.inset(-3.0),
                action: UiAction::ToggleChecklistItem {
                    tab_idx,
                    block_idx: item.block_idx,
                },
            });
            24.0 * scale
        }
    };

    let text_x0 = base_x + indent_px + list_offset;
    let avail_w = (content_w - indent_px - list_offset).max(60.0);
    let text_color = if matches!(p.list_kind, ListKind::Checklist(true)) {
        DaylightTheme::INK_MUTED
    } else {
        Color::from_doc(base_style.color)
    };

    for (line_idx, (line_str, line_char_start)) in item.wrapped_lines.iter().enumerate() {
        let line_w = fonts.measure_text(slot, line_str, px_size);
        let draw_x = match p.alignment {
            Alignment::Left | Alignment::Justify => text_x0,
            Alignment::Center => text_x0 + ((avail_w - line_w) * 0.5).max(0.0),
            Alignment::Right => text_x0 + (avail_w - line_w).max(0.0),
        };
        let baseline_y = base_y + (line_idx as f32) * line_h + px_size * 0.92;

        if let Some(hl) = base_style.highlight {
            if !line_str.is_empty() {
                surface.fill_rounded_rect(
                    Rect::new(draw_x - 2.0, baseline_y - px_size * 0.82, line_w + 4.0, px_size * 1.05),
                    2.5,
                    Color::from_doc(hl),
                );
            }
        }

        surface.draw_text(fonts, slot, px_size, draw_x, baseline_y, text_color, line_str);

        if base_style.underline && line_w > 0.0 {
            surface.fill_rect(
                Rect::new(draw_x, baseline_y + 2.0, line_w, 1.2),
                text_color,
            );
        }
        if (base_style.strikethrough || matches!(p.list_kind, ListKind::Checklist(true)))
            && line_w > 0.0
        {
            surface.fill_rect(
                Rect::new(draw_x, baseline_y - px_size * 0.3, line_w, 1.2),
                text_color,
            );
        }

        // Spellcheck wavy red underline
        if w.spellcheck && !line_str.is_empty() {
            let mut scan_x = draw_x;
            for piece in line_str.split_inclusive(' ') {
                let pw = fonts.measure_text(slot, piece, px_size);
                let trimmed = piece.trim();
                if !trimmed.is_empty() && !spellchecker.is_valid(trimmed) {
                    let tw = fonts.measure_text(slot, trimmed, px_size);
                    surface.fill_rect(
                        Rect::new(scan_x, baseline_y + 2.5, tw, 1.6),
                        DaylightTheme::PDF_ACCENT,
                    );
                }
                scan_x += pw;
            }
        }

        // Active blinking caret
        let line_chars = line_str.chars().count();
        let is_last_line = line_idx + 1 == item.wrapped_lines.len();
        let in_this_line = w.cursor_char >= *line_char_start
            && (w.cursor_char < *line_char_start + line_chars
                || (is_last_line && w.cursor_char <= *line_char_start + line_chars + 1));

        if is_focused_pane
            && w.active_table_cell.is_none()
            && w.cursor_block == item.block_idx
            && in_this_line
            && cursor_blink_on
        {
            let rel_chars = w.cursor_char.saturating_sub(*line_char_start).min(line_chars);
            let prefix: String = line_str.chars().take(rel_chars).collect();
            let cx = draw_x + fonts.measure_text(slot, &prefix, px_size);
            surface.fill_rect(
                Rect::new(cx, baseline_y - px_size * 0.84, 2.0, px_size * 1.06),
                DaylightTheme::WORD_ACCENT,
            );
        }

        // Clickable line hit region
        let line_rect = Rect::new(
            base_x,
            base_y + (line_idx as f32) * line_h,
            content_w,
            line_h,
        );
        let click_char = *line_char_start + line_chars;
        hit_regions.push(HitRegion {
            rect: line_rect,
            action: UiAction::ClickWordParagraph {
                tab_idx,
                block_idx: item.block_idx,
                char_idx: click_char,
            },
        });
    }
}

fn paint_word_toolbar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    mouse_x: f32,
    mouse_y: f32,
    bar: Rect,
    w: &WordTabState,
) {
    let cur_para = w.doc.blocks.get(w.cursor_block).and_then(|b| match b {
        Block::Paragraph(p) => Some(p),
        _ => None,
    });
    let bold_on = cur_para
        .and_then(|p| p.spans.first())
        .map(|s| s.style.bold)
        .unwrap_or(false);
    let italic_on = cur_para
        .and_then(|p| p.spans.first())
        .map(|s| s.style.italic)
        .unwrap_or(false);
    let under_on = cur_para
        .and_then(|p| p.spans.first())
        .map(|s| s.style.underline)
        .unwrap_or(false);
    let style_lbl = cur_para.map(|p| p.style.label()).unwrap_or("Table");
    let font_lbl = cur_para
        .and_then(|p| p.spans.first())
        .map(|s| match s.style.font_family {
            doc_model::FontFamily::Sans => "Sans",
            doc_model::FontFamily::Serif => "Serif",
            doc_model::FontFamily::Mono => "Mono",
        })
        .unwrap_or("Sans");

    let buttons: &[(&str, CommandId, bool)] = &[
        (style_lbl, CommandId::CycleStyle, false),
        (font_lbl, CommandId::CycleFontFamily, false),
        ("A-", CommandId::FontSizeDown, false),
        ("A+", CommandId::FontSizeUp, false),
        ("B", CommandId::ToggleBold, bold_on),
        ("I", CommandId::ToggleItalic, italic_on),
        ("U", CommandId::ToggleUnderline, under_on),
        ("Ink", CommandId::CycleTextColor, false),
        ("Mark", CommandId::ToggleHighlight, false),
        ("Left", CommandId::AlignLeft, false),
        ("Center", CommandId::AlignCenter, false),
        ("• List", CommandId::ToggleBulletList, false),
        ("☑ Task", CommandId::ToggleChecklist, false),
        ("+ Table", CommandId::InsertTable, false),
        ("Break", CommandId::InsertPageBreak, false),
        ("Paper", CommandId::CyclePaperSize, false),
        ("Spell", CommandId::ToggleSpellcheck, w.spellcheck),
        ("-> PDF", CommandId::ExportWordToPdf, false),
        ("History", CommandId::OpenSnapshotsDrawer, false),
    ];

    let mut x = bar.x + 12.0;
    let btn_y = bar.y + 7.0;
    let btn_h = 28.0;

    for (label, cmd, active) in buttons {
        let tw = fonts.measure_text(FontSlot::SansBold, label, 11.5);
        let bw = (tw + 16.0).max(30.0);
        if x + bw > bar.x + bar.w - 10.0 {
            break;
        }
        let r = Rect::new(x, btn_y, bw, btn_h);
        let hovered = r.contains(mouse_x, mouse_y);
        let is_pdf_cta = *cmd == CommandId::ExportWordToPdf;
        let bg = if is_pdf_cta {
            DaylightTheme::PDF_SOFT
        } else if *active {
            DaylightTheme::WORD_SOFT
        } else if hovered {
            DaylightTheme::HOVER
        } else {
            DaylightTheme::PAPER
        };
        let fg = if is_pdf_cta {
            DaylightTheme::PDF_ACCENT
        } else if *active {
            DaylightTheme::WORD_ACCENT
        } else {
            DaylightTheme::INK
        };
        surface.fill_rounded_rect(r, 6.0, bg);
        surface.stroke_rounded_rect(
            r,
            6.0,
            1.0,
            if *active {
                DaylightTheme::WORD_ACCENT
            } else {
                DaylightTheme::BORDER
            },
        );
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.5,
            r.x + (bw - tw) * 0.5,
            r.y + 18.5,
            fg,
            label,
        );
        hit_regions.push(HitRegion {
            rect: r,
            action: UiAction::ExecCommand(*cmd),
        });
        x += bw + 5.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_word_sidebar(
    surface: &mut Surface,
    fonts: &mut FontBook,
    hit_regions: &mut Vec<HitRegion>,
    tab_idx: usize,
    mouse_x: f32,
    mouse_y: f32,
    sb: Rect,
    w: &WordTabState,
) {
    surface.fill_rect(sb, DaylightTheme::SIDEBAR);
    surface.fill_rect(
        Rect::new(sb.x + sb.w - 1.0, sb.y, 1.0, sb.h),
        DaylightTheme::BORDER,
    );

    surface.draw_text(
        fonts,
        FontSlot::SansBold,
        11.0,
        sb.x + 16.0,
        sb.y + 24.0,
        DaylightTheme::INK_MUTED,
        "DOCUMENT OUTLINE",
    );

    let outline = w.doc.outline();
    let mut y = sb.y + 38.0;
    if outline.is_empty() {
        surface.draw_text(
            fonts,
            FontSlot::SansRegular,
            12.0,
            sb.x + 16.0,
            y + 14.0,
            DaylightTheme::INK_MUTED,
            "Add H1 / H2 headings to populate",
        );
        y += 32.0;
    } else {
        for item in outline.iter().take(12) {
            if y + 26.0 > sb.y + sb.h - 150.0 {
                break;
            }
            let indent = (item.level.saturating_sub(1) as f32) * 10.0;
            let row = Rect::new(sb.x + 8.0, y, sb.w - 18.0, 24.0);
            let is_cur = w.cursor_block == item.block_index;
            if is_cur || row.contains(mouse_x, mouse_y) {
                surface.fill_rounded_rect(
                    row,
                    5.0,
                    if is_cur {
                        DaylightTheme::WORD_SOFT
                    } else {
                        DaylightTheme::HOVER
                    },
                );
            }
            surface.draw_text_clipped(
                fonts,
                if item.level <= 1 {
                    FontSlot::SansBold
                } else {
                    FontSlot::SansRegular
                },
                12.0,
                row.x + 8.0 + indent,
                row.y + 16.5,
                row.w - 14.0 - indent,
                if is_cur {
                    DaylightTheme::WORD_ACCENT
                } else {
                    DaylightTheme::INK_SUB
                },
                &item.title,
            );
            hit_regions.push(HitRegion {
                rect: row,
                action: UiAction::JumpOutline {
                    tab_idx,
                    block_idx: item.block_index,
                    target_scroll_y: (item.block_index as f32) * 32.0,
                },
            });
            y += 26.0;
        }
    }

    // Bottom Live Stats Card
    let stats = w.doc.stats();
    let card_h = 128.0;
    let card = Rect::new(
        sb.x + 12.0,
        (sb.y + sb.h - card_h - 14.0).max(y + 12.0),
        sb.w - 24.0,
        card_h,
    );
    if card.y + card.h <= sb.y + sb.h - 4.0 {
        surface.fill_rounded_rect(card, 8.0, DaylightTheme::PAPER);
        surface.stroke_rounded_rect(card, 8.0, 1.0, DaylightTheme::BORDER);
        surface.draw_text(
            fonts,
            FontSlot::SansBold,
            11.0,
            card.x + 12.0,
            card.y + 20.0,
            DaylightTheme::WORD_ACCENT,
            "LIVE METRICS",
        );
        let lines = [
            format!("Words: {}", stats.words),
            format!("Characters: {}", stats.chars),
            format!("Reading time: ~{} min", stats.reading_time_minutes.max(1)),
            format!("Paper: {}", w.doc.page_setup.paper_size.label()),
            format!("Snapshots: {}", w.snapshots.list().len()),
        ];
        for (i, line) in lines.iter().enumerate() {
            surface.draw_text_clipped(
                fonts,
                FontSlot::SansRegular,
                11.5,
                card.x + 12.0,
                card.y + 39.0 + (i as f32) * 17.5,
                card.w - 20.0,
                DaylightTheme::INK_SUB,
                line,
            );
        }
    }
}
